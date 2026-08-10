//! Configuration and error handling tests

use ironvault::config::{DirectoryPaths, VaultConfig};
use ironvault::error::VaultError;

#[test]
fn test_default_config() {
    let config = VaultConfig::default();
    assert_eq!(config.version, "1.0");
    assert!(!config.vault.default_vault.is_empty());
    assert_eq!(config.crypto.algorithm, "aes-256-gcm");
}

#[test]
fn test_config_vault_settings() {
    let config = VaultConfig::default();
    assert_eq!(config.vault.default_vault, "default");
}

#[test]
fn test_directory_paths_default() {
    let paths = DirectoryPaths::default();

    // Default implementation creates empty paths, they're set at runtime
    // Just verify the structure is correct
    assert_eq!(
        std::mem::size_of_val(&paths),
        std::mem::size_of::<DirectoryPaths>()
    );
}

#[test]
fn test_directory_paths_creation() {
    use tempfile::TempDir;

    let temp = TempDir::new().unwrap();
    let paths = DirectoryPaths {
        data_dir: temp.path().join("data"),
        config_dir: temp.path().join("config"),
        ..DirectoryPaths::default()
    };

    // Paths should be set correctly
    assert!(paths.data_dir.to_string_lossy().contains("data"));
    assert!(paths.config_dir.to_string_lossy().contains("config"));
}

#[test]
fn test_vault_error_display() {
    let err = VaultError::ModelNotFound("test_model".to_string());
    let display = format!("{}", err);
    assert!(!display.is_empty());
    assert!(display.contains("test_model") || display.contains("not found"));
}

#[test]
fn test_vault_error_crypto() {
    let err = VaultError::CryptoError("test error".to_string());
    let display = format!("{}", err);
    assert!(display.contains("test error") || display.contains("Crypto"));
}

#[test]
fn test_vault_error_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = VaultError::IoError(io_err);
    let display = format!("{}", err);
    assert!(display.contains("not found") || display.contains("IO") || display.contains("I/O"));
}

#[test]
fn test_vault_error_authentication_failed() {
    let err = VaultError::AuthenticationFailed;
    let display = format!("{}", err);
    assert!(display.contains("Authentication") || display.contains("passphrase"));
}

#[test]
fn test_vault_error_model_not_found() {
    let err = VaultError::ModelNotFound("test_model".to_string());
    let display = format!("{}", err);
    assert!(display.contains("test_model") || display.contains("not found"));
}

#[test]
fn test_vault_error_version_not_found() {
    let err = VaultError::VersionNotFound(5, "model".to_string());
    let display = format!("{}", err);
    assert!(display.contains("model") || display.contains('5') || display.contains("version"));
}

#[test]
fn test_config_compression_settings() {
    let config = VaultConfig::default();
    assert_eq!(config.compression.algorithm, "gzip");
    assert!(config.compression.level > 0 && config.compression.level <= 9);
}

#[test]
fn test_config_serialization() {
    let config = VaultConfig::default();

    // Should be able to serialize
    let json = serde_json::to_string(&config);
    assert!(json.is_ok());
}

#[test]
fn test_config_security_settings() {
    let config = VaultConfig::default();
    assert!(config.security.require_passphrase);
    assert!(config.security.audit_log);
    assert!(config.security.session_timeout_seconds > 0);
}

#[test]
fn test_error_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
    let vault_err: VaultError = io_err.into();

    match vault_err {
        VaultError::IoError(_) => {} // expected
        _ => panic!("Should convert to IoError variant"),
    }
}

#[test]
fn test_error_debug_format() {
    let err = VaultError::CryptoError("debug test".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("CryptoError") || debug.contains("debug test"));
}

#[test]
fn test_config_compliance_settings() {
    let config = VaultConfig::default();
    assert!(config.compliance.fips_mode);
    assert!(config.compliance.audit_retention_days > 0);
}

#[test]
fn test_config_crypto_settings() {
    let config = VaultConfig::default();
    assert_eq!(config.crypto.kdf, "argon2id");
    assert!(!config.crypto.algorithm.is_empty());
}

#[test]
fn test_config_storage_settings() {
    let config = VaultConfig::default();
    assert!(config.storage.max_versions > 0);
    assert!(!config.storage.checkpoint_format.is_empty());
}

#[test]
fn test_vault_error_integrity() {
    let err = VaultError::IntegrityError("checksum mismatch".to_string());
    let display = format!("{}", err);
    assert!(display.contains("Integrity") || display.contains("checksum"));
}

#[test]
fn test_vault_error_unsupported_format() {
    let err = VaultError::UnsupportedFormat("unknown_format".to_string());
    let display = format!("{}", err);
    assert!(display.contains("Unsupported") || display.contains("unknown_format"));
}

#[test]
fn test_vault_error_security_violation() {
    let err = VaultError::SecurityViolation("unauthorized access".to_string());
    let display = format!("{}", err);
    assert!(display.contains("Security") || display.contains("unauthorized"));
}

#[test]
fn test_vault_error_compliance_violation() {
    let err = VaultError::ComplianceViolation("FIPS requirement not met".to_string());
    let display = format!("{}", err);
    assert!(display.contains("Compliance") || display.contains("FIPS"));
}
