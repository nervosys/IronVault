use ironvault::formats::{ModelFormat, ModelMetadata};
use ironvault::model_card::*;
use ironvault::{Result, Vault, VaultConfig};
use tempfile::tempdir;

#[test]
fn test_attach_and_extract_model_card() -> Result<()> {
    let temp_dir = tempdir()?;
    let mut config = VaultConfig::new()?;
    config.dirs.vault_dir = temp_dir.path().to_path_buf();

    // Create and unlock vault
    let mut vault = Vault::new(Some(config))?;
    let passphrase = b"test-passphrase".to_vec();
    vault.unlock(passphrase.clone())?;

    // Store a model
    let model_data = b"test model data".to_vec();
    let metadata = ModelMetadata::new("test-model".to_string(), ModelFormat::PyTorch)
        .with_description("Test model".to_string());

    vault.store_model("test-model", model_data, metadata, None)?;

    // Create a model card
    let details = ModelDetails {
        name: "test-model".to_string(),
        version: "1.0.0".to_string(),
        description: "Test model for integration".to_string(),
        model_type: "Test".to_string(),
        architecture: "Simple".to_string(),
        size: "1KB".to_string(),
        framework: "PyTorch".to_string(),
        format: "pytorch".to_string(),
        license: Some("MIT".to_string()),
        citation: None,
        developers: vec!["Test Team".to_string()],
        contact: Some("test@example.com".to_string()),
        repository: None,
        paper: None,
    };

    let intended_use = IntendedUse {
        primary_uses: vec!["Testing".to_string()],
        primary_users: vec!["Developers".to_string()],
        out_of_scope_uses: vec!["Production".to_string()],
        use_case_examples: None,
    };

    let card = ModelCard::new(details, intended_use);
    let card_json = card.to_json()?;

    // Attach card to model
    vault.update_version_metadata("test-model", 1, "model_card", card_json.clone())?;

    // Extract card from model
    let extracted_json = vault
        .get_version_metadata("test-model", 1, "model_card")
        .expect("Model card should exist");

    assert_eq!(card_json, extracted_json);

    // Parse and verify
    let extracted_card = ModelCard::from_json(&extracted_json)?;
    assert_eq!(extracted_card.model_details.name, "test-model");
    assert_eq!(extracted_card.model_details.version, "1.0.0");

    Ok(())
}

#[test]
fn test_generate_card_from_metadata() -> Result<()> {
    let temp_dir = tempdir()?;
    let mut config = VaultConfig::new()?;
    config.dirs.vault_dir = temp_dir.path().to_path_buf();

    // Create and unlock vault
    let mut vault = Vault::new(Some(config))?;
    let passphrase = b"test-passphrase".to_vec();
    vault.unlock(passphrase)?;

    // Store a model with metadata
    let model_data = b"test model data".to_vec();
    let metadata = ModelMetadata::new("ml-model".to_string(), ModelFormat::ONNX)
        .with_description("Machine learning model".to_string())
        .with_framework("PyTorch".to_string())
        .with_task("Classification".to_string())
        .add_custom_field("architecture".to_string(), "ResNet-50".to_string())
        .add_custom_field("license".to_string(), "Apache-2.0".to_string());

    vault.store_model("ml-model", model_data, metadata, None)?;

    // Get version info
    let versions = vault.list_versions("ml-model");
    assert_eq!(versions.len(), 1);

    let version = &versions[0];
    assert_eq!(version.version, 1);
    assert_eq!(version.format, "ONNX");
    assert!(version.metadata.contains_key("description"));
    assert_eq!(
        version.metadata.get("description").unwrap(),
        "Machine learning model"
    );
    assert_eq!(version.metadata.get("framework").unwrap(), "PyTorch");
    assert_eq!(version.metadata.get("task").unwrap(), "Classification");

    Ok(())
}

#[test]
fn test_update_and_get_metadata() -> Result<()> {
    let temp_dir = tempdir()?;
    let mut config = VaultConfig::new()?;
    config.dirs.vault_dir = temp_dir.path().to_path_buf();

    // Create and unlock vault
    let mut vault = Vault::new(Some(config))?;
    let passphrase = b"test-passphrase".to_vec();
    vault.unlock(passphrase)?;

    // Store a model
    let model_data = b"test model data".to_vec();
    let metadata = ModelMetadata::new("meta-model".to_string(), ModelFormat::TensorFlow);

    vault.store_model("meta-model", model_data, metadata, None)?;

    // Update metadata
    vault.update_version_metadata("meta-model", 1, "custom_field", "custom_value".to_string())?;

    // Get metadata
    let value = vault
        .get_version_metadata("meta-model", 1, "custom_field")
        .expect("Custom field should exist");

    assert_eq!(value, "custom_value");

    // Try to get non-existent metadata
    let missing = vault.get_version_metadata("meta-model", 1, "nonexistent");
    assert!(missing.is_none());

    Ok(())
}

#[test]
fn test_card_attach_to_nonexistent_model() -> Result<()> {
    let temp_dir = tempdir()?;
    let mut config = VaultConfig::new()?;
    config.dirs.vault_dir = temp_dir.path().to_path_buf();

    let mut vault = Vault::new(Some(config))?;

    // Try to attach card to nonexistent model
    let result = vault.update_version_metadata("nonexistent", 1, "model_card", "{}".to_string());

    assert!(result.is_err());

    Ok(())
}
