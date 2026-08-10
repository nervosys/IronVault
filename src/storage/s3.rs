//! AWS S3 storage backend

use async_trait::async_trait;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{primitives::ByteStream, Client};

use crate::error::{Result, VaultError};
use crate::storage::StorageBackend;

/// AWS S3 storage backend
pub struct S3Backend {
    client: Client,
    bucket: String,
    prefix: String,
}

impl S3Backend {
    /// Create new S3 storage backend
    ///
    /// # Arguments
    /// * `bucket` - S3 bucket name
    /// * `region` - AWS region (e.g., "us-east-1")
    /// * `prefix` - Optional key prefix (folder path)
    pub async fn new(bucket: String, region: String, prefix: Option<String>) -> Result<Self> {
        let region_provider =
            RegionProviderChain::first_try(aws_sdk_s3::config::Region::new(region));

        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;

        let client = Client::new(&config);

        Ok(Self {
            client,
            bucket,
            prefix: prefix.unwrap_or_default(),
        })
    }

    fn get_key(&self, key: &str) -> String {
        if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}/{}", self.prefix.trim_end_matches('/'), key)
        }
    }
}

#[async_trait]
impl StorageBackend for S3Backend {
    async fn upload(&self, key: &str, data: &[u8]) -> Result<()> {
        let s3_key = self.get_key(key);
        let byte_stream = ByteStream::from(data.to_vec());

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .body(byte_stream)
            .send()
            .await
            .map_err(|e| VaultError::StorageError(format!("S3 upload failed: {}", e)))?;

        Ok(())
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let s3_key = self.get_key(key);

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("NoSuchKey") {
                    VaultError::ModelNotFound(key.to_string())
                } else {
                    VaultError::StorageError(format!("S3 download failed: {}", e))
                }
            })?;

        let data = response
            .body
            .collect()
            .await
            .map_err(|e| VaultError::StorageError(format!("S3 body read failed: {}", e)))?
            .into_bytes()
            .to_vec();

        Ok(data)
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let s3_key = self.get_key(key);

        // Check if exists first
        let exists = self.exists(key).await?;
        if !exists {
            return Ok(false);
        }

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .send()
            .await
            .map_err(|e| VaultError::StorageError(format!("S3 delete failed: {}", e)))?;

        Ok(true)
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let s3_key = self.get_key(key);

        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("NotFound") {
                    Ok(false)
                } else {
                    Err(VaultError::StorageError(format!("S3 head failed: {}", e)))
                }
            }
        }
    }

    async fn list(&self) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut request = self.client.list_objects_v2().bucket(&self.bucket);

            if !self.prefix.is_empty() {
                request = request.prefix(&self.prefix);
            }

            if let Some(token) = continuation_token {
                request = request.continuation_token(token);
            }

            let response = request
                .send()
                .await
                .map_err(|e| VaultError::StorageError(format!("S3 list failed: {}", e)))?;

            if let Some(contents) = response.contents {
                for object in contents {
                    if let Some(key) = object.key {
                        // Strip prefix if present
                        let clean_key = if !self.prefix.is_empty() {
                            key.strip_prefix(&format!("{}/", self.prefix.trim_end_matches('/')))
                                .unwrap_or(&key)
                                .to_string()
                        } else {
                            key
                        };
                        keys.push(clean_key);
                    }
                }
            }

            if response.is_truncated.unwrap_or(false) {
                continuation_token = response.next_continuation_token;
            } else {
                break;
            }
        }

        Ok(keys)
    }

    async fn size(&self, key: &str) -> Result<u64> {
        let s3_key = self.get_key(key);

        let response = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&s3_key)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("NotFound") {
                    VaultError::ModelNotFound(key.to_string())
                } else {
                    VaultError::StorageError(format!("S3 head failed: {}", e))
                }
            })?;

        Ok(response.content_length.unwrap_or(0) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require AWS credentials and a test bucket
    // They are disabled by default. Enable with: cargo test --features s3-integration-tests

    #[tokio::test]
    #[ignore = "requires live AWS credentials and a test bucket"]
    async fn test_s3_backend() {
        let bucket = std::env::var("TEST_S3_BUCKET").unwrap();
        let region = std::env::var("TEST_S3_REGION").unwrap_or("us-east-1".to_string());

        let backend = S3Backend::new(bucket, region, Some("test-ironvault".to_string()))
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
