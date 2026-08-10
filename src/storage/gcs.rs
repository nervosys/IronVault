//! Google Cloud Storage backend

use async_trait::async_trait;
use cloud_storage::{Client, Object};

use crate::error::{Result, VaultError};
use crate::storage::StorageBackend;

/// Google Cloud Storage backend
pub struct GcsBackend {
    client: Client,
    bucket: String,
    prefix: String,
}

impl GcsBackend {
    /// Create new GCS storage backend
    ///
    /// # Arguments
    /// * `bucket` - GCS bucket name
    /// * `project` - GCP project ID
    /// * `prefix` - Optional object prefix (folder path)
    ///
    /// # Authentication
    /// Uses GOOGLE_APPLICATION_CREDENTIALS environment variable
    /// pointing to service account JSON key file
    pub async fn new(bucket: String, _project: String, prefix: Option<String>) -> Result<Self> {
        let client = Client::default();

        Ok(Self {
            client,
            bucket,
            prefix: prefix.unwrap_or_default(),
        })
    }

    fn get_object_name(&self, key: &str) -> String {
        if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}/{}", self.prefix.trim_end_matches('/'), key)
        }
    }
}

#[async_trait]
impl StorageBackend for GcsBackend {
    async fn upload(&self, key: &str, data: &[u8]) -> Result<()> {
        let object_name = self.get_object_name(key);

        Object::create(
            &self.bucket,
            data.to_vec(),
            &object_name,
            "application/octet-stream",
        )
        .await
        .map_err(|e| VaultError::StorageError(format!("GCS upload failed: {}", e)))?;

        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let object_name = self.get_object_name(key);

        let data = Object::download(&self.bucket, &object_name)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("Not Found") {
                    VaultError::ModelNotFound(key.to_string())
                } else {
                    VaultError::StorageError(format!("GCS download failed: {}", e))
                }
            })?;

        Ok(data)
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let object_name = self.get_object_name(key);

        // Check if exists first
        let exists = self.exists(key).await?;
        if !exists {
            return Ok(false);
        }

        Object::delete(&self.bucket, &object_name)
            .await
            .map_err(|e| VaultError::StorageError(format!("GCS delete failed: {}", e)))?;

        Ok(true)
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let object_name = self.get_object_name(key);

        match Object::read(&self.bucket, &object_name).await {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("404") || e.to_string().contains("Not Found") {
                    Ok(false)
                } else {
                    Err(VaultError::StorageError(format!("GCS read failed: {}", e)))
                }
            }
        }
    }

    async fn list(&self) -> Result<Vec<String>> {
        let mut keys = Vec::new();

        let prefix = if !self.prefix.is_empty() {
            Some(self.prefix.clone())
        } else {
            None
        };

        let objects = Object::list_prefix(&self.bucket, prefix)
            .await
            .map_err(|e| VaultError::StorageError(format!("GCS list failed: {}", e)))?;

        for object in objects {
            let name = object.name;
            // Strip prefix if present
            let clean_name = if !self.prefix.is_empty() {
                name.strip_prefix(&format!("{}/", self.prefix.trim_end_matches('/')))
                    .unwrap_or(&name)
                    .to_string()
            } else {
                name
            };
            keys.push(clean_name);
        }

        Ok(keys)
    }

    async fn size(&self, key: &str) -> Result<u64> {
        let object_name = self.get_object_name(key);

        let object = Object::read(&self.bucket, &object_name)
            .await
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("Not Found") {
                    VaultError::ModelNotFound(key.to_string())
                } else {
                    VaultError::StorageError(format!("GCS read failed: {}", e))
                }
            })?;

        Ok(object.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require GCS credentials and a test bucket
    // They are disabled by default. Enable with: cargo test --features gcs-integration-tests

    #[tokio::test]
    #[ignore]
    async fn test_gcs_backend() {
        let bucket = std::env::var("TEST_GCS_BUCKET").unwrap();
        let project = std::env::var("TEST_GCS_PROJECT").unwrap();

        let backend = GcsBackend::new(bucket, project, Some("test-ironvault".to_string()))
            .await
            .unwrap();

        let data = b"test data";
        backend.upload("test.txt", data).await.unwrap();

        assert!(backend.exists("test.txt").await.unwrap());

        let retrieved = backend.download("test.txt").await.unwrap();
        assert_eq!(data, &retrieved[..]);

        let size = backend.size("test.txt").await.unwrap();
        assert_eq!(size, data.len() as u64);

        let deleted = backend.delete("test.txt").await.unwrap();
        assert!(deleted);
    }
}
