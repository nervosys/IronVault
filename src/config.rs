//! XDG Base Directory Specification compliant configuration
//!
//! Cross-platform support for Linux, macOS, and Windows following XDG standards.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::crypto::compression::{CompressionAlgorithm, CompressionLevel};
use crate::error::{Result, VaultError};

/// IronVault (AIMV) Configuration
///
/// Directory structure:
/// - Config: ~/.config/ai/models/ (or platform equivalent)
/// - Data: ~/.local/share/ai/models/ (or platform equivalent)
/// - Cache: ~/.cache/ai/models/ (or platform equivalent)
/// - Backends: ~/.config/ai/backends/ (cloud storage configs)
/// - Utilities: ~/.config/ai/utilities/ (utility configs)
/// - Databases: ~/.config/ai/databases/ (knowledge bases, training data)
///
/// Compliance:
/// - XDG Base Directory Specification
/// - CMMC AC.3.014: Separate duties of individuals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    /// Version of configuration format
    pub version: String,

    /// Vault settings
    pub vault: VaultSettings,

    /// Cryptographic settings
    pub crypto: CryptoSettings,

    /// Compression settings
    pub compression: CompressionSettings,

    /// Storage settings
    pub storage: StorageSettings,

    /// Security settings
    pub security: SecuritySettings,

    /// Compliance settings
    pub compliance: ComplianceSettings,

    /// Telemetry settings
    #[serde(default)]
    pub telemetry: TelemetrySettings,

    /// Federation settings
    #[serde(default)]
    pub federation: FederationSettings,

    /// Directory paths (not serialized, computed at runtime)
    #[serde(skip)]
    pub dirs: DirectoryPaths,
}

/// Default vault selection settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultSettings {
    pub default_vault: String,
}

/// Cryptographic algorithm and key derivation settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoSettings {
    pub algorithm: String,
    pub kdf: String,
}

/// Compression algorithm and level settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionSettings {
    pub algorithm: String,
    pub level: u8,
}

/// Storage backend behavior settings (versioning, cleanup).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSettings {
    pub max_versions: u32,
    pub auto_cleanup: bool,
    pub checkpoint_format: String,
    /// Models larger than this threshold (in bytes) use chunked streaming
    /// encryption instead of monolithic encryption. Default: 16 MiB.
    /// Set to 0 to always use streaming, or `u64::MAX` to disable.
    #[serde(default = "default_streaming_threshold")]
    pub streaming_threshold: u64,
}

/// Default streaming threshold: 16 MiB.
fn default_streaming_threshold() -> u64 {
    16 * 1024 * 1024
}

/// Security policy settings (passphrase, session timeout, audit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    pub require_passphrase: bool,
    pub session_timeout_seconds: u64,
    pub audit_log: bool,

    /// Mirror every audit entry into a hash-linked blockchain (default: off).
    ///
    /// Opt-in because it changes on-disk behaviour for an existing vault: the
    /// chain is append-only and never pruned, so it grows without bound while
    /// `audit_log` alone rotates at a size cap. Requires `audit_log` — the
    /// chain is fed from the audit logger, so with the log off there is
    /// nothing to mirror.
    #[serde(default)]
    pub blockchain_audit: bool,

    /// Audit entries per block (default: 1).
    ///
    /// One entry per block is deliberate. `BlockchainAudit` holds pending
    /// entries in memory and only writes them to disk on finalize, so any
    /// value above 1 means a process that exits before the threshold silently
    /// drops the entries it was asked to make tamper-evident. A larger value
    /// trades that durability for fewer, denser block files; the logger
    /// finalizes on drop to narrow the window, but a crash still loses
    /// whatever is pending.
    #[serde(default = "default_blockchain_block_size")]
    pub blockchain_block_size: usize,
}

/// Default entries per audit block: 1 — see [`SecuritySettings::blockchain_block_size`].
fn default_blockchain_block_size() -> usize {
    1
}

/// Compliance and regulatory settings (CVE scanning, audit retention).
///
/// `fips_mode` was removed in 7.0. It defaulted to true, was documented as
/// "Enforce FIPS-validated algorithms only", and was read by nothing --
/// there is no FIPS mode to enter, because the KDF is Argon2id either way
/// and the implementations hold no CMVP certificate. A switch that appears
/// enabled and enforces nothing is worse than no switch. Old config files
/// still load: serde ignores the unknown key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceSettings {
    pub cve_scanning: bool,
    pub audit_retention_days: u32,
}

/// Federation settings — syncing models between `iv` nodes.
///
/// Off by default. Enabling it exposes `/api/v1/federation/*` on `iv serve`,
/// which hands model bytes to any caller presenting an accepted key, so it is
/// a deliberate act rather than something an upgrade turns on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationSettings {
    /// Serve the federation endpoints (default: false).
    #[serde(default)]
    pub enabled: bool,

    /// This node's stable ID. Generated on first use if empty.
    #[serde(default)]
    pub node_id: String,

    /// Human-readable name for this node.
    #[serde(default)]
    pub node_name: String,

    /// Encrypt model bytes in transit with the `AIMVSEAL` envelope
    /// (default: true).
    ///
    /// TLS protects the hop; this protects the object, so a peer's reverse
    /// proxy, request log, or on-disk cache never holds a readable model. It
    /// requires both nodes to share `$IRONVAULT_FEDERATION_PASSPHRASE`.
    /// Turning it off is only defensible on a network you fully control.
    #[serde(default = "default_true")]
    pub seal_transfers: bool,

    /// Peers this node may sync with.
    #[serde(default)]
    pub peers: Vec<FederationPeerSettings>,
}

impl Default for FederationSettings {
    /// Hand-written rather than derived: `#[derive(Default)]` would make
    /// `seal_transfers` false, quietly shipping models in the clear while the
    /// serde default says true. A security default must not depend on which
    /// path constructed the struct.
    fn default() -> Self {
        Self {
            enabled: false,
            node_id: String::new(),
            node_name: String::new(),
            seal_transfers: true,
            peers: Vec::new(),
        }
    }
}

/// A single federation peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPeerSettings {
    /// Peer's node ID.
    pub node_id: String,
    /// Display name.
    #[serde(default)]
    pub name: String,
    /// Base URL, e.g. `https://peer.example.com`.
    pub endpoint: String,
    /// Shared key for this peer, as a literal or a KMS URI
    /// (`env://NAME`, `file://path`, `aws-sm://…`, `azure-kv://…`, `vault://…`).
    ///
    /// Prefer a URI. A literal here is a secret sitting in a config file that
    /// tends to end up in version control.
    ///
    /// The same key authenticates both directions: this node sends it to the
    /// peer, and accepts it from the peer. One shared secret per pair rather
    /// than two half-configured ones.
    #[serde(default)]
    pub api_key: Option<String>,
    /// Whether sync with this peer is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// serde default for boolean fields that should default to `true`.
fn default_true() -> bool {
    true
}

/// Telemetry and analytics settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySettings {
    /// Whether this config *permits* telemetry (default: true).
    ///
    /// This is a gate, not the switch. Setting it `false` disables telemetry
    /// outright; leaving it `true` defers to `telemetry.yaml`, whose own
    /// `enabled` defaults to **false**. A default install therefore sends
    /// nothing — `iv` is opt-in, as the README states.
    ///
    /// The gate defaults open on purpose: flipping it to `false` would
    /// override users who opted in via `telemetry.yaml`.
    pub enabled: bool,
    /// Anonymous device ID (auto-generated)
    #[serde(default = "default_device_id")]
    pub device_id: String,
}

fn default_device_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl Default for TelemetrySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            device_id: default_device_id(),
        }
    }
}

/// Relocates every config/data/cache directory under one root.
pub const ENV_HOME: &str = "IRONVAULT_HOME";
/// Overrides the config directory (holds `config.yaml`, profiles, plugins).
pub const ENV_CONFIG: &str = "IRONVAULT_CONFIG";
/// Overrides the default vault name.
pub const ENV_VAULT: &str = "IRONVAULT_VAULT";

/// Serialises directory creation and permission tightening within the process.
static INIT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Create a directory, tolerating a concurrent creator.
///
/// Two processes (or two threads) initialising the vault at once will race:
/// one may be rewriting a parent's ACL while the other creates a child. A
/// single retry covers that window; `AlreadyExists` is always success.
fn create_dir_resilient(dir: &std::path::Path) -> Result<()> {
    match fs::create_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(_) if dir.is_dir() => Ok(()),
        Err(first) => match fs::create_dir_all(dir) {
            Ok(()) => Ok(()),
            Err(_) if dir.is_dir() => Ok(()),
            Err(_) => Err(VaultError::IoError(first)),
        },
    }
}

/// Read an environment variable, treating empty/whitespace values as unset.
///
/// Delegates to [`crate::env::var`] so the 4.x `aimodelvault_*` / `AIM_*`
/// spellings keep working through the 5.0 rename.
fn non_empty_env(name: &str) -> Option<String> {
    crate::env::var(name)
}

/// XDG-compliant directory paths for config, data, cache, and logs.
#[derive(Debug, Clone, Default)]
pub struct DirectoryPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    /// The directory vaults live *in* — `…/data/vaults`, not one vault.
    ///
    /// [`VaultConfig::get_vault_path`] joins the vault's name onto this, so
    /// naming a vault here gives you `…/vaults/default/default`. Worth stating,
    /// because the tests in `vault.rs` all set it to `data/vaults/default` and a
    /// downstream caller copied that shape from them.
    pub vault_dir: PathBuf,
    pub log_dir: PathBuf,
    pub backends_dir: PathBuf,
    pub utilities_dir: PathBuf,
    pub databases_dir: PathBuf,
}

impl VaultConfig {
    /// Create new configuration with defaults
    ///
    /// Honors three environment overrides:
    /// - `IRONVAULT_HOME` — relocate all config/data/cache directories
    /// - `IRONVAULT_CONFIG` — path to the config file to load
    /// - `IRONVAULT_VAULT` — default vault name
    pub fn new() -> Result<Self> {
        let dirs = Self::get_project_dirs()?;
        Self::ensure_directories(&dirs)?;

        let config_file = dirs.config_dir.join("config.yaml");

        let mut config = if config_file.exists() {
            Self::load_from_file(&config_file, dirs)?
        } else {
            let config = Self::default_with_dirs(dirs);
            config.save()?;
            config
        };

        if let Some(name) = non_empty_env(ENV_VAULT) {
            config.vault.default_vault = name;
        }

        Ok(config)
    }

    /// Create configuration with custom directory paths
    pub fn with_dirs(dirs: DirectoryPaths) -> Result<Self> {
        Self::ensure_directories(&dirs)?;
        Ok(Self::default_with_dirs(dirs))
    }

    /// Get XDG project directories for IronVault (AIMV)
    ///
    /// Uses shorter, organized paths:
    /// - ~/.config/ai/models/
    /// - ~/.local/share/ai/models/
    /// - ~/.cache/ai/models/
    /// - ~/.config/ai/backends/
    /// - ~/.config/ai/utilities/
    /// - ~/.config/ai/databases/
    fn get_project_dirs() -> Result<DirectoryPaths> {
        let mut dirs = Self::platform_dirs()?;

        // `IRONVAULT_CONFIG` relocates just the config tree.
        if let Some(config_root) = non_empty_env(ENV_CONFIG) {
            let config_dir = PathBuf::from(config_root);
            dirs.backends_dir = config_dir.join("backends");
            dirs.utilities_dir = config_dir.join("utilities");
            dirs.databases_dir = config_dir.join("databases");
            dirs.config_dir = config_dir;
        }

        Ok(dirs)
    }

    /// Directory layout before environment overrides are applied.
    fn platform_dirs() -> Result<DirectoryPaths> {
        use directories::BaseDirs;

        // `IRONVAULT_HOME` relocates every directory under one root. Used for
        // test isolation, containers, and per-project vaults.
        if let Some(root) = non_empty_env(ENV_HOME) {
            let root = PathBuf::from(root);
            let config_dir = root.join("config");
            let data_dir = root.join("data");
            return Ok(DirectoryPaths {
                cache_dir: root.join("cache"),
                vault_dir: data_dir.join("vaults"),
                log_dir: data_dir.join("logs"),
                backends_dir: config_dir.join("backends"),
                utilities_dir: config_dir.join("utilities"),
                databases_dir: config_dir.join("databases"),
                config_dir,
                data_dir,
            });
        }

        let base_dirs = BaseDirs::new().ok_or_else(|| {
            VaultError::ConfigError("Failed to determine base directories".to_string())
        })?;

        // Use shorter paths under ~/.config/ai/, ~/.local/share/ai/, etc.
        let config_base = base_dirs.config_dir().join("ai");
        let data_base = base_dirs.data_dir().join("ai");
        let cache_base = base_dirs.cache_dir().join("ai");

        let config_dir = config_base.join("models");
        let data_dir = data_base.join("models");
        let cache_dir = cache_base.join("models");
        let vault_dir = data_dir.join("vaults");
        let log_dir = data_dir.join("logs");
        let backends_dir = config_base.join("backends");
        let utilities_dir = config_base.join("utilities");
        let databases_dir = config_base.join("databases");

        Ok(DirectoryPaths {
            config_dir,
            data_dir,
            cache_dir,
            vault_dir,
            log_dir,
            backends_dir,
            utilities_dir,
            databases_dir,
        })
    }

    /// Ensure all required directories exist with secure permissions
    fn ensure_directories(dirs: &DirectoryPaths) -> Result<()> {
        let all = [
            &dirs.config_dir,
            &dirs.data_dir,
            &dirs.cache_dir,
            &dirs.vault_dir,
            &dirs.log_dir,
            &dirs.backends_dir,
            &dirs.utilities_dir,
            &dirs.databases_dir,
        ];

        // Serialise first-run setup. Several callers in one process (CLI
        // handlers, the API server's workers, the test harness) can initialise
        // the same directories at once, and on Windows `icacls /inheritance:r`
        // briefly leaves a directory without a usable DACL — a concurrent
        // create or write in that window fails with "Access is denied".
        let _guard = INIT_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        // Restrict each directory immediately after creating it, parents before
        // children: `vault_dir` and `log_dir` live under `data_dir`, and
        // tightening a parent's ACL once a child already exists makes `icacls`
        // fail on the child.
        for dir in all {
            if !dir.is_dir() {
                // A separate process may still be mid-setup.
                create_dir_resilient(dir)?;
                crate::permissions::restrict_dir(dir)?;
            }
        }

        Ok(())
    }

    /// Load configuration from file
    fn load_from_file(path: &PathBuf, dirs: DirectoryPaths) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        let mut config: VaultConfig = serde_yaml_ng::from_str(&contents)?;
        config.dirs = dirs;
        Ok(config)
    }

    /// Create default configuration with directories
    fn default_with_dirs(dirs: DirectoryPaths) -> Self {
        Self {
            version: "1.0".to_string(),
            vault: VaultSettings {
                default_vault: "default".to_string(),
            },
            crypto: CryptoSettings {
                algorithm: "aes-256-gcm".to_string(),
                kdf: "argon2id".to_string(),
            },
            compression: CompressionSettings {
                algorithm: "gzip".to_string(),
                level: 6,
            },
            storage: StorageSettings {
                max_versions: 10,
                auto_cleanup: true,
                checkpoint_format: "v{version}_{timestamp}".to_string(),
                streaming_threshold: default_streaming_threshold(),
            },
            security: SecuritySettings {
                require_passphrase: true,
                session_timeout_seconds: 3600,
                audit_log: true,
                blockchain_audit: false,
                blockchain_block_size: default_blockchain_block_size(),
            },
            compliance: ComplianceSettings {
                cve_scanning: true,
                audit_retention_days: 90,
            },
            telemetry: TelemetrySettings::default(),
            federation: FederationSettings::default(),
            dirs,
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_file = self.dirs.config_dir.join("config.yaml");
        let contents = serde_yaml_ng::to_string(self)?;
        fs::write(&config_file, contents)?;
        crate::permissions::restrict_file(&config_file)?;

        Ok(())
    }

    /// Get path to specific vault
    pub fn get_vault_path(&self, vault_name: Option<&str>) -> PathBuf {
        let name = vault_name.unwrap_or(&self.vault.default_vault);
        self.dirs.vault_dir.join(name)
    }

    /// Get audit log path
    pub fn get_audit_log_path(&self) -> PathBuf {
        self.dirs.log_dir.join("audit.log")
    }

    /// Directory holding the blockchain audit trail's block files.
    ///
    /// Sits beside the audit log rather than inside the vault data directory:
    /// the chain exists to be checked against the vault, so a single delete of
    /// the vault should not take the evidence with it.
    pub fn get_audit_chain_dir(&self) -> PathBuf {
        self.dirs.log_dir.join("chain")
    }

    /// Get compression algorithm
    pub fn get_compression_algorithm(&self) -> CompressionAlgorithm {
        match self.compression.algorithm.as_str() {
            "gzip" => CompressionAlgorithm::Gzip,
            "lzma" => CompressionAlgorithm::Lzma,
            "none" => CompressionAlgorithm::None,
            _ => CompressionAlgorithm::Gzip,
        }
    }

    /// Get compression level
    pub fn get_compression_level(&self) -> CompressionLevel {
        match self.compression.level {
            0 => CompressionLevel::None,
            1 => CompressionLevel::Fast,
            9 => CompressionLevel::Maximum,
            _ => CompressionLevel::Balanced,
        }
    }
}

/// Note: `VaultConfig::default()` panics if the home directory cannot be determined.
/// Prefer `VaultConfig::new()` which returns `Result` for fallible creation.
impl Default for VaultConfig {
    fn default() -> Self {
        Self::new().expect("Failed to create default configuration: home directory unavailable")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_settings_default() {
        // Covers line 126 — TelemetrySettings::default()
        // This is the permission gate, which defaults open; the effective
        // switch is `telemetry::TelemetryConfig::enabled`, asserted below.
        let ts = TelemetrySettings::default();
        assert!(ts.enabled);
        assert!(!ts.device_id.is_empty());
    }

    /// A default install must transmit nothing.
    ///
    /// Two structs are both called "telemetry enabled" with opposite defaults:
    /// the gate here (open) and `telemetry::TelemetryConfig` (closed). Only
    /// their combination is opt-in, so a well-meaning change to either could
    /// silently start beaconing. This pins the property the README promises.
    #[test]
    fn test_telemetry_is_opt_in_by_default() {
        let effective = crate::telemetry::TelemetryConfig::default();
        assert!(
            !effective.enabled,
            "telemetry must be off by default — the README promises opt-in"
        );
    }

    #[test]
    fn test_vault_config_with_dirs() {
        // Covers line 165 — VaultConfig::with_dirs()
        let temp = tempfile::tempdir().unwrap();
        let dirs = DirectoryPaths {
            config_dir: temp.path().join("config"),
            data_dir: temp.path().join("data"),
            cache_dir: temp.path().join("cache"),
            vault_dir: temp.path().join("vaults"),
            log_dir: temp.path().join("logs"),
            backends_dir: temp.path().join("backends"),
            utilities_dir: temp.path().join("utils"),
            databases_dir: temp.path().join("dbs"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        assert!(config.dirs.config_dir.starts_with(temp.path()));
    }

    #[test]
    fn test_vault_config_new() {
        // Covers lines 155, 158, 159, 160 — VaultConfig::new() both branches
        let config = VaultConfig::new().unwrap();
        assert!(!config.dirs.config_dir.as_os_str().is_empty());
    }

    #[test]
    fn test_compression_level_settings() {
        let mut config = VaultConfig::new().unwrap();
        config.compression.level = 0;
        assert!(matches!(
            config.get_compression_level(),
            CompressionLevel::None
        ));
        config.compression.level = 1;
        assert!(matches!(
            config.get_compression_level(),
            CompressionLevel::Fast
        ));
        config.compression.level = 9;
        assert!(matches!(
            config.get_compression_level(),
            CompressionLevel::Maximum
        ));
        config.compression.level = 5;
        assert!(matches!(
            config.get_compression_level(),
            CompressionLevel::Balanced
        ));
    }

    #[test]
    fn test_compression_algorithm_variants() {
        let mut config = VaultConfig::new().unwrap();
        config.compression.algorithm = "gzip".to_string();
        assert!(matches!(
            config.get_compression_algorithm(),
            CompressionAlgorithm::Gzip
        ));
        config.compression.algorithm = "lzma".to_string();
        assert!(matches!(
            config.get_compression_algorithm(),
            CompressionAlgorithm::Lzma
        ));
        config.compression.algorithm = "none".to_string();
        assert!(matches!(
            config.get_compression_algorithm(),
            CompressionAlgorithm::None
        ));
        config.compression.algorithm = "unknown_algo".to_string();
        assert!(matches!(
            config.get_compression_algorithm(),
            CompressionAlgorithm::Gzip
        ));
    }

    #[test]
    fn test_vault_config_save_and_reload() {
        let temp = tempfile::tempdir().unwrap();
        let dirs = DirectoryPaths {
            config_dir: temp.path().join("config"),
            data_dir: temp.path().join("data"),
            cache_dir: temp.path().join("cache"),
            vault_dir: temp.path().join("vaults"),
            log_dir: temp.path().join("logs"),
            backends_dir: temp.path().join("backends"),
            utilities_dir: temp.path().join("utils"),
            databases_dir: temp.path().join("dbs"),
        };
        let config = VaultConfig::with_dirs(dirs.clone()).unwrap();
        config.save().unwrap();

        // Now load_from_file is exercised
        let config_file = dirs.config_dir.join("config.yaml");
        assert!(config_file.exists());
        let reloaded = VaultConfig::load_from_file(&config_file, dirs).unwrap();
        assert_eq!(reloaded.vault.default_vault, "default");
        assert_eq!(reloaded.crypto.algorithm, "aes-256-gcm");
    }

    #[test]
    fn test_vault_path_and_audit_log_path() {
        let config = VaultConfig::new().unwrap();
        let vault_path = config.get_vault_path(None);
        assert!(vault_path.ends_with("default"));

        let custom_path = config.get_vault_path(Some("my-vault"));
        assert!(custom_path.ends_with("my-vault"));

        let audit_path = config.get_audit_log_path();
        assert!(audit_path.ends_with("audit.log"));
    }

    #[test]
    fn test_vault_config_default_impl() {
        // Covers L335-336 — Default for VaultConfig
        let config = VaultConfig::default();
        assert_eq!(config.vault.default_vault, "default");
        assert_eq!(config.crypto.algorithm, "aes-256-gcm");
    }
}
