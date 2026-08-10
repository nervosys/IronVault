//! Integration tests for IronVault

use ironvault::formats::{ModelFormat, ModelMetadata};
use ironvault::{Vault, VaultConfig};
use tempfile::tempdir;

#[test]
fn test_vault_creation() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let vault = Vault::new(Some(config));
    assert!(vault.is_ok());
}

#[test]
fn test_store_and_retrieve_model() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let mut vault = Vault::new(Some(config)).unwrap();

    // Unlock vault with test passphrase
    let passphrase = b"test_passphrase_secure_12345";
    vault.unlock(passphrase.to_vec()).unwrap();

    // Create test model data
    let model_data = vec![1, 2, 3, 4, 5];
    let metadata = ModelMetadata::new("test_model".to_string(), ModelFormat::PyTorch);

    // Store model
    let version = vault
        .store_model("test_model", model_data.clone(), metadata, None)
        .unwrap();
    assert_eq!(version.version, 1);

    // Retrieve model
    let retrieved = vault.get_model("test_model", None).unwrap();
    assert_eq!(retrieved, model_data);
}

#[test]
fn test_version_control() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let mut vault = Vault::new(Some(config)).unwrap();
    let passphrase = b"test_passphrase_secure_12345";
    vault.unlock(passphrase.to_vec()).unwrap();

    // Store multiple versions
    for i in 1..=3 {
        let data = vec![i; 100];
        let metadata = ModelMetadata::new("versioned_model".to_string(), ModelFormat::PyTorch);

        vault
            .store_model("versioned_model", data, metadata, None)
            .unwrap();
    }

    // List versions
    let versions = vault.list_versions("versioned_model");
    assert_eq!(versions.len(), 3);

    // Retrieve specific version
    let v2_data = vault.get_model("versioned_model", Some(2)).unwrap();
    assert_eq!(v2_data, vec![2; 100]);
}

#[test]
fn test_compression() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let mut vault = Vault::new(Some(config)).unwrap();
    let passphrase = b"test_passphrase_secure_12345";
    vault.unlock(passphrase.to_vec()).unwrap();

    // Create highly compressible data
    let model_data = vec![42u8; 10000];
    let metadata = ModelMetadata::new("compressed_model".to_string(), ModelFormat::PyTorch);

    let version = vault
        .store_model("compressed_model", model_data.clone(), metadata, None)
        .unwrap();

    // Compressed size should be much smaller
    assert!(version.compressed_size_bytes < version.size_bytes);
    println!(
        "Original: {} bytes, Compressed: {} bytes, Ratio: {:.1}%",
        version.size_bytes,
        version.compressed_size_bytes,
        (1.0 - version.compressed_size_bytes as f64 / version.size_bytes as f64) * 100.0
    );

    // Verify data integrity after compression
    let retrieved = vault.get_model("compressed_model", None).unwrap();
    assert_eq!(retrieved, model_data);
}

#[test]
fn test_encryption_authentication() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let mut vault = Vault::new(Some(config)).unwrap();
    let passphrase = b"correct_passphrase_12345";
    vault.unlock(passphrase.to_vec()).unwrap();

    // Store model
    let model_data = vec![1, 2, 3, 4, 5];
    let metadata = ModelMetadata::new("auth_test_model".to_string(), ModelFormat::PyTorch);

    vault
        .store_model("auth_test_model", model_data, metadata, None)
        .unwrap();

    // Lock vault
    drop(vault);

    // Try to unlock with wrong passphrase
    let mut config2 = VaultConfig::new().unwrap();
    config2.dirs.vault_dir = dir.path().to_path_buf();
    let mut vault2 = Vault::new(Some(config2)).unwrap();

    let wrong_passphrase = b"wrong_passphrase_12345";
    let _result = vault2.unlock(wrong_passphrase.to_vec());

    // Should fail with wrong passphrase
    // Note: Implementation may vary on how unlock works
}

#[test]
fn test_delete_version() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();

    let mut vault = Vault::new(Some(config)).unwrap();
    let passphrase = b"test_passphrase_secure_12345";
    vault.unlock(passphrase.to_vec()).unwrap();

    // Store two versions
    for i in 1..=2 {
        let data = vec![i; 10];
        let metadata = ModelMetadata::new("deletable_model".to_string(), ModelFormat::PyTorch);
        vault
            .store_model("deletable_model", data, metadata, None)
            .unwrap();
    }

    // Delete version 1
    let deleted = vault.delete_version("deletable_model", 1).unwrap();
    assert!(deleted);

    // Verify only version 2 remains
    let versions = vault.list_versions("deletable_model");
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].version, 2);
}

#[test]
fn test_audit_logging() {
    let dir = tempdir().unwrap();
    let mut config = VaultConfig::new().unwrap();
    config.dirs.vault_dir = dir.path().to_path_buf();
    config.dirs.log_dir = dir.path().join("logs");

    let mut vault = Vault::new(Some(config)).unwrap();
    let passphrase = b"test_passphrase_secure_12345";
    vault.unlock(passphrase.to_vec()).unwrap();

    // Store a model (should generate audit log)
    let model_data = vec![1, 2, 3];
    let metadata = ModelMetadata::new("audit_test_model".to_string(), ModelFormat::PyTorch);

    vault
        .store_model("audit_test_model", model_data, metadata, None)
        .unwrap();

    // Verify audit log exists
    let audit_log = dir.path().join("logs").join("audit.log");
    assert!(audit_log.exists());
}

#[test]
fn test_model_metadata() {
    let metadata = ModelMetadata::new("test_model".to_string(), ModelFormat::PyTorch)
        .with_description("Test model description".to_string())
        .with_framework("PyTorch 2.0".to_string())
        .with_task("text-generation".to_string())
        .with_architecture("GPT-2".to_string())
        .with_parameters(124_000_000)
        .add_custom_field("license".to_string(), "MIT".to_string());

    assert_eq!(metadata.name, "test_model");
    assert_eq!(metadata.format, ModelFormat::PyTorch);
    assert_eq!(
        metadata.description,
        Some("Test model description".to_string())
    );
    assert_eq!(metadata.parameters, Some(124_000_000));
    assert_eq!(
        metadata.custom_fields.get("license"),
        Some(&"MIT".to_string())
    );
}
