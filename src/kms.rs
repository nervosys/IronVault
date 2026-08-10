//! Secrets manager integration — derive vault passphrases from external KMS.
//!
//! Provides a uniform interface for retrieving secrets from:
//! - Environment variables (default / CI)
//! - Local files (permission-checked)
//! - AWS Secrets Manager (requires the `s3` feature)
//! - Azure Key Vault
//! - HashiCorp Vault
//!
//! Secrets are addressed by URI so they can be passed anywhere a passphrase is
//! accepted — see [`KmsUri`] for the scheme table.
//!
//! ```no_run
//! use ironvault::kms::{self, KmsUri};
//!
//! let uri: KmsUri = "vault://secret/iv/passphrase".parse()?;
//! let secret = kms::fetch(&uri)?;
//! # Ok::<(), ironvault::VaultError>(())
//! ```
//!
//! Every backend returns a [`Zeroizing<String>`] so the plaintext is wiped from
//! memory when dropped.

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::error::{Result, VaultError};

// ── Types ────────────────────────────────────────────────────────────────────

/// Supported KMS backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KmsBackend {
    /// Read from environment variable.
    Env,
    /// Read from a local file (must not be group/world readable on Unix).
    File,
    /// AWS Secrets Manager.
    AwsSecretsManager,
    /// Azure Key Vault.
    AzureKeyVault,
    /// HashiCorp Vault.
    HashicorpVault,
}

impl KmsBackend {
    /// URI scheme for this backend (the part before `://`).
    #[must_use]
    pub fn scheme(self) -> &'static str {
        match self {
            KmsBackend::Env => "env",
            KmsBackend::File => "file",
            KmsBackend::AwsSecretsManager => "aws-sm",
            KmsBackend::AzureKeyVault => "azure-kv",
            KmsBackend::HashicorpVault => "vault",
        }
    }
}

impl std::fmt::Display for KmsBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            KmsBackend::Env => "env",
            KmsBackend::File => "file",
            KmsBackend::AwsSecretsManager => "aws-secrets-manager",
            KmsBackend::AzureKeyVault => "azure-key-vault",
            KmsBackend::HashicorpVault => "hashicorp-vault",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for KmsBackend {
    type Err = VaultError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().replace('_', "-").as_str() {
            "env" | "environment" => Ok(KmsBackend::Env),
            "file" | "path" => Ok(KmsBackend::File),
            "aws" | "aws-sm" | "aws-secrets-manager" => Ok(KmsBackend::AwsSecretsManager),
            "azure" | "azure-kv" | "azure-key-vault" => Ok(KmsBackend::AzureKeyVault),
            "hashicorp" | "hashicorp-vault" | "vault" | "hcv" => Ok(KmsBackend::HashicorpVault),
            _ => Err(VaultError::InvalidInput(format!(
                "Unknown KMS backend: {s}"
            ))),
        }
    }
}

/// A parsed secret reference.
///
/// | URI                                | Backend              | Resolution                                                   |
/// | ---------------------------------- | -------------------- | ------------------------------------------------------------ |
/// | `env://NAME`                       | Environment variable | Value of `$NAME`                                              |
/// | `file:///abs/path`                 | Local file           | File contents, trailing newline trimmed                       |
/// | `aws-sm://secret-name`             | AWS Secrets Manager  | `GetSecretValue`; region from `AWS_REGION`                    |
/// | `azure-kv://vault-name/secret`     | Azure Key Vault      | `GET https://{vault}.vault.azure.net/secrets/{secret}`        |
/// | `vault://mount/path/key`           | HashiCorp Vault      | KV v2 then v1 under `$VAULT_ADDR`, field `key` of the secret  |
///
/// Anything without a recognised `scheme://` prefix is not a KMS URI — callers
/// should treat such values as literal secrets. Use [`is_kms_uri`] to check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmsUri {
    /// Which backend resolves this reference.
    pub backend: KmsBackend,
    /// Secret name / path, backend-specific.
    pub secret_id: String,
    /// Vault name (Azure), region (AWS), or address override (HashiCorp).
    pub endpoint: Option<String>,
}

impl std::fmt::Display for KmsUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.backend, &self.endpoint) {
            (KmsBackend::AzureKeyVault, Some(v)) => {
                write!(f, "azure-kv://{}/{}", v, self.secret_id)
            }
            _ => write!(f, "{}://{}", self.backend.scheme(), self.secret_id),
        }
    }
}

impl std::str::FromStr for KmsUri {
    type Err = VaultError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let (scheme, rest) = s.split_once("://").ok_or_else(|| {
            VaultError::InvalidInput(format!(
                "Not a KMS URI (expected `scheme://...`): {s}. \
                 Known schemes: env, file, aws-sm, azure-kv, vault"
            ))
        })?;

        let backend: KmsBackend = scheme.parse()?;
        let rest = rest.trim();

        if rest.is_empty() {
            return Err(VaultError::InvalidInput(format!(
                "KMS URI `{s}` has an empty secret reference"
            )));
        }

        match backend {
            KmsBackend::AzureKeyVault => {
                // azure-kv://<vault-name>/<secret-name>
                let (vault, secret) = rest.split_once('/').ok_or_else(|| {
                    VaultError::InvalidInput(format!(
                        "Azure Key Vault URI must be `azure-kv://<vault>/<secret>`, got: {s}"
                    ))
                })?;
                if vault.is_empty() || secret.is_empty() {
                    return Err(VaultError::InvalidInput(format!(
                        "Azure Key Vault URI must be `azure-kv://<vault>/<secret>`, got: {s}"
                    )));
                }
                Ok(KmsUri {
                    backend,
                    secret_id: secret.to_string(),
                    endpoint: Some(vault.to_string()),
                })
            }
            KmsBackend::HashicorpVault => {
                // vault://<mount>/<path...>/<key> — needs at least mount + key.
                if !rest.contains('/') {
                    return Err(VaultError::InvalidInput(format!(
                        "HashiCorp Vault URI must be `vault://<mount>/<path>/<key>`, got: {s}"
                    )));
                }
                Ok(KmsUri {
                    backend,
                    secret_id: rest.to_string(),
                    endpoint: None,
                })
            }
            KmsBackend::File => Ok(KmsUri {
                backend,
                // `file:///c:/x` and `file://./rel` both land here; strip the
                // leading slash only when it precedes a Windows drive letter.
                secret_id: normalize_file_path(rest),
                endpoint: None,
            }),
            KmsBackend::Env | KmsBackend::AwsSecretsManager => Ok(KmsUri {
                backend,
                secret_id: rest.to_string(),
                endpoint: None,
            }),
        }
    }
}

/// Strip the `file://` authority-slash so both POSIX and Windows paths work.
fn normalize_file_path(rest: &str) -> String {
    let bytes = rest.as_bytes();
    // `/c:/Users/...` → `c:/Users/...`
    if bytes.len() >= 3 && bytes[0] == b'/' && bytes[2] == b':' && bytes[1].is_ascii_alphabetic() {
        return rest[1..].to_string();
    }
    rest.to_string()
}

/// True when `s` looks like a KMS URI with a scheme this crate understands.
///
/// Used to distinguish `IRONVAULT_PASSPHRASE=hunter2` (a literal secret)
/// from `IRONVAULT_PASSPHRASE=vault://secret/iv/pass` (a reference).
#[must_use]
pub fn is_kms_uri(s: &str) -> bool {
    s.split_once("://")
        .is_some_and(|(scheme, rest)| scheme.parse::<KmsBackend>().is_ok() && !rest.is_empty())
}

/// Legacy request struct — prefer [`KmsUri`].
#[derive(Debug, Clone)]
pub struct KmsRequest {
    /// Backend to query.
    pub backend: KmsBackend,
    /// Secret name / ARN / Key Vault secret / Vault path.
    pub secret_id: String,
    /// Optional region, vault name, or endpoint override.
    pub endpoint: Option<String>,
}

impl From<KmsRequest> for KmsUri {
    fn from(r: KmsRequest) -> Self {
        KmsUri {
            backend: r.backend,
            secret_id: r.secret_id,
            endpoint: r.endpoint,
        }
    }
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Resolve a secret reference to its plaintext value.
///
/// The returned value is wrapped in `Zeroizing` so it is wiped from memory
/// when dropped.
pub fn fetch(uri: &KmsUri) -> Result<Zeroizing<String>> {
    match uri.backend {
        KmsBackend::Env => fetch_from_env(&uri.secret_id),
        KmsBackend::File => fetch_from_file(&uri.secret_id),
        KmsBackend::AwsSecretsManager => fetch_from_aws(uri),
        KmsBackend::AzureKeyVault => fetch_from_azure(uri),
        KmsBackend::HashicorpVault => fetch_from_hashicorp(uri),
    }
}

/// Resolve a string that may be either a KMS URI or a literal secret.
///
/// Literal values are returned unchanged, so this is safe to call on any
/// user-supplied passphrase.
pub fn resolve(value: &str) -> Result<Zeroizing<String>> {
    if is_kms_uri(value) {
        let uri = value.parse::<KmsUri>()?;

        // `scheme()` is one of five `&'static str` literals. The rest of the
        // URI is deliberately untouched: `secret_id` names a secret and
        // `endpoint` names infrastructure, and both are exactly the sort of
        // free-form value `feature.detail` must never carry.
        crate::telemetry::track_feature("kms", Some(uri.backend.scheme()));

        fetch(&uri)
    } else {
        Ok(Zeroizing::new(value.to_string()))
    }
}

/// Fetch a passphrase from the configured KMS backend.
pub fn fetch_secret(req: &KmsRequest) -> Result<Zeroizing<String>> {
    fetch(&KmsUri {
        backend: req.backend,
        secret_id: req.secret_id.clone(),
        endpoint: req.endpoint.clone(),
    })
}

/// List available backends (useful for CLI help text).
#[must_use]
pub fn available_backends() -> Vec<KmsBackend> {
    vec![
        KmsBackend::Env,
        KmsBackend::File,
        KmsBackend::AwsSecretsManager,
        KmsBackend::AzureKeyVault,
        KmsBackend::HashicorpVault,
    ]
}

// ── Backend implementations ──────────────────────────────────────────────────

fn fetch_from_env(var_name: &str) -> Result<Zeroizing<String>> {
    std::env::var(var_name)
        .map(Zeroizing::new)
        .map_err(|_| VaultError::ConfigError(format!("Environment variable '{var_name}' not set")))
}

fn fetch_from_file(path: &str) -> Result<Zeroizing<String>> {
    let path = std::path::Path::new(path);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(path)?;
        let mode = meta.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            return Err(VaultError::SecurityViolation(format!(
                "Secret file {} is readable by group/other (mode {:o}); chmod 600 it",
                path.display(),
                mode
            )));
        }
    }

    let contents = std::fs::read_to_string(path)?;
    Ok(Zeroizing::new(
        contents.trim_end_matches(['\n', '\r']).to_string(),
    ))
}

/// HashiCorp Vault via the HTTP KV API.
///
/// Tries KV v2 (`/v1/<mount>/data/<path>`) first, then falls back to KV v1
/// (`/v1/<mount>/<path>`). The final URI segment names the field to read.
fn fetch_from_hashicorp(uri: &KmsUri) -> Result<Zeroizing<String>> {
    let addr = uri
        .endpoint
        .clone()
        .or_else(|| std::env::var("VAULT_ADDR").ok())
        .ok_or_else(|| {
            VaultError::ConfigError(
                "HashiCorp Vault address not set — export VAULT_ADDR (e.g. https://vault:8200)"
                    .to_string(),
            )
        })?;
    let token = std::env::var("VAULT_TOKEN").map_err(|_| {
        VaultError::ConfigError("VAULT_TOKEN not set — required for HashiCorp Vault".to_string())
    })?;

    let (secret_path, field) = uri.secret_id.rsplit_once('/').ok_or_else(|| {
        VaultError::InvalidInput(format!(
            "HashiCorp Vault path must include a field: vault://<mount>/<path>/<key>, got {}",
            uri.secret_id
        ))
    })?;
    let (mount, rest) = secret_path.split_once('/').unwrap_or((secret_path, ""));

    let addr = addr.trim_end_matches('/');
    let v2 = if rest.is_empty() {
        format!("{addr}/v1/{mount}/data")
    } else {
        format!("{addr}/v1/{mount}/data/{rest}")
    };
    let v1 = format!("{addr}/v1/{secret_path}");

    let client = http_client()?;

    let mut body: Option<serde_json::Value> = None;
    for url in [v2, v1] {
        let resp = client
            .get(&url)
            .header("X-Vault-Token", &token)
            .send()
            .map_err(|e| VaultError::ConfigError(format!("HashiCorp Vault request failed: {e}")))?;
        if resp.status().as_u16() == 404 {
            continue;
        }
        if !resp.status().is_success() {
            return Err(VaultError::ConfigError(format!(
                "HashiCorp Vault returned {} for {url}",
                resp.status()
            )));
        }
        body = Some(resp.json().map_err(|e| {
            VaultError::SerializationError(format!("Invalid JSON from HashiCorp Vault: {e}"))
        })?);
        break;
    }

    let body = body.ok_or_else(|| {
        VaultError::ConfigError(format!(
            "Secret '{secret_path}' not found in HashiCorp Vault (tried KV v2 and v1)"
        ))
    })?;

    // KV v2 nests the map one level deeper than KV v1.
    let data = body
        .get("data")
        .and_then(|d| d.get("data").or(Some(d)))
        .ok_or_else(|| {
            VaultError::ConfigError("HashiCorp Vault response had no `data` object".to_string())
        })?;

    let value = data
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            VaultError::ConfigError(format!(
                "Field '{field}' not present in HashiCorp Vault secret '{secret_path}'"
            ))
        })?;

    Ok(Zeroizing::new(value.to_string()))
}

/// Azure Key Vault via the REST API.
///
/// Authenticates with a bearer token from `AZURE_KEYVAULT_TOKEN` (or
/// `AZURE_ACCESS_TOKEN`), which can be minted with:
/// `az account get-access-token --resource https://vault.azure.net --query accessToken -o tsv`
fn fetch_from_azure(uri: &KmsUri) -> Result<Zeroizing<String>> {
    let vault = uri.endpoint.as_deref().ok_or_else(|| {
        VaultError::InvalidInput("Azure Key Vault URI must name a vault".to_string())
    })?;
    let token = std::env::var("AZURE_KEYVAULT_TOKEN")
        .or_else(|_| std::env::var("AZURE_ACCESS_TOKEN"))
        .map_err(|_| {
            VaultError::ConfigError(
                "AZURE_KEYVAULT_TOKEN not set — mint one with \
                 `az account get-access-token --resource https://vault.azure.net \
                 --query accessToken -o tsv`"
                    .to_string(),
            )
        })?;

    // Accept both a bare vault name and a fully-qualified host.
    let host = if vault.contains('.') {
        vault.to_string()
    } else {
        format!("{vault}.vault.azure.net")
    };
    let url = format!(
        "https://{host}/secrets/{}?api-version=7.4",
        uri.secret_id.trim_start_matches('/')
    );

    let resp = http_client()?
        .get(&url)
        .bearer_auth(&token)
        .send()
        .map_err(|e| VaultError::ConfigError(format!("Azure Key Vault request failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(VaultError::ConfigError(format!(
            "Azure Key Vault returned {} for secret '{}'",
            resp.status(),
            uri.secret_id
        )));
    }

    let body: serde_json::Value = resp.json().map_err(|e| {
        VaultError::SerializationError(format!("Invalid JSON from Azure Key Vault: {e}"))
    })?;

    let value = body
        .get("value")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            VaultError::ConfigError(format!(
                "Azure Key Vault secret '{}' had no `value` field",
                uri.secret_id
            ))
        })?;

    Ok(Zeroizing::new(value.to_string()))
}

/// AWS Secrets Manager via the AWS SDK (requires the `s3` feature).
#[cfg(feature = "s3")]
fn fetch_from_aws(uri: &KmsUri) -> Result<Zeroizing<String>> {
    let secret_id = uri.secret_id.clone();
    let region_override = uri
        .endpoint
        .clone()
        .or_else(|| std::env::var("AWS_REGION").ok());

    let fetch = async move {
        let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
        if let Some(region) = region_override {
            loader = loader.region(aws_sdk_secretsmanager::config::Region::new(region));
        }
        let config = loader.load().await;
        let client = aws_sdk_secretsmanager::Client::new(&config);

        let out = client
            .get_secret_value()
            .secret_id(&secret_id)
            .send()
            .await
            .map_err(|e| {
                VaultError::ConfigError(format!(
                    "AWS Secrets Manager GetSecretValue failed for '{secret_id}': {e}"
                ))
            })?;

        // Secrets Manager stores either a string or a binary blob.
        if let Some(s) = out.secret_string() {
            return Ok(Zeroizing::new(s.to_string()));
        }
        if let Some(blob) = out.secret_binary() {
            return String::from_utf8(blob.as_ref().to_vec())
                .map(Zeroizing::new)
                .map_err(|_| {
                    VaultError::ConfigError(format!(
                        "AWS secret '{secret_id}' is binary and not valid UTF-8"
                    ))
                });
        }
        Err(VaultError::ConfigError(format!(
            "AWS secret '{secret_id}' has neither a string nor a binary value"
        )))
    };

    block_on(fetch)
}

#[cfg(not(feature = "s3"))]
fn fetch_from_aws(uri: &KmsUri) -> Result<Zeroizing<String>> {
    Err(VaultError::ConfigError(format!(
        "AWS Secrets Manager support is not compiled in (secret: {}). \
         Rebuild with `--features s3`.",
        uri.secret_id
    )))
}

/// Run a future to completion from sync code, whether or not a Tokio runtime
/// is already driving the current thread.
#[cfg(feature = "s3")]
fn block_on<F: std::future::Future<Output = Result<Zeroizing<String>>>>(
    fut: F,
) -> Result<Zeroizing<String>> {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(VaultError::IoError)?
            .block_on(fut),
    }
}

/// Shared HTTP client for the REST-based backends.
fn http_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| VaultError::ConfigError(format!("Failed to build HTTP client: {e}")))
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_parse() {
        assert_eq!("env".parse::<KmsBackend>().unwrap(), KmsBackend::Env);
        assert_eq!("file".parse::<KmsBackend>().unwrap(), KmsBackend::File);
        assert_eq!(
            "aws".parse::<KmsBackend>().unwrap(),
            KmsBackend::AwsSecretsManager
        );
        assert_eq!(
            "azure-key-vault".parse::<KmsBackend>().unwrap(),
            KmsBackend::AzureKeyVault
        );
        assert_eq!(
            "hashicorp".parse::<KmsBackend>().unwrap(),
            KmsBackend::HashicorpVault
        );
        assert!("unknown".parse::<KmsBackend>().is_err());
    }

    #[test]
    fn test_backend_scheme_roundtrip() {
        for b in available_backends() {
            assert_eq!(b.scheme().parse::<KmsBackend>().unwrap(), b);
        }
    }

    #[test]
    fn test_uri_parse_env() {
        let uri: KmsUri = "env://MY_SECRET".parse().unwrap();
        assert_eq!(uri.backend, KmsBackend::Env);
        assert_eq!(uri.secret_id, "MY_SECRET");
        assert_eq!(uri.to_string(), "env://MY_SECRET");
    }

    #[test]
    fn test_uri_parse_file_posix_and_windows() {
        let posix: KmsUri = "file:///etc/ironvault/pass".parse().unwrap();
        assert_eq!(posix.secret_id, "/etc/ironvault/pass");

        let windows: KmsUri = "file:///c:/secrets/pass.txt".parse().unwrap();
        assert_eq!(windows.secret_id, "c:/secrets/pass.txt");

        let relative: KmsUri = "file://./pass.txt".parse().unwrap();
        assert_eq!(relative.secret_id, "./pass.txt");
    }

    #[test]
    fn test_uri_parse_azure() {
        let uri: KmsUri = "azure-kv://my-vault/hmac-key".parse().unwrap();
        assert_eq!(uri.backend, KmsBackend::AzureKeyVault);
        assert_eq!(uri.endpoint.as_deref(), Some("my-vault"));
        assert_eq!(uri.secret_id, "hmac-key");
        assert_eq!(uri.to_string(), "azure-kv://my-vault/hmac-key");

        assert!("azure-kv://only-vault".parse::<KmsUri>().is_err());
    }

    #[test]
    fn test_uri_parse_hashicorp() {
        let uri: KmsUri = "vault://secret/iv/passphrase".parse().unwrap();
        assert_eq!(uri.backend, KmsBackend::HashicorpVault);
        assert_eq!(uri.secret_id, "secret/iv/passphrase");

        assert!("vault://nofield".parse::<KmsUri>().is_err());
    }

    #[test]
    fn test_uri_parse_aws() {
        let uri: KmsUri = "aws-sm://prod/iv-passphrase".parse().unwrap();
        assert_eq!(uri.backend, KmsBackend::AwsSecretsManager);
        assert_eq!(uri.secret_id, "prod/iv-passphrase");
    }

    #[test]
    fn test_uri_parse_rejects_non_uri() {
        assert!("hunter2".parse::<KmsUri>().is_err());
        assert!("ftp://host/x".parse::<KmsUri>().is_err());
        assert!("env://".parse::<KmsUri>().is_err());
    }

    #[test]
    fn test_is_kms_uri() {
        assert!(is_kms_uri("env://X"));
        assert!(is_kms_uri("vault://secret/a/b"));
        assert!(!is_kms_uri("hunter2"));
        assert!(!is_kms_uri("env://"));
        assert!(!is_kms_uri("https://example.com"));
    }

    #[test]
    fn test_fetch_from_env() {
        std::env::set_var("IRONVAULT_TEST_SECRET_KMS", "super-secret-passphrase");
        let uri: KmsUri = "env://IRONVAULT_TEST_SECRET_KMS".parse().unwrap();
        assert_eq!(&*fetch(&uri).unwrap(), "super-secret-passphrase");
        std::env::remove_var("IRONVAULT_TEST_SECRET_KMS");
    }

    #[test]
    fn test_env_missing() {
        let uri: KmsUri = "env://IRONVAULT_DEFINITELY_NOT_SET_42".parse().unwrap();
        assert!(fetch(&uri).is_err());
    }

    #[test]
    fn test_fetch_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pass.txt");
        std::fs::write(&path, "file-passphrase\n").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }

        let uri = KmsUri {
            backend: KmsBackend::File,
            secret_id: path.to_string_lossy().to_string(),
            endpoint: None,
        };
        // Trailing newline is trimmed so `echo secret > f` works as expected.
        assert_eq!(&*fetch(&uri).unwrap(), "file-passphrase");
    }

    #[cfg(unix)]
    #[test]
    fn test_file_rejects_loose_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("loose.txt");
        std::fs::write(&path, "secret").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

        let uri = KmsUri {
            backend: KmsBackend::File,
            secret_id: path.to_string_lossy().to_string(),
            endpoint: None,
        };
        assert!(fetch(&uri).is_err());
    }

    #[test]
    fn test_file_missing() {
        let uri: KmsUri = "file:///definitely/not/here/iv-secret".parse().unwrap();
        assert!(fetch(&uri).is_err());
    }

    #[test]
    fn test_resolve_passes_through_literals() {
        assert_eq!(&*resolve("hunter2").unwrap(), "hunter2");
    }

    #[test]
    fn test_resolve_follows_uri() {
        std::env::set_var("IRONVAULT_TEST_RESOLVE", "from-env");
        assert_eq!(&*resolve("env://IRONVAULT_TEST_RESOLVE").unwrap(), "from-env");
        std::env::remove_var("IRONVAULT_TEST_RESOLVE");
    }

    #[test]
    fn test_hashicorp_requires_config() {
        // No VAULT_ADDR / VAULT_TOKEN in the test environment.
        std::env::remove_var("VAULT_ADDR");
        std::env::remove_var("VAULT_TOKEN");
        let uri: KmsUri = "vault://secret/iv/pass".parse().unwrap();
        assert!(fetch(&uri).is_err());
    }

    #[test]
    fn test_azure_requires_token() {
        std::env::remove_var("AZURE_KEYVAULT_TOKEN");
        std::env::remove_var("AZURE_ACCESS_TOKEN");
        let uri: KmsUri = "azure-kv://my-vault/secret".parse().unwrap();
        assert!(fetch(&uri).is_err());
    }

    #[cfg(not(feature = "s3"))]
    #[test]
    fn test_aws_reports_missing_feature() {
        let uri: KmsUri = "aws-sm://prod/pass".parse().unwrap();
        let err = fetch(&uri).unwrap_err().to_string();
        assert!(err.contains("--features s3"), "unexpected error: {err}");
    }

    #[test]
    fn test_legacy_request_api() {
        std::env::set_var("IRONVAULT_TEST_LEGACY", "legacy-value");
        let req = KmsRequest {
            backend: KmsBackend::Env,
            secret_id: "IRONVAULT_TEST_LEGACY".into(),
            endpoint: None,
        };
        assert_eq!(&*fetch_secret(&req).unwrap(), "legacy-value");
        std::env::remove_var("IRONVAULT_TEST_LEGACY");
    }

    #[test]
    fn test_available_backends() {
        assert_eq!(available_backends().len(), 5);
    }
}
