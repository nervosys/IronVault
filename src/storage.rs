//! Storage backend for encrypted model data
//!
//! Supports multiple storage backends:
//! - Local filesystem (default)
//! - AWS S3
//! - Azure Blob Storage
//! - Google Cloud Storage

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::crypto::compression::{compress, decompress, CompressionAlgorithm, CompressionLevel};
use crate::crypto::{SecureKey, VaultCrypto};
use crate::error::{Result, VaultError};
use async_trait::async_trait;

// Cloud backend modules
pub mod local;

#[cfg(feature = "s3")]
pub mod s3;

#[cfg(feature = "azure")]
pub mod azure;

// GCS support disabled due to security vulnerabilities in cloud-storage dependency
// See SECURITY_AUDIT.md for details on RUSTSEC-2025-0009 and RUSTSEC-2025-0010
// #[cfg(feature = "gcs")]
// pub mod gcs;

/// Storage backend trait for different storage providers
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Upload data to storage
    async fn upload(&self, key: &str, data: &[u8]) -> Result<()>;

    /// Download data from storage
    async fn download(&self, key: &str) -> Result<Vec<u8>>;

    /// Delete data from storage
    async fn delete(&self, key: &str) -> Result<bool>;

    /// Check if key exists
    async fn exists(&self, key: &str) -> Result<bool>;

    /// List all keys (files)
    async fn list(&self) -> Result<Vec<String>>;

    /// Get size of stored data
    async fn size(&self, key: &str) -> Result<u64>;
}

/// Storage backend configuration
#[derive(Debug, Clone)]
pub enum StorageConfig {
    /// Local filesystem storage.
    Local { path: PathBuf },
    /// AWS S3 object storage.
    S3 {
        bucket: String,
        region: String,
        prefix: Option<String>,
    },
    /// Azure Blob storage.
    Azure {
        account: String,
        container: String,
        prefix: Option<String>,
    },
    /// Google Cloud Storage.
    Gcs {
        bucket: String,
        project: String,
        prefix: Option<String>,
    },
}

impl StorageConfig {
    /// Create a storage backend from configuration
    ///
    /// `async` is not redundant: the `s3` and `azure` arms below await their
    /// backend constructors. Clippy only sees an unused `async` in a build
    /// where both features are off and those arms are compiled out — dropping
    /// it would break every feature-enabled build and every caller.
    #[allow(clippy::unused_async)]
    pub async fn create_backend(&self) -> Result<Box<dyn StorageBackend>> {
        match self {
            StorageConfig::Local { path } => {
                let backend = local::LocalBackend::new(path.clone())?;
                Ok(Box::new(backend))
            }
            #[cfg(feature = "s3")]
            StorageConfig::S3 {
                bucket,
                region,
                prefix,
            } => {
                let backend =
                    s3::S3Backend::new(bucket.clone(), region.clone(), prefix.clone()).await?;
                Ok(Box::new(backend))
            }
            #[cfg(not(feature = "s3"))]
            StorageConfig::S3 { .. } => Err(VaultError::ConfigError(
                "S3 support not enabled. Rebuild with --features s3".to_string(),
            )),
            #[cfg(feature = "azure")]
            StorageConfig::Azure {
                account,
                container,
                prefix,
            } => {
                let backend =
                    azure::AzureBackend::new(account.clone(), container.clone(), prefix.clone())
                        .await?;
                Ok(Box::new(backend))
            }
            #[cfg(not(feature = "azure"))]
            StorageConfig::Azure { .. } => Err(VaultError::ConfigError(
                "Azure support not enabled. Rebuild with --features azure".to_string(),
            )),
            // GCS support disabled due to critical security vulnerabilities
            // in cloud-storage dependency (RUSTSEC-2025-0009, RUSTSEC-2025-0010)
            StorageConfig::Gcs { .. } => Err(VaultError::ConfigError(
                "GCS support temporarily disabled due to security vulnerabilities. Use S3 or Azure instead.".to_string(),
            )),
        }
    }
}

/// Storage backend for encrypted and compressed model data
pub struct Storage {
    vault_path: PathBuf,
    crypto: VaultCrypto,
}

impl Storage {
    /// Create new storage instance
    pub fn new(vault_path: &Path) -> Result<Self> {
        if !vault_path.exists() {
            fs::create_dir_all(vault_path)?;
            crate::permissions::restrict_dir(vault_path)?;
        }

        Ok(Self {
            vault_path: vault_path.to_path_buf(),
            crypto: VaultCrypto::new()?,
        })
    }

    /// Store data (compress then encrypt)
    pub fn store(
        &self,
        filename: &str,
        data: &[u8],
        key: &SecureKey,
        compression: CompressionAlgorithm,
        compression_level: CompressionLevel,
    ) -> Result<(u64, u64)> {
        // Compress data
        let compressed = compress(data, compression, compression_level)?;
        let compressed_size = compressed.len() as u64;

        // Encrypt compressed data
        let encrypted = self.crypto.encrypt(&compressed, key)?;

        // Write to file
        let file_path = self.vault_path.join(filename);
        let mut file = File::create(&file_path)?;
        file.write_all(&encrypted)?;
        crate::permissions::restrict_file(&file_path)?;

        Ok((data.len() as u64, compressed_size))
    }

    /// Retrieve data (decrypt then decompress)
    pub fn retrieve(
        &self,
        filename: &str,
        key: &SecureKey,
        compression: CompressionAlgorithm,
    ) -> Result<Vec<u8>> {
        let file_path = self.vault_path.join(filename);

        if !file_path.exists() {
            return Err(VaultError::ModelNotFound(filename.to_string()));
        }

        // Read encrypted data
        let mut file = File::open(&file_path)?;
        let mut encrypted = Vec::new();
        file.read_to_end(&mut encrypted)?;

        // Decrypt
        let compressed = self.crypto.decrypt(&encrypted, key)?;

        // Decompress
        let data = decompress(&compressed, compression)?;

        Ok(data)
    }

    /// Delete stored file
    pub fn delete(&self, filename: &str) -> Result<bool> {
        let file_path = self.vault_path.join(filename);

        if file_path.exists() {
            fs::remove_file(&file_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Check if file exists
    pub fn exists(&self, filename: &str) -> bool {
        self.vault_path.join(filename).exists()
    }

    /// Get file size
    pub fn file_size(&self, filename: &str) -> Result<u64> {
        let file_path = self.vault_path.join(filename);
        let metadata = fs::metadata(&file_path)?;
        Ok(metadata.len())
    }

    /// List all stored files
    pub fn list_files(&self) -> Result<Vec<String>> {
        let mut files = Vec::new();

        for entry in fs::read_dir(&self.vault_path)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                if let Some(name) = entry.file_name().to_str() {
                    files.push(name.to_string());
                }
            }
        }

        Ok(files)
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> Result<StorageStats> {
        let mut total_size = 0u64;
        let mut file_count = 0usize;

        for entry in fs::read_dir(&self.vault_path)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                total_size += entry.metadata()?.len();
                file_count += 1;
            }
        }

        Ok(StorageStats {
            total_size_bytes: total_size,
            file_count,
        })
    }

    /// Store data using chunked streaming encryption (constant memory).
    ///
    /// Compresses the data first, then encrypts in fixed-size chunks.
    /// Each chunk is independently authenticated; a stream MAC guards
    /// against truncation and reordering.
    pub fn store_streamed(
        &self,
        filename: &str,
        data: &[u8],
        key: &SecureKey,
        compression: CompressionAlgorithm,
        compression_level: CompressionLevel,
    ) -> Result<(u64, u64)> {
        use crate::crypto::streaming::encrypt_chunked;

        // Compress data
        let compressed = compress(data, compression, compression_level)?;
        let compressed_size = compressed.len() as u64;

        // Encrypt using chunked streaming (default 4 MiB chunks)
        let encrypted = encrypt_chunked(&self.crypto, &compressed, key, 0)?;

        // Write to file
        let file_path = self.vault_path.join(filename);
        let mut file = File::create(&file_path)?;
        file.write_all(&encrypted)?;
        crate::permissions::restrict_file(&file_path)?;

        Ok((data.len() as u64, compressed_size))
    }

    /// Retrieve data stored with chunked streaming encryption.
    ///
    /// Auto-detects the AIMV chunked format and uses the appropriate
    /// decryption path. Falls back to monolithic decryption for
    /// legacy (non-chunked) files.
    pub fn retrieve_auto(
        &self,
        filename: &str,
        key: &SecureKey,
        compression: CompressionAlgorithm,
    ) -> Result<Vec<u8>> {
        use crate::crypto::streaming::{decrypt_chunked, is_chunked_format};

        let file_path = self.vault_path.join(filename);

        if !file_path.exists() {
            return Err(VaultError::ModelNotFound(filename.to_string()));
        }

        // Read encrypted data
        let mut file = File::open(&file_path)?;
        let mut encrypted = Vec::new();
        file.read_to_end(&mut encrypted)?;

        // Auto-detect format
        let compressed = if is_chunked_format(&encrypted) {
            decrypt_chunked(&self.crypto, &encrypted, key)?
        } else {
            self.crypto.decrypt(&encrypted, key)?
        };

        // Decompress
        let data = decompress(&compressed, compression)?;

        Ok(data)
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_size_bytes: u64,
    pub file_count: usize,
}

// ── Trait implementation ─────────────────────────────────────

impl crate::traits::BlobStore for Storage {
    fn put(&self, key: &str, data: &[u8], encryption_key: &SecureKey) -> Result<(u64, u64)> {
        self.store(
            key,
            data,
            encryption_key,
            crate::crypto::compression::CompressionAlgorithm::Gzip,
            crate::crypto::compression::CompressionLevel::Balanced,
        )
    }

    fn get(&self, key: &str, encryption_key: &SecureKey) -> Result<Vec<u8>> {
        self.retrieve(
            key,
            encryption_key,
            crate::crypto::compression::CompressionAlgorithm::Gzip,
        )
    }

    fn remove(&self, key: &str) -> Result<bool> {
        self.delete(key)
    }

    fn exists(&self, key: &str) -> bool {
        Storage::exists(self, key)
    }

    fn size(&self, key: &str) -> Result<u64> {
        self.file_size(key)
    }

    fn list_keys(&self) -> Result<Vec<String>> {
        self.list_files()
    }

    fn stats(&self) -> Result<crate::traits::BlobStoreStats> {
        let s = self.get_stats()?;
        Ok(crate::traits::BlobStoreStats {
            total_size_bytes: s.total_size_bytes,
            file_count: s.file_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_storage() {
        let temp_dir = tempdir().unwrap();
        let storage = Storage::new(temp_dir.path()).unwrap();

        let crypto = VaultCrypto::new().unwrap();
        let passphrase = b"test_passphrase_with_sufficient_entropy".to_vec();
        let (key, _) = crypto.derive_key(passphrase, None).unwrap();

        let data = b"Test model data";
        // Get stats
        let (orig_size, _comp_size) = storage
            .store(
                "test.enc",
                data,
                &key,
                CompressionAlgorithm::Gzip,
                CompressionLevel::Balanced,
            )
            .unwrap();

        assert_eq!(orig_size, data.len() as u64);

        let retrieved = storage
            .retrieve("test.enc", &key, CompressionAlgorithm::Gzip)
            .unwrap();

        assert_eq!(data, &retrieved[..]);
    }

    #[test]
    fn test_storage_delete_and_exists() {
        let temp_dir = tempdir().unwrap();
        let storage = Storage::new(temp_dir.path()).unwrap();
        let crypto = VaultCrypto::new().unwrap();
        let (key, _) = crypto.derive_key(b"delete_test".to_vec(), None).unwrap();

        storage
            .store(
                "del.enc",
                b"data",
                &key,
                CompressionAlgorithm::None,
                CompressionLevel::None,
            )
            .unwrap();

        assert!(storage.exists("del.enc"));
        assert!(storage.delete("del.enc").unwrap());
        assert!(!storage.exists("del.enc"));
        assert!(!storage.delete("del.enc").unwrap());
    }

    #[test]
    fn test_storage_file_size_and_list() {
        let temp_dir = tempdir().unwrap();
        let storage = Storage::new(temp_dir.path()).unwrap();
        let crypto = VaultCrypto::new().unwrap();
        let (key, _) = crypto.derive_key(b"list_test".to_vec(), None).unwrap();

        storage
            .store(
                "a.enc",
                b"aaaa",
                &key,
                CompressionAlgorithm::None,
                CompressionLevel::None,
            )
            .unwrap();
        storage
            .store(
                "b.enc",
                b"bb",
                &key,
                CompressionAlgorithm::None,
                CompressionLevel::None,
            )
            .unwrap();

        let files = storage.list_files().unwrap();
        assert_eq!(files.len(), 2);

        let size = storage.file_size("a.enc").unwrap();
        assert!(size > 0);
    }

    #[test]
    fn test_storage_stats() {
        let temp_dir = tempdir().unwrap();
        let storage = Storage::new(temp_dir.path()).unwrap();
        let crypto = VaultCrypto::new().unwrap();
        let (key, _) = crypto.derive_key(b"stats_test".to_vec(), None).unwrap();

        storage
            .store(
                "x.enc",
                b"hello",
                &key,
                CompressionAlgorithm::Gzip,
                CompressionLevel::Fast,
            )
            .unwrap();

        let stats = storage.get_stats().unwrap();
        assert_eq!(stats.file_count, 1);
        assert!(stats.total_size_bytes > 0);
    }

    #[test]
    fn test_storage_streamed_and_retrieve_auto() {
        let temp_dir = tempdir().unwrap();
        let storage = Storage::new(temp_dir.path()).unwrap();
        let crypto = VaultCrypto::new().unwrap();
        let (key, _) = crypto.derive_key(b"stream_test".to_vec(), None).unwrap();

        let data = vec![0xAB; 1024];
        storage
            .store_streamed(
                "s.enc",
                &data,
                &key,
                CompressionAlgorithm::Gzip,
                CompressionLevel::Balanced,
            )
            .unwrap();

        let retrieved = storage
            .retrieve_auto("s.enc", &key, CompressionAlgorithm::Gzip)
            .unwrap();
        assert_eq!(data, retrieved);
    }

    #[test]
    fn test_storage_retrieve_auto_legacy_fallback() {
        let temp_dir = tempdir().unwrap();
        let storage = Storage::new(temp_dir.path()).unwrap();
        let crypto = VaultCrypto::new().unwrap();
        let (key, _) = crypto.derive_key(b"legacy_test".to_vec(), None).unwrap();

        storage
            .store(
                "legacy.enc",
                b"old format",
                &key,
                CompressionAlgorithm::None,
                CompressionLevel::None,
            )
            .unwrap();

        let retrieved = storage
            .retrieve_auto("legacy.enc", &key, CompressionAlgorithm::None)
            .unwrap();
        assert_eq!(b"old format", &retrieved[..]);
    }

    #[test]
    fn test_storage_retrieve_missing_file() {
        let temp_dir = tempdir().unwrap();
        let storage = Storage::new(temp_dir.path()).unwrap();
        let crypto = VaultCrypto::new().unwrap();
        let (key, _) = crypto.derive_key(b"missing_test".to_vec(), None).unwrap();

        let result = storage.retrieve("nonexistent.enc", &key, CompressionAlgorithm::None);
        assert!(result.is_err());
    }

    #[test]
    fn test_storage_blob_store_trait() {
        let temp_dir = tempdir().unwrap();
        let storage = Storage::new(temp_dir.path()).unwrap();
        let crypto = VaultCrypto::new().unwrap();
        let (key, _) = crypto.derive_key(b"blob_test".to_vec(), None).unwrap();

        use crate::traits::BlobStore;

        let (orig, stored) = storage.put("blob.enc", b"blob data", &key).unwrap();
        assert!(orig > 0);
        assert!(stored > 0);

        let data = storage.get("blob.enc", &key).unwrap();
        assert_eq!(data, b"blob data");

        assert!(storage.exists("blob.enc"));
        let size = storage.size("blob.enc").unwrap();
        assert!(size > 0);

        let keys = storage.list_keys().unwrap();
        assert!(keys.contains(&"blob.enc".to_string()));

        let stats = storage.stats().unwrap();
        assert_eq!(stats.file_count, 1);

        assert!(storage.remove("blob.enc").unwrap());
        assert!(!storage.exists("blob.enc"));
    }

    #[test]
    fn test_storage_config_create_backend_s3_disabled() {
        let config = StorageConfig::S3 {
            bucket: "test-bucket".into(),
            region: "us-east-1".into(),
            prefix: None,
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(config.create_backend());
        #[cfg(not(feature = "s3"))]
        {
            assert!(result.is_err());
            let err_msg = format!("{}", result.err().unwrap());
            assert!(err_msg.contains("S3 support not enabled"));
        }
        // With `s3` enabled the call reaches the AWS SDK, whose outcome depends
        // on ambient credentials — asserting either way would be flaky.
        #[cfg(feature = "s3")]
        drop(result);
    }

    #[test]
    fn test_storage_config_create_backend_azure_disabled() {
        let config = StorageConfig::Azure {
            account: "test-account".into(),
            container: "test-container".into(),
            prefix: None,
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(config.create_backend());
        #[cfg(not(feature = "azure"))]
        {
            assert!(result.is_err());
            let err_msg = format!("{}", result.err().unwrap());
            assert!(err_msg.contains("Azure support not enabled"));
        }
        // With `azure` enabled the call reaches the Azure SDK, which requires
        // ambient credentials — asserting either way would be flaky.
        #[cfg(feature = "azure")]
        drop(result);
    }

    #[test]
    fn test_storage_config_create_backend_gcs_disabled() {
        let config = StorageConfig::Gcs {
            bucket: "test-bucket".into(),
            project: "test-project".into(),
            prefix: None,
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(config.create_backend());
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(err_msg.contains("GCS support temporarily disabled"));
    }

    #[test]
    fn test_storage_config_create_backend_local() {
        let temp_dir = tempdir().unwrap();
        let config = StorageConfig::Local {
            path: temp_dir.path().to_path_buf(),
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(config.create_backend());
        assert!(result.is_ok());
    }

    #[test]
    fn test_storage_new_creates_directory() {
        let temp_dir = tempdir().unwrap();
        let new_path = temp_dir.path().join("new_vault_dir");
        assert!(!new_path.exists());
        let _storage = Storage::new(&new_path).unwrap();
        assert!(new_path.exists());
    }

    #[test]
    fn test_storage_retrieve_auto_missing() {
        let temp_dir = tempdir().unwrap();
        let storage = Storage::new(temp_dir.path()).unwrap();
        let crypto = VaultCrypto::new().unwrap();
        let (key, _) = crypto.derive_key(b"auto_miss_test".to_vec(), None).unwrap();
        let result = storage.retrieve_auto("nonexistent.enc", &key, CompressionAlgorithm::None);
        assert!(result.is_err());
    }
}
