//! Cloud storage command handlers (push, pull, list, config).

use ironvault::{Result, VaultConfig, VaultError};
// Only the `cloud pull --store` path constructs metadata, and that path is
// compiled behind the cloud backend features.
#[cfg(any(feature = "s3", feature = "azure"))]
use ironvault::formats::{ModelFormat, ModelMetadata};

use crate::cli::args::CloudCommands;
use crate::cli::helpers::{build_vault, prompt_passphrase};

/// Decrypt a downloaded object, or pass it through if it predates sealing.
///
/// Objects pushed before 4.3.0 are plaintext and carry no magic. Refusing them
/// would strand data already in a bucket, so they are accepted with a warning
/// rather than an error -- but the warning matters, because a plaintext object
/// sitting in cloud storage is exactly the exposure sealing exists to close.
#[cfg(any(feature = "s3", feature = "azure"))]
fn unseal_downloaded(data: Vec<u8>, passphrase: &[u8]) -> Result<Vec<u8>> {
    use ironvault::cloud_envelope;

    if cloud_envelope::is_sealed(&data) {
        println!("🔓 Sealed object — decrypting");
        cloud_envelope::open(&data, passphrase.to_vec())
    } else {
        eprintln!(
            "\n⚠️  This object is NOT encrypted. It was uploaded by a version before\n   \
             4.3.0, which sent the plaintext model. It is readable by anyone with\n   \
             read access to the bucket. Re-push it with this version to seal it,\n   \
             then delete the old object."
        );
        Ok(data)
    }
}

pub fn handle_cloud(command: CloudCommands, config: VaultConfig, use_sqlite: bool) -> Result<()> {
    match command {
        CloudCommands::Push {
            model,
            version,
            provider,
            bucket,
        } => {
            println!("☁️  Pushing model to cloud storage");
            println!("   Model: {}", model);
            println!("   Provider: {}", provider);
            println!("   Bucket: {}", bucket);

            // Open vault and get model. The passphrase is needed twice --
            // once to unlock, once to seal the upload -- and `unlock` consumes
            // it, so keep a copy for the envelope.
            let mut vault = build_vault(config.clone(), use_sqlite)?;
            let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
            let seal_passphrase = passphrase.clone();
            vault.unlock(passphrase)?;

            // Get version to push
            let version_num = if let Some(v) = version {
                v
            } else {
                vault
                    .list_versions(&model)
                    .last()
                    .map(|mv| mv.version)
                    .ok_or_else(|| {
                        VaultError::ModelNotFound(format!(
                            "Model '{}' not found or has no versions",
                            model
                        ))
                    })?
            };

            // `get_model` decrypts and decompresses, so this is the plaintext
            // model. It is sealed below before anything leaves the process --
            // see `cloud_envelope`. Prior to 4.3.0 this buffer was uploaded
            // as-is.
            let plaintext = vault.get_model(&model, Some(version_num))?;
            let versions = vault.list_versions(&model);
            let model_version = versions
                .iter()
                .find(|v| v.version == version_num)
                .ok_or_else(|| VaultError::VersionNotFound(version_num, model.clone()))?;

            let _data = ironvault::cloud_envelope::seal(&plaintext, seal_passphrase)?;
            drop(plaintext);
            println!("🔒 Sealed with AES-256-GCM (Argon2id, per-object salt)");

            // Construct remote path
            let _remote_path = format!("{}/{}/v{}.vault", model, model_version.format, version_num);

            // Push to cloud based on provider
            match provider.to_lowercase().as_str() {
                "s3" => {
                    #[cfg(feature = "s3")]
                    {
                        use ironvault::storage::StorageConfig;
                        println!("📤 Uploading to S3...");
                        let region =
                            std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());
                        println!("   Region: {}", region);
                        println!("   Path: {}", _remote_path);
                        println!("   Size: {} bytes", _data.len());

                        let storage_config = StorageConfig::S3 {
                            bucket: bucket.clone(),
                            region,
                            prefix: None,
                        };
                        let rt = tokio::runtime::Runtime::new().map_err(|e| {
                            VaultError::StorageError(format!(
                                "Failed to create async runtime: {}",
                                e
                            ))
                        })?;
                        rt.block_on(async {
                            let backend = storage_config.create_backend().await?;
                            backend.upload(&_remote_path, &_data).await
                        })?;

                        println!("\n✅ Model pushed to S3 successfully!");
                        println!("   Bucket: {}", bucket);
                        println!("   Key: {}", _remote_path);
                    }
                    #[cfg(not(feature = "s3"))]
                    {
                        println!("⚠️  S3 support not enabled in this build");
                        println!("   To enable: cargo build --release --features s3");
                    }
                }
                "azure" => {
                    #[cfg(feature = "azure")]
                    {
                        use ironvault::storage::StorageConfig;
                        let account = std::env::var("AZURE_STORAGE_ACCOUNT").map_err(|_| {
                            VaultError::ConfigError(
                                "AZURE_STORAGE_ACCOUNT env var not set".to_string(),
                            )
                        })?;
                        println!("📤 Uploading to Azure Blob Storage...");
                        println!("   Container: {}", bucket);
                        println!("   Path: {}", _remote_path);
                        println!("   Size: {} bytes", _data.len());

                        let storage_config = StorageConfig::Azure {
                            account,
                            container: bucket.clone(),
                            prefix: None,
                        };
                        let rt = tokio::runtime::Runtime::new().map_err(|e| {
                            VaultError::StorageError(format!(
                                "Failed to create async runtime: {}",
                                e
                            ))
                        })?;
                        rt.block_on(async {
                            let backend = storage_config.create_backend().await?;
                            backend.upload(&_remote_path, &_data).await
                        })?;

                        println!("\n✅ Model pushed to Azure successfully!");
                    }
                    #[cfg(not(feature = "azure"))]
                    {
                        println!("⚠️  Azure support not enabled in this build");
                        println!("   To enable: cargo build --release --features azure");
                    }
                }
                "gcs" => {
                    println!(
                        "⚠️  GCS support temporarily disabled due to security vulnerabilities"
                    );
                    println!("   Use S3 or Azure instead. See SECURITY_AUDIT.md for details.");
                }
                _ => {
                    return Err(VaultError::InvalidInput(format!(
                        "Unsupported provider: {}. Use 's3', 'azure', or 'gcs'",
                        provider
                    )));
                }
            }
        }

        CloudCommands::Pull {
            model,
            provider,
            bucket,
            remote_path,
        } => {
            println!("☁️  Pulling model from cloud storage");
            println!("   Model: {}", model);
            println!("   Provider: {}", provider);
            println!("   Bucket: {}", bucket);
            println!("   Remote path: {}", remote_path);

            match provider.to_lowercase().as_str() {
                "s3" => {
                    #[cfg(feature = "s3")]
                    {
                        use ironvault::storage::StorageConfig;
                        let region =
                            std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());
                        println!("📥 Downloading from S3...");

                        let storage_config = StorageConfig::S3 {
                            bucket: bucket.clone(),
                            region,
                            prefix: None,
                        };
                        let rt = tokio::runtime::Runtime::new().map_err(|e| {
                            VaultError::StorageError(format!(
                                "Failed to create async runtime: {}",
                                e
                            ))
                        })?;
                        let data = rt.block_on(async {
                            let backend = storage_config.create_backend().await?;
                            backend.download(&remote_path).await
                        })?;

                        // Store into vault
                        let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
                        let data = unseal_downloaded(data, &passphrase)?;
                        let mut vault = build_vault(config.clone(), use_sqlite)?;
                        vault.unlock(passphrase)?;

                        let model_format = ModelFormat::from_extension(
                            std::path::Path::new(&remote_path)
                                .extension()
                                .and_then(|s| s.to_str())
                                .unwrap_or("bin"),
                        );
                        let metadata = ModelMetadata::new(model.clone(), model_format);
                        let version = vault.store_model(&model, data, metadata, None)?;

                        println!("\n✅ Model pulled and stored successfully!");
                        println!("   Model: {} v{}", model, version.version);
                    }
                    #[cfg(not(feature = "s3"))]
                    {
                        println!("⚠️  S3 support not enabled in this build");
                        println!("   To enable: cargo build --release --features s3");
                    }
                }
                "azure" => {
                    #[cfg(feature = "azure")]
                    {
                        use ironvault::storage::StorageConfig;
                        let account = std::env::var("AZURE_STORAGE_ACCOUNT").map_err(|_| {
                            VaultError::ConfigError(
                                "AZURE_STORAGE_ACCOUNT env var not set".to_string(),
                            )
                        })?;
                        println!("📥 Downloading from Azure Blob Storage...");

                        let storage_config = StorageConfig::Azure {
                            account,
                            container: bucket.clone(),
                            prefix: None,
                        };
                        let rt = tokio::runtime::Runtime::new().map_err(|e| {
                            VaultError::StorageError(format!(
                                "Failed to create async runtime: {}",
                                e
                            ))
                        })?;
                        let data = rt.block_on(async {
                            let backend = storage_config.create_backend().await?;
                            backend.download(&remote_path).await
                        })?;

                        // Store into vault
                        let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
                        let data = unseal_downloaded(data, &passphrase)?;
                        let mut vault = build_vault(config.clone(), use_sqlite)?;
                        vault.unlock(passphrase)?;

                        let model_format = ModelFormat::from_extension(
                            std::path::Path::new(&remote_path)
                                .extension()
                                .and_then(|s| s.to_str())
                                .unwrap_or("bin"),
                        );
                        let metadata = ModelMetadata::new(model.clone(), model_format);
                        let version = vault.store_model(&model, data, metadata, None)?;

                        println!("\n✅ Model pulled and stored successfully!");
                        println!("   Model: {} v{}", model, version.version);
                    }
                    #[cfg(not(feature = "azure"))]
                    {
                        println!("⚠️  Azure support not enabled in this build");
                        println!("   To enable: cargo build --release --features azure");
                    }
                }
                "gcs" => {
                    println!(
                        "⚠️  GCS support temporarily disabled due to security vulnerabilities"
                    );
                    println!("   Use S3 or Azure instead.");
                }
                _ => {
                    return Err(VaultError::InvalidInput(format!(
                        "Unsupported provider: {}. Use 's3', 'azure', or 'gcs'",
                        provider
                    )));
                }
            }
        }

        CloudCommands::List {
            provider,
            bucket,
            prefix,
        } => {
            println!("☁️  Listing cloud storage contents");
            println!("   Provider: {}", provider);
            println!("   Bucket: {}", bucket);
            if let Some(ref p) = prefix {
                println!("   Prefix: {}", p);
            }

            match provider.to_lowercase().as_str() {
                "s3" => {
                    #[cfg(feature = "s3")]
                    {
                        use ironvault::storage::StorageConfig;
                        let region =
                            std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());

                        let storage_config = StorageConfig::S3 {
                            bucket: bucket.clone(),
                            region,
                            prefix: prefix.clone(),
                        };
                        let rt = tokio::runtime::Runtime::new().map_err(|e| {
                            VaultError::StorageError(format!(
                                "Failed to create async runtime: {}",
                                e
                            ))
                        })?;
                        let keys = rt.block_on(async {
                            let backend = storage_config.create_backend().await?;
                            backend.list().await
                        })?;

                        println!(
                            "\n📋 S3 Bucket '{}' Contents ({} items):",
                            bucket,
                            keys.len()
                        );
                        for key in &keys {
                            println!("   {}", key);
                        }
                        if keys.is_empty() {
                            println!("   (empty)");
                        }
                    }
                    #[cfg(not(feature = "s3"))]
                    {
                        println!("⚠️  S3 support not enabled in this build");
                        println!("   To enable: cargo build --release --features s3");
                    }
                }
                "azure" => {
                    #[cfg(feature = "azure")]
                    {
                        use ironvault::storage::StorageConfig;
                        let account = std::env::var("AZURE_STORAGE_ACCOUNT").map_err(|_| {
                            VaultError::ConfigError(
                                "AZURE_STORAGE_ACCOUNT env var not set".to_string(),
                            )
                        })?;

                        let storage_config = StorageConfig::Azure {
                            account,
                            container: bucket.clone(),
                            prefix: prefix.clone(),
                        };
                        let rt = tokio::runtime::Runtime::new().map_err(|e| {
                            VaultError::StorageError(format!(
                                "Failed to create async runtime: {}",
                                e
                            ))
                        })?;
                        let keys = rt.block_on(async {
                            let backend = storage_config.create_backend().await?;
                            backend.list().await
                        })?;

                        println!(
                            "\n📋 Azure Container '{}' Contents ({} items):",
                            bucket,
                            keys.len()
                        );
                        for key in &keys {
                            println!("   {}", key);
                        }
                        if keys.is_empty() {
                            println!("   (empty)");
                        }
                    }
                    #[cfg(not(feature = "azure"))]
                    {
                        println!("⚠️  Azure support not enabled in this build");
                        println!("   To enable: cargo build --release --features azure");
                    }
                }
                "gcs" => {
                    println!(
                        "⚠️  GCS support temporarily disabled due to security vulnerabilities"
                    );
                    println!("   Use S3 or Azure instead.");
                }
                _ => {
                    return Err(VaultError::InvalidInput(format!(
                        "Unsupported provider: {}. Use 's3', 'azure', or 'gcs'",
                        provider
                    )));
                }
            }
        }

        CloudCommands::Config { provider, show } => {
            println!("☁️  Cloud Storage Configuration");
            println!("   Provider: {}", provider);

            if show {
                match provider.to_lowercase().as_str() {
                    "s3" => {
                        println!("\n📝 AWS S3 Configuration:");
                        println!("   Required environment variables:");
                        println!(
                            "   - AWS_ACCESS_KEY_ID: {}",
                            if std::env::var("AWS_ACCESS_KEY_ID").is_ok() {
                                "✅ Set"
                            } else {
                                "❌ Not set"
                            }
                        );
                        println!(
                            "   - AWS_SECRET_ACCESS_KEY: {}",
                            if std::env::var("AWS_SECRET_ACCESS_KEY").is_ok() {
                                "✅ Set"
                            } else {
                                "❌ Not set"
                            }
                        );
                        println!(
                            "   - AWS_REGION (optional): {}",
                            std::env::var("AWS_REGION")
                                .unwrap_or_else(|_| "Not set (defaults to us-east-1)".to_string())
                        );

                        println!("\n💡 To configure:");
                        println!("   export AWS_ACCESS_KEY_ID=your_access_key");
                        println!("   export AWS_SECRET_ACCESS_KEY=your_secret_key");
                        println!("   export AWS_REGION=us-east-1  # optional");
                    }
                    "azure" => {
                        // Keep this list in step with `AzureBackend::new`. It used to
                        // advertise AZURE_STORAGE_KEY, which that constructor rejects
                        // outright — the Azure SDK for Rust v1 has no shared-key
                        // credential — so following this output produced a hard error.
                        let is_set = |var: &str| {
                            if std::env::var(var).is_ok() {
                                "✅ Set"
                            } else {
                                "❌ Not set"
                            }
                        };

                        println!("\n📝 Azure Blob Storage Configuration:");
                        println!("   Storage account (always required):");
                        println!(
                            "   - AZURE_STORAGE_ACCOUNT: {}",
                            is_set("AZURE_STORAGE_ACCOUNT")
                        );

                        println!("\n   Credentials — a SAS token, or Entra ID:");
                        println!(
                            "   - AZURE_STORAGE_SAS_TOKEN: {}",
                            is_set("AZURE_STORAGE_SAS_TOKEN")
                        );
                        println!("   - AZURE_TENANT_ID:      {}", is_set("AZURE_TENANT_ID"));
                        println!("   - AZURE_CLIENT_ID:      {}", is_set("AZURE_CLIENT_ID"));
                        println!(
                            "   - AZURE_CLIENT_SECRET:  {}",
                            is_set("AZURE_CLIENT_SECRET")
                        );

                        println!("\n💡 To configure with a SAS token:");
                        println!("   export AZURE_STORAGE_ACCOUNT=your_account_name");
                        println!("   export AZURE_STORAGE_SAS_TOKEN=\"$(az storage container \\");
                        println!("       generate-sas --account-name your_account_name \\");
                        println!("       --name your_container --permissions rwdl \\");
                        println!("       --expiry 2030-01-01 --output tsv)\"");

                        println!("\n💡 Or with an Entra ID service principal:");
                        println!("   export AZURE_TENANT_ID=...");
                        println!("   export AZURE_CLIENT_ID=...");
                        println!("   export AZURE_CLIENT_SECRET=...");
                        println!(
                            "\n   Managed identity and `az login` are also picked up automatically."
                        );

                        if std::env::var("AZURE_STORAGE_KEY").is_ok()
                            && std::env::var("AZURE_STORAGE_SAS_TOKEN").is_err()
                        {
                            println!(
                                "\n⚠️  AZURE_STORAGE_KEY is set but is not supported — the Azure \
                                 SDK for Rust v1 has no shared-key credential."
                            );
                            println!("   Mint a SAS from that key, or use Entra ID.");
                        }
                    }
                    "gcs" => {
                        println!("\n📝 Google Cloud Storage Configuration:");
                        println!("   ⚠️  GCS support temporarily disabled due to security vulnerabilities");
                        println!("   Use S3 or Azure instead");
                        println!("\n   For details, see SECURITY_AUDIT.md");
                    }
                    _ => {
                        return Err(VaultError::InvalidInput(format!(
                            "Unsupported provider: {}. Use 's3', 'azure', or 'gcs'",
                            provider
                        )));
                    }
                }
            } else {
                println!("\n💡 Use --show flag to display current configuration");
                println!("   Example: iv cloud config --provider s3 --show");
            }
        }
    }

    Ok(())
}
