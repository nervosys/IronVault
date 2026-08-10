//! Error types for IronVault
//!
//! The top-level [`VaultError`] enum covers all failure modes.  Domain-specific
//! sub-error types ([`CryptoError`], [`StorageError`], [`ConversionError`])
//! carry richer context and convert into `VaultError` via `From`.

use std::io;
use thiserror::Error;

/// Result type alias for IronVault operations
pub type Result<T> = std::result::Result<T, VaultError>;

// ── Domain-specific error types ─────────────────────────────────────────────

/// Errors originating from cryptographic operations.
#[derive(Error, Debug)]
pub enum CryptoError {
    /// Key derivation failure (Argon2id / PBKDF2)
    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),

    /// Encryption failure
    #[error("Encryption failed: {0}")]
    Encryption(String),

    /// Decryption failure (wrong key, corrupted ciphertext, …)
    #[error("Decryption failed: {0}")]
    Decryption(String),

    /// Data integrity check mismatch (HMAC / SHA-256)
    #[error("Integrity check failed: {0}")]
    Integrity(String),

    /// Generic / uncategorised crypto error
    #[error("Cryptographic error: {0}")]
    Other(String),
}

/// Errors originating from storage and I/O operations.
#[derive(Error, Debug)]
pub enum StorageError {
    /// Underlying I/O error
    #[error(transparent)]
    Io(#[from] io::Error),

    /// Serialization / deserialization failure
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Compression / decompression failure
    #[error("Compression error: {0}")]
    Compression(String),

    /// Database backend error (SQLite, Sled, …)
    #[error("Database error: {0}")]
    Database(String),

    /// Generic storage error
    #[error("Storage error: {0}")]
    Other(String),
}

/// Errors originating from model format conversion.
#[derive(Error, Debug)]
pub enum ConversionError {
    /// Requested conversion path is not supported
    #[error("Unsupported conversion: {0}")]
    Unsupported(String),

    /// Validation of the converted output failed
    #[error("Validation failed: {0}")]
    Validation(String),

    /// Generic conversion error
    #[error("Conversion error: {0}")]
    Other(String),
}

// ── From impls: domain → VaultError ─────────────────────────────────────────

impl From<CryptoError> for VaultError {
    fn from(err: CryptoError) -> Self {
        match err {
            CryptoError::Integrity(msg) => VaultError::IntegrityError(msg),
            other => VaultError::CryptoError(other.to_string()),
        }
    }
}

impl From<StorageError> for VaultError {
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::Io(e) => VaultError::IoError(e),
            StorageError::Serialization(msg) => VaultError::SerializationError(msg),
            StorageError::Compression(msg) => VaultError::CompressionError(msg),
            StorageError::Database(msg) => VaultError::StorageError(msg),
            StorageError::Other(msg) => VaultError::StorageError(msg),
        }
    }
}

impl From<ConversionError> for VaultError {
    fn from(err: ConversionError) -> Self {
        match err {
            ConversionError::Unsupported(msg) => VaultError::UnsupportedFormat(msg),
            other => VaultError::ConversionError(other.to_string()),
        }
    }
}

// ── Top-level error ─────────────────────────────────────────────────────────

/// IronVault error types
///
/// Marked `#[non_exhaustive]` so that adding a category later is not a breaking
/// change for downstream matches. Inside this crate the enum is still
/// exhaustive, which is what forces [`VaultError::exit_code`] to assign every
/// new variant a code rather than letting it fall through a wildcard — the
/// omission that let the published exit-code tables drift from reality.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum VaultError {
    /// Cryptographic operation failed
    #[error("Cryptographic error: {0}")]
    CryptoError(String),

    /// Invalid passphrase or authentication failure
    #[error("Authentication failed: invalid passphrase or corrupted data")]
    AuthenticationFailed,

    /// Data integrity check failed
    #[error("Integrity check failed: {0}")]
    IntegrityError(String),

    /// Version control error
    #[error("Version control error: {0}")]
    VersionError(String),

    /// Model not found
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Version not found
    #[error("Version {0} not found for model {1}")]
    VersionNotFound(u32, String),

    /// A named resource that is not a model or a version does not exist —
    /// a profile, a registered vault, a backup schedule, a document.
    ///
    /// Carries a full message rather than a bare name so the caller can say
    /// which kind of thing was missing.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Format conversion error
    #[error("Format conversion error: {0}")]
    ConversionError(String),

    /// Unsupported model format
    #[error("Unsupported model format: {0}")]
    UnsupportedFormat(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] io::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Compression error
    #[error("Compression error: {0}")]
    CompressionError(String),

    /// Security policy violation
    #[error("Security policy violation: {0}")]
    SecurityViolation(String),

    /// Compliance violation
    #[error("Compliance violation: {0}")]
    ComplianceViolation(String),

    /// Audit log error
    #[error("Audit log error: {0}")]
    AuditError(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Storage/database error
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Process exit code for a successful run.
pub const EXIT_SUCCESS: u8 = 0;
/// An error that does not fall into any more specific category below.
pub const EXIT_GENERAL: u8 = 1;
/// Wrong passphrase, or ciphertext that failed its authentication tag.
pub const EXIT_AUTH: u8 = 2;
/// A named model, version, or resource does not exist.
pub const EXIT_NOT_FOUND: u8 = 3;
/// The OS refused access, or a security policy did.
pub const EXIT_PERMISSION: u8 = 4;
/// A checksum or signature did not match — corruption or tampering.
pub const EXIT_INTEGRITY: u8 = 5;
/// The caller supplied something this command cannot act on.
pub const EXIT_INVALID_INPUT: u8 = 6;
/// The configuration file is missing, malformed, or invalid.
pub const EXIT_CONFIG: u8 = 7;
/// A compliance policy check failed.
pub const EXIT_COMPLIANCE: u8 = 8;

impl VaultError {
    /// The process exit code this error maps to.
    ///
    /// This is a **stability contract**: agents and CI pipelines branch on
    /// these numbers, so the mapping from category to code must not change
    /// once published. It is mirrored in `README.md`, `AGENTS.md`,
    /// `docs/CLI.md`, `.well-known/agents.json`, and
    /// `.well-known/ontology.jsonld` — change all of them together, and only
    /// ever by assigning a *new* code to a category that had none.
    ///
    /// Codes 0–5 match the table those manifests already published. 6–8 are
    /// additions for categories that previously fell through to `1`.
    ///
    /// `IoError` is split: the kernel refusing access is a distinct,
    /// actionable outcome (fix the permissions) from a disk or network
    /// failure, so `PermissionDenied` earns [`EXIT_PERMISSION`] while every
    /// other I/O failure stays [`EXIT_GENERAL`].
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            VaultError::AuthenticationFailed => EXIT_AUTH,

            VaultError::ModelNotFound(_)
            | VaultError::VersionNotFound(_, _)
            | VaultError::NotFound(_) => EXIT_NOT_FOUND,

            VaultError::SecurityViolation(_) => EXIT_PERMISSION,
            VaultError::IoError(err) if err.kind() == io::ErrorKind::PermissionDenied => {
                EXIT_PERMISSION
            }

            VaultError::IntegrityError(_) => EXIT_INTEGRITY,

            VaultError::InvalidInput(_) | VaultError::UnsupportedFormat(_) => EXIT_INVALID_INPUT,

            VaultError::ConfigError(_) => EXIT_CONFIG,

            VaultError::ComplianceViolation(_) => EXIT_COMPLIANCE,

            // Everything else is a genuine failure with no more specific
            // handling an agent could apply. Listed exhaustively rather than
            // with a wildcard so that adding a variant forces a decision here.
            VaultError::CryptoError(_)
            | VaultError::VersionError(_)
            | VaultError::ConversionError(_)
            | VaultError::IoError(_)
            | VaultError::SerializationError(_)
            | VaultError::CompressionError(_)
            | VaultError::AuditError(_)
            | VaultError::StorageError(_) => EXIT_GENERAL,
        }
    }

    /// Stable, constant name for the error variant.
    ///
    /// This exists so telemetry can report *which kind* of failure occurred
    /// without touching the message. Every variant that carries a `String`
    /// carries an interpolated one — model names, filesystem paths, and in
    /// `ConfigError`'s case the path of a config file under the user's home
    /// directory. `Display` output is therefore unsafe to report, and this
    /// returns a fixed literal per variant instead: the set of possible values
    /// is exactly the list below, and nothing else can ever appear.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            VaultError::CryptoError(_) => "crypto",
            VaultError::AuthenticationFailed => "authentication_failed",
            VaultError::IntegrityError(_) => "integrity",
            VaultError::VersionError(_) => "version",
            VaultError::ModelNotFound(_) => "model_not_found",
            VaultError::VersionNotFound(_, _) => "version_not_found",
            VaultError::NotFound(_) => "not_found",
            VaultError::ConversionError(_) => "conversion",
            VaultError::UnsupportedFormat(_) => "unsupported_format",
            VaultError::IoError(_) => "io",
            VaultError::ConfigError(_) => "config",
            VaultError::SerializationError(_) => "serialization",
            VaultError::CompressionError(_) => "compression",
            VaultError::SecurityViolation(_) => "security_violation",
            VaultError::InvalidInput(_) => "invalid_input",
            VaultError::ComplianceViolation(_) => "compliance_violation",
            VaultError::AuditError(_) => "audit",
            VaultError::StorageError(_) => "storage",
        }
    }
}

impl From<serde_json::Error> for VaultError {
    fn from(err: serde_json::Error) -> Self {
        VaultError::SerializationError(err.to_string())
    }
}

impl From<serde_yaml_ng::Error> for VaultError {
    fn from(err: serde_yaml_ng::Error) -> Self {
        VaultError::SerializationError(err.to_string())
    }
}

impl From<zip::result::ZipError> for VaultError {
    fn from(err: zip::result::ZipError) -> Self {
        VaultError::IoError(io::Error::other(err.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zip_error_conversion() {
        // Covers lines 94, 95 — From<ZipError>
        let zip_err = zip::result::ZipError::FileNotFound;
        let vault_err: VaultError = zip_err.into();
        match vault_err {
            VaultError::IoError(_) => {} // expected
            _ => panic!("Expected IoError"),
        }
    }

    #[test]
    fn test_serde_yml_error_conversion() {
        let yaml_err = serde_yaml_ng::from_str::<serde_yaml_ng::Value>("\t").unwrap_err();
        let vault_err: VaultError = yaml_err.into();
        match vault_err {
            VaultError::SerializationError(_) => {}
            _ => panic!("Expected SerializationError"),
        }
    }

    #[test]
    fn test_serde_json_error_conversion() {
        // Covers L81-83 — From<serde_json::Error>
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let vault_err: VaultError = json_err.into();
        match vault_err {
            VaultError::SerializationError(msg) => {
                assert!(!msg.is_empty());
            }
            _ => panic!("Expected SerializationError"),
        }
    }

    #[test]
    fn test_error_display_messages() {
        // Covers L7 (type alias used implicitly) + various Display branches
        let errors: Vec<VaultError> = vec![
            VaultError::CryptoError("crypto fail".into()),
            VaultError::AuthenticationFailed,
            VaultError::IntegrityError("integrity fail".into()),
            VaultError::VersionError("version fail".into()),
            VaultError::ModelNotFound("model1".into()),
            VaultError::VersionNotFound(3, "model1".into()),
            VaultError::ConversionError("conv fail".into()),
            VaultError::UnsupportedFormat("xyz".into()),
            VaultError::ConfigError("config fail".into()),
            VaultError::SerializationError("serde fail".into()),
            VaultError::CompressionError("comp fail".into()),
            VaultError::SecurityViolation("sec fail".into()),
            VaultError::ComplianceViolation("cc fail".into()),
            VaultError::AuditError("audit fail".into()),
            VaultError::InvalidInput("bad input".into()),
            VaultError::StorageError("store fail".into()),
        ];

        let expected_substrings = [
            "crypto fail",
            "invalid passphrase",
            "integrity fail",
            "version fail",
            "model1",
            "Version 3 not found for model model1",
            "conv fail",
            "xyz",
            "config fail",
            "serde fail",
            "comp fail",
            "sec fail",
            "cc fail",
            "audit fail",
            "bad input",
            "store fail",
        ];

        for (err, expected) in errors.iter().zip(expected_substrings.iter()) {
            let msg = format!("{}", err);
            assert!(
                msg.contains(expected),
                "Error '{}' should contain '{}'",
                msg,
                expected
            );
        }
    }

    #[test]
    fn test_result_type_alias() {
        // Covers L7 — the Result<T> type alias usage
        let ok_result: super::Result<i32> = Ok(42);
        assert_eq!(ok_result.ok(), Some(42));

        let err_result: super::Result<i32> = Err(VaultError::CryptoError("test".into()));
        assert!(err_result.is_err());
    }

    #[test]
    fn test_io_error_conversion() {
        // Covers L87 — From<io::Error>
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let vault_err: VaultError = io_err.into();
        match vault_err {
            VaultError::IoError(e) => {
                assert!(e.to_string().contains("file not found"));
            }
            _ => panic!("Expected IoError"),
        }
    }

    #[test]
    fn test_error_debug_format() {
        // L93 — ensure Debug trait works (thiserror derives it)
        let err = VaultError::StorageError("db fail".into());
        let debug = format!("{:?}", err);
        assert!(debug.contains("StorageError"));
    }

    #[test]
    fn test_domain_error_display() {
        // CryptoError Display
        assert!(CryptoError::KeyDerivation("kdf".into())
            .to_string()
            .contains("kdf"));
        assert!(CryptoError::Encryption("enc".into())
            .to_string()
            .contains("enc"));
        assert!(CryptoError::Decryption("dec".into())
            .to_string()
            .contains("dec"));
        assert!(CryptoError::Integrity("int".into())
            .to_string()
            .contains("int"));
        assert!(CryptoError::Other("oth".into()).to_string().contains("oth"));

        // StorageError Display
        let io = StorageError::Io(std::io::Error::other("io"));
        assert!(io.to_string().contains("io"));
        assert!(StorageError::Serialization("ser".into())
            .to_string()
            .contains("ser"));
        assert!(StorageError::Compression("comp".into())
            .to_string()
            .contains("comp"));
        assert!(StorageError::Database("db".into())
            .to_string()
            .contains("db"));
        assert!(StorageError::Other("oth".into())
            .to_string()
            .contains("oth"));

        // ConversionError Display
        assert!(ConversionError::Unsupported("uns".into())
            .to_string()
            .contains("uns"));
        assert!(ConversionError::Validation("val".into())
            .to_string()
            .contains("val"));
        assert!(ConversionError::Other("oth".into())
            .to_string()
            .contains("oth"));
    }

    // ── Domain-specific error conversion tests ──────────────────────────

    #[test]
    fn test_crypto_error_into_vault_error() {
        let cases: Vec<(CryptoError, &str)> = vec![
            (CryptoError::KeyDerivation("bad salt".into()), "bad salt"),
            (CryptoError::Encryption("aes fail".into()), "aes fail"),
            (CryptoError::Decryption("wrong key".into()), "wrong key"),
            (CryptoError::Other("misc".into()), "misc"),
        ];
        for (crypto_err, expected) in cases {
            let vault_err: VaultError = crypto_err.into();
            match &vault_err {
                VaultError::CryptoError(msg) => assert!(msg.contains(expected)),
                _ => panic!("Expected CryptoError, got {:?}", vault_err),
            }
        }

        // Integrity maps to IntegrityError
        let integrity = CryptoError::Integrity("hash mismatch".into());
        let vault_err: VaultError = integrity.into();
        assert!(matches!(vault_err, VaultError::IntegrityError(_)));
    }

    #[test]
    fn test_storage_error_into_vault_error() {
        let io_err = StorageError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
        assert!(matches!(VaultError::from(io_err), VaultError::IoError(_)));

        let ser = StorageError::Serialization("bad json".into());
        assert!(matches!(
            VaultError::from(ser),
            VaultError::SerializationError(_)
        ));

        let comp = StorageError::Compression("zlib".into());
        assert!(matches!(
            VaultError::from(comp),
            VaultError::CompressionError(_)
        ));

        let db = StorageError::Database("sqlite locked".into());
        assert!(matches!(VaultError::from(db), VaultError::StorageError(_)));

        let other = StorageError::Other("unknown".into());
        assert!(matches!(
            VaultError::from(other),
            VaultError::StorageError(_)
        ));
    }

    #[test]
    fn test_conversion_error_into_vault_error() {
        let unsup = ConversionError::Unsupported("onnx→gguf".into());
        assert!(matches!(
            VaultError::from(unsup),
            VaultError::UnsupportedFormat(_)
        ));

        let val = ConversionError::Validation("shape mismatch".into());
        assert!(matches!(
            VaultError::from(val),
            VaultError::ConversionError(_)
        ));

        let other = ConversionError::Other("misc".into());
        assert!(matches!(
            VaultError::from(other),
            VaultError::ConversionError(_)
        ));
    }

    // ── Exit codes ──────────────────────────────────────────────────────────

    /// Pins the published contract. If this test needs editing to pass, the
    /// change is breaking for every agent and CI pipeline that branches on
    /// these numbers — update the manifests in the same commit, or don't.
    #[test]
    fn test_exit_codes_match_the_published_contract() {
        let cases: &[(VaultError, u8)] = &[
            (VaultError::AuthenticationFailed, 2),
            (VaultError::ModelNotFound("m".into()), 3),
            (VaultError::VersionNotFound(2, "m".into()), 3),
            (VaultError::NotFound("profile 'p'".into()), 3),
            (VaultError::SecurityViolation("policy".into()), 4),
            (VaultError::IntegrityError("checksum".into()), 5),
            (VaultError::InvalidInput("bad".into()), 6),
            (VaultError::UnsupportedFormat("xyz".into()), 6),
            (VaultError::ConfigError("malformed".into()), 7),
            (VaultError::ComplianceViolation("weak".into()), 8),
            (VaultError::CryptoError("aead".into()), 1),
            (VaultError::VersionError("index".into()), 1),
            (VaultError::ConversionError("path".into()), 1),
            (VaultError::SerializationError("json".into()), 1),
            (VaultError::CompressionError("zstd".into()), 1),
            (VaultError::AuditError("log".into()), 1),
            (VaultError::StorageError("db".into()), 1),
        ];

        for (err, expected) in cases {
            assert_eq!(
                err.exit_code(),
                *expected,
                "{err:?} must exit {expected} — this mapping is a published contract"
            );
        }
    }

    #[test]
    fn test_permission_denied_io_error_is_distinguished_from_other_io_failures() {
        // The kernel refusing access is actionable (fix the permissions);
        // a disk or network failure is not, so they must not share a code.
        let denied = VaultError::IoError(io::Error::from(io::ErrorKind::PermissionDenied));
        assert_eq!(denied.exit_code(), EXIT_PERMISSION);

        let not_found = VaultError::IoError(io::Error::from(io::ErrorKind::NotFound));
        assert_eq!(not_found.exit_code(), EXIT_GENERAL);

        let broken = VaultError::IoError(io::Error::other("disk on fire"));
        assert_eq!(broken.exit_code(), EXIT_GENERAL);
    }

    #[test]
    fn test_success_is_the_only_zero_code() {
        // A failure that exits 0 is the bug class this mapping exists to
        // prevent, so assert no error can ever produce one.
        let all = [
            VaultError::AuthenticationFailed,
            VaultError::ModelNotFound("m".into()),
            VaultError::VersionNotFound(1, "m".into()),
            VaultError::NotFound("thing".into()),
            VaultError::SecurityViolation("p".into()),
            VaultError::IntegrityError("c".into()),
            VaultError::InvalidInput("i".into()),
            VaultError::UnsupportedFormat("f".into()),
            VaultError::ConfigError("c".into()),
            VaultError::ComplianceViolation("v".into()),
            VaultError::CryptoError("c".into()),
            VaultError::VersionError("v".into()),
            VaultError::ConversionError("c".into()),
            VaultError::IoError(io::Error::other("io")),
            VaultError::SerializationError("s".into()),
            VaultError::CompressionError("c".into()),
            VaultError::AuditError("a".into()),
            VaultError::StorageError("s".into()),
        ];
        for err in &all {
            assert_ne!(err.exit_code(), EXIT_SUCCESS, "{err:?} must not exit 0");
        }
        assert_eq!(EXIT_SUCCESS, 0);
    }
}
