//! Azure Blob Storage backend
//!
//! Built on the Azure SDK for Rust v1 (`azure_storage_blob`). That SDK
//! authenticates with Entra ID tokens or a pre-signed (SAS) URL only — it has
//! no shared-key support, so `AZURE_STORAGE_KEY` is no longer accepted. See
//! [`AzureBackend::new`] for the supported credential sources.

use std::sync::Arc;

use async_trait::async_trait;
use azure_core::credentials::TokenCredential;
use azure_core::http::RequestContent;
use azure_storage_blob::models::{
    BlobClientDeleteOptions, BlobClientGetPropertiesResultHeaders,
    BlobContainerClientListBlobsOptions,
};
use azure_storage_blob::{BlobClient, BlobContainerClient};
use futures_util::TryStreamExt;
use url::Url;

use crate::error::{Result, VaultError};
use crate::storage::StorageBackend;

/// Azure Blob Storage backend
pub struct AzureBackend {
    container: BlobContainerClient,
    container_url: Url,
    credential: Option<Arc<dyn TokenCredential>>,
    prefix: String,
}

fn config_err(msg: impl Into<String>) -> VaultError {
    VaultError::ConfigError(msg.into())
}

fn storage_err(op: &str, e: &impl std::fmt::Display) -> VaultError {
    VaultError::StorageError(format!("Azure {op} failed: {e}"))
}

/// Build an Entra ID credential.
///
/// A service principal configured entirely through the environment is used when
/// present — the shape CI and containers use — otherwise the developer-tools
/// chain (`az login` / `azd auth login`).
fn entra_credential() -> Result<Arc<dyn TokenCredential>> {
    let tenant = std::env::var("AZURE_TENANT_ID").ok();
    let client = std::env::var("AZURE_CLIENT_ID").ok();
    let secret = std::env::var("AZURE_CLIENT_SECRET").ok();

    if let (Some(tenant), Some(client), Some(secret)) = (tenant, client, secret) {
        return azure_identity::ClientSecretCredential::new(&tenant, client, secret.into(), None)
            .map(|c| c as Arc<dyn TokenCredential>)
            .map_err(|e| config_err(format!("Invalid Entra ID service principal: {e}")));
    }

    azure_identity::DeveloperToolsCredential::new(None)
        .map(|c| c as Arc<dyn TokenCredential>)
        .map_err(|e| {
            config_err(format!(
                "No Azure credentials found. Set AZURE_STORAGE_SAS_TOKEN, or configure Entra ID \
                 (AZURE_TENANT_ID / AZURE_CLIENT_ID / AZURE_CLIENT_SECRET), or sign in with \
                 `az login`: {e}"
            ))
        })
}

/// Azure reports a missing blob as 404 / `BlobNotFound`.
fn is_not_found(e: &azure_core::Error) -> bool {
    if let Some(status) = e.http_status() {
        return status == azure_core::http::StatusCode::NotFound;
    }
    e.to_string().contains("BlobNotFound")
}

impl AzureBackend {
    /// Create a new Azure Blob Storage backend.
    ///
    /// # Arguments
    /// * `account` - Storage account name (or a full `https://…` endpoint)
    /// * `container` - Container name
    /// * `prefix` - Optional blob prefix (folder path)
    ///
    /// # Authentication
    ///
    /// Resolved in this order:
    ///
    /// 1. `AZURE_STORAGE_SAS_TOKEN` — a pre-signed SAS appended to the container
    ///    URL. No additional credential is used.
    /// 2. Entra ID via the standard credential chain (`AZURE_CLIENT_ID` /
    ///    `AZURE_TENANT_ID` / `AZURE_CLIENT_SECRET`, managed identity, or a
    ///    developer sign-in).
    ///
    /// `AZURE_STORAGE_KEY` is **not** supported: the Azure SDK for Rust v1 has
    /// no shared-key credential. Mint a SAS from the key, or use Entra ID.
    pub async fn new(account: String, container: String, prefix: Option<String>) -> Result<Self> {
        if std::env::var("AZURE_STORAGE_KEY").is_ok()
            && std::env::var("AZURE_STORAGE_SAS_TOKEN").is_err()
        {
            return Err(config_err(
                "AZURE_STORAGE_KEY (shared key) is no longer supported — the Azure SDK for \
                 Rust v1 provides no shared-key credential. Either set \
                 AZURE_STORAGE_SAS_TOKEN to a SAS generated from that key \
                 (`az storage container generate-sas`), or authenticate with Entra ID by \
                 setting AZURE_CLIENT_ID / AZURE_TENANT_ID / AZURE_CLIENT_SECRET.",
            ));
        }

        // Accept a bare account name or a fully-qualified endpoint.
        let host = if account.contains("://") {
            account.trim_end_matches('/').to_string()
        } else {
            format!("https://{account}.blob.core.windows.net")
        };

        let mut container_url = Url::parse(&format!("{host}/{container}"))
            .map_err(|e| config_err(format!("Invalid Azure container URL: {e}")))?;

        // A SAS authenticates via the URL itself, so no TokenCredential is used.
        let credential: Option<Arc<dyn TokenCredential>> =
            if let Ok(sas) = std::env::var("AZURE_STORAGE_SAS_TOKEN") {
                container_url.set_query(Some(sas.trim_start_matches('?')));
                None
            } else {
                Some(entra_credential()?)
            };

        let container_client =
            BlobContainerClient::new(container_url.clone(), credential.clone(), None)
                .map_err(|e| config_err(format!("Failed to create Azure client: {e}")))?;

        Ok(Self {
            container: container_client,
            container_url,
            credential,
            prefix: prefix.unwrap_or_default(),
        })
    }

    fn get_blob_name(&self, key: &str) -> String {
        if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}/{}", self.prefix.trim_end_matches('/'), key)
        }
    }

    /// Build a client for one blob, preserving any SAS query on the base URL.
    fn blob_client(&self, key: &str) -> Result<BlobClient> {
        let mut url = self.container_url.clone();
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|()| config_err("Azure container URL cannot be a base"))?;
            for part in self.get_blob_name(key).split('/') {
                segments.push(part);
            }
        }
        BlobClient::new(url, self.credential.clone(), None)
            .map_err(|e| storage_err("client construction", &e))
    }
}

#[async_trait]
impl StorageBackend for AzureBackend {
    async fn upload(&self, key: &str, data: &[u8]) -> Result<()> {
        let client = self.blob_client(key)?;
        // The SDK's request body must be owned, so the borrowed slice is copied.
        client
            .upload(RequestContent::from(data.to_vec()), None)
            .await
            .map_err(|e| storage_err("upload", &e))?;
        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let client = self.blob_client(key)?;
        let response = client.download(None).await.map_err(|e| {
            if is_not_found(&e) {
                VaultError::ModelNotFound(key.to_string())
            } else {
                storage_err("download", &e)
            }
        })?;

        let body = response
            .body
            .collect()
            .await
            .map_err(|e| storage_err("download body read", &e))?;
        Ok(body.to_vec())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let client = self.blob_client(key)?;
        match client
            .delete(Some(BlobClientDeleteOptions::default()))
            .await
        {
            Ok(_) => Ok(true),
            Err(e) if is_not_found(&e) => Ok(false),
            Err(e) => Err(storage_err("delete", &e)),
        }
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let client = self.blob_client(key)?;
        client
            .exists()
            .await
            .map_err(|e| storage_err("existence check", &e))
    }

    async fn list(&self) -> Result<Vec<String>> {
        let mut options = BlobContainerClientListBlobsOptions::default();
        if !self.prefix.is_empty() {
            options.prefix = Some(self.prefix.clone());
        }

        // The pager yields individual blobs, transparently following pages.
        let mut blobs = self
            .container
            .list_blobs(Some(options))
            .map_err(|e| storage_err("list", &e))?;

        let mut keys = Vec::new();
        while let Some(blob) = blobs
            .try_next()
            .await
            .map_err(|e| storage_err("list", &e))?
        {
            let Some(name) = blob.name else { continue };
            // Report keys relative to the configured prefix.
            let clean = if self.prefix.is_empty() {
                name
            } else {
                name.strip_prefix(&format!("{}/", self.prefix.trim_end_matches('/')))
                    .unwrap_or(&name)
                    .to_string()
            };
            keys.push(clean);
        }

        Ok(keys)
    }

    async fn size(&self, key: &str) -> Result<u64> {
        let client = self.blob_client(key)?;
        let response = client.get_properties(None).await.map_err(|e| {
            if is_not_found(&e) {
                VaultError::ModelNotFound(key.to_string())
            } else {
                storage_err("properties", &e)
            }
        })?;

        response
            .content_length()
            .map_err(|e| storage_err("properties", &e))?
            .ok_or_else(|| storage_err("properties", &"blob reported no Content-Length"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// These cases all mutate process-global environment variables, so they run
    /// as one test rather than racing each other under the parallel harness.
    #[tokio::test]
    async fn test_credential_resolution() {
        // 1. Shared-key auth disappeared with the SDK v1 migration. Users must be
        //    told what to do instead, not left with an opaque auth failure.
        std::env::set_var("AZURE_STORAGE_KEY", "dGVzdGtleQ==");
        std::env::remove_var("AZURE_STORAGE_SAS_TOKEN");

        let msg = match AzureBackend::new("acct".into(), "container".into(), None).await {
            Err(e) => e.to_string(),
            Ok(_) => panic!("shared key must be rejected"),
        };
        assert!(msg.contains("AZURE_STORAGE_SAS_TOKEN"), "got: {msg}");
        assert!(msg.contains("AZURE_CLIENT_ID"), "got: {msg}");

        // 2. A SAS authenticates through the URL — no Entra ID setup required,
        //    and no TokenCredential is constructed.
        std::env::set_var("AZURE_STORAGE_SAS_TOKEN", "sv=2022-11-02&sig=deadbeef");
        let backend = AzureBackend::new("acct".into(), "container".into(), Some("p".into()))
            .await
            .expect("a SAS should authenticate without any Entra ID setup");
        assert!(backend.credential.is_none());
        assert_eq!(
            backend.container_url.query(),
            Some("sv=2022-11-02&sig=deadbeef")
        );
        assert_eq!(backend.get_blob_name("m.bin"), "p/m.bin");

        // 3. The SAS and the prefix must both survive onto per-blob URLs,
        //    otherwise every request would be unauthenticated or misrouted.
        std::env::set_var("AZURE_STORAGE_SAS_TOKEN", "sig=abc");
        let backend = AzureBackend::new("acct".into(), "cont".into(), Some("models".into()))
            .await
            .unwrap();
        let client = backend.blob_client("a/b.bin").unwrap();
        let url = client.url();
        assert_eq!(url.path(), "/cont/models/a/b.bin");
        assert_eq!(
            url.query(),
            Some("sig=abc"),
            "SAS must survive on blob URLs"
        );

        std::env::remove_var("AZURE_STORAGE_KEY");
        std::env::remove_var("AZURE_STORAGE_SAS_TOKEN");
    }

    #[tokio::test]
    #[ignore = "requires live Azure credentials and a test container"]
    async fn test_azure_backend() {
        let account = std::env::var("TEST_AZURE_ACCOUNT").unwrap();
        let container = std::env::var("TEST_AZURE_CONTAINER").unwrap();

        let backend = AzureBackend::new(account, container, Some("test-ironvault".to_string()))
            .await
            .unwrap();

        let data = b"test data";
        backend.upload("test.txt", data).await.unwrap();
        assert!(backend.exists("test.txt").await.unwrap());
        let retrieved = backend.download("test.txt").await.unwrap();
        assert_eq!(data, &retrieved[..]);
        assert_eq!(backend.size("test.txt").await.unwrap(), data.len() as u64);
        assert!(backend.delete("test.txt").await.unwrap());
    }
}
