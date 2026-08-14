//! Main Vault implementation
//!
//! Provides high-level API for secure model storage and retrieval.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::audit::{AuditEntry, AuditEventType, AuditLogger};
use crate::config::VaultConfig;
use crate::crypto::{KeyManager, SecureKey, VaultCrypto};
use crate::error::{Result, VaultError};
use crate::formats::ModelMetadata;
use crate::storage::Storage;
use crate::traits::{EventBus, VaultEvent, VaultState, VersionRepo};
use crate::version::{ModelVersion, VersionControl};

/// Version storage backend selector.
///
/// Allows choosing between JSON file-based storage (backward-compatible)
/// and SQLite with ACID guarantees. Both implement the `VersionRepo` trait;
/// this enum provides seamless dispatch.
pub enum VersionBackend {
    /// JSON file-based storage (default, backward-compatible).
    Json(VersionControl),
    /// SQLite database with WAL mode and ACID guarantees.
    #[cfg(feature = "sqlite")]
    Sqlite(crate::version_sqlite::SqliteVersionRepo),
}

impl VersionBackend {
    #[allow(clippy::too_many_arguments)]
    fn add_version(
        &mut self,
        model: &str,
        file_path: &str,
        format: &str,
        size_bytes: u64,
        compressed_size_bytes: u64,
        checksum: &str,
        metadata: Option<HashMap<String, String>>,
        parent_version: Option<u32>,
    ) -> Result<ModelVersion> {
        match self {
            Self::Json(vc) => vc.add_version(
                model,
                file_path,
                format,
                size_bytes,
                compressed_size_bytes,
                checksum,
                metadata,
                parent_version,
            ),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(repo) => repo.add_version(
                model,
                file_path,
                format,
                size_bytes,
                compressed_size_bytes,
                checksum,
                metadata,
                parent_version,
            ),
        }
    }

    fn get_version(&self, model: &str, version: Option<u32>) -> Option<&ModelVersion> {
        match self {
            Self::Json(vc) => vc.get_version(model, version),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(repo) => repo.get_version(model, version),
        }
    }

    fn list_versions(&self, model: &str) -> Vec<&ModelVersion> {
        match self {
            Self::Json(vc) => vc.list_versions(model),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(repo) => repo.list_versions(model),
        }
    }

    fn get_lineage(&self, model: &str, version: u32) -> Vec<&ModelVersion> {
        match self {
            Self::Json(vc) => vc.get_lineage(model, version),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(repo) => repo.get_lineage(model, version),
        }
    }

    fn delete_version(&mut self, model: &str, version: u32) -> Result<bool> {
        match self {
            Self::Json(vc) => vc.delete_version(model, version),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(repo) => repo.delete_version(model, version),
        }
    }

    fn cleanup_old_versions(&mut self, model: &str, keep_count: usize) -> Result<Vec<u32>> {
        match self {
            Self::Json(vc) => vc.cleanup_old_versions(model, keep_count),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(repo) => repo.cleanup_old_versions(model, keep_count),
        }
    }

    fn verify_checksum(&self, model: &str, version: u32, data: &[u8]) -> bool {
        match self {
            Self::Json(vc) => vc.verify_checksum(model, version, data),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(repo) => repo.verify_checksum(model, version, data),
        }
    }

    fn update_metadata(
        &mut self,
        model: &str,
        version: u32,
        key: &str,
        value: String,
    ) -> Result<()> {
        match self {
            Self::Json(vc) => vc.update_metadata(model, version, key, value),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(repo) => repo.update_metadata(model, version, key, value),
        }
    }

    fn get_metadata(&self, model: &str, version: u32, key: &str) -> Option<String> {
        match self {
            Self::Json(vc) => vc.get_metadata(model, version, key),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(repo) => repo.get_metadata(model, version, key),
        }
    }

    fn list_models(&self) -> Vec<String> {
        match self {
            Self::Json(vc) => {
                use crate::traits::VersionRepo;
                vc.list_models()
            }
            #[cfg(feature = "sqlite")]
            Self::Sqlite(repo) => {
                use crate::traits::VersionRepo;
                repo.list_models()
            }
        }
    }

    /// Number of distinct models tracked.
    fn model_count(&self) -> usize {
        match self {
            Self::Json(vc) => vc.versions.len(),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(repo) => {
                use crate::traits::VersionRepo;
                repo.list_models().len()
            }
        }
    }

    /// Total number of versions across all models.
    fn total_version_count(&self) -> usize {
        match self {
            Self::Json(vc) => vc.versions.values().map(|v| v.len()).sum(),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(repo) => {
                let models = {
                    use crate::traits::VersionRepo;
                    repo.list_models()
                };
                models.iter().map(|m| repo.list_versions(m).len()).sum()
            }
        }
    }

    /// Iterate all model names and their versions (used by re-encryption).
    fn all_model_versions(&self) -> Vec<(String, Vec<ModelVersion>)> {
        match self {
            Self::Json(vc) => vc
                .versions
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(repo) => {
                let models = {
                    use crate::traits::VersionRepo;
                    repo.list_models()
                };
                models
                    .into_iter()
                    .map(|m| {
                        let vers = repo.list_versions(&m).into_iter().cloned().collect();
                        (m, vers)
                    })
                    .collect()
            }
        }
    }
}

/// Main vault for secure model storage
/// Build the audit logger for a config, with the blockchain mirror attached
/// when `security.blockchain_audit` is set.
///
/// `blockchain_audit` is subordinate to `audit_log`: with logging off there is
/// no entry stream to mirror, so the chain stays off too regardless of the
/// flag. Shared by both vault constructors so the two cannot drift.
fn build_audit_logger(config: &VaultConfig) -> Result<Option<AuditLogger>> {
    if !config.security.audit_log {
        return Ok(None);
    }

    let log_path = config.get_audit_log_path();
    let logger = if config.security.blockchain_audit {
        AuditLogger::with_chain(
            &log_path,
            &config.get_audit_chain_dir(),
            config.security.blockchain_block_size,
        )?
    } else {
        AuditLogger::new(&log_path)?
    };

    Ok(Some(logger))
}

pub struct Vault {
    config: VaultConfig,
    storage: Storage,
    version_backend: VersionBackend,
    audit_logger: Option<AuditLogger>,
    crypto: VaultCrypto,
    key_manager: KeyManager,
    active_key: Option<SecureKey>,
    /// Event bus for dispatching domain events to subscribers.
    event_bus: EventBus,
    /// Shared metrics counters updated by MetricsSubscriber.
    metrics: Option<std::sync::Arc<crate::traits::VaultMetrics>>,
    /// Tracks the number of operations since unlock (for observability).
    operations_count: AtomicU64,
    /// When the vault was unlocked (for state reporting).
    unlocked_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Vault {
    /// Create or open a vault (uses JSON version backend by default).
    ///
    /// For SQLite version storage, use [`VaultBuilder::sqlite_versions()`].
    pub fn new(config: Option<VaultConfig>) -> Result<Self> {
        let config = match config {
            Some(c) => c,
            None => VaultConfig::new()?,
        };

        let vault_path = config.get_vault_path(None);

        // Ensure vault directory exists
        if !vault_path.exists() {
            fs::create_dir_all(&vault_path)?;
            crate::permissions::restrict_dir(&vault_path)?;
        }

        let storage = Storage::new(&vault_path)?;
        let version_backend = VersionBackend::Json(VersionControl::new(&vault_path)?);

        let audit_logger = build_audit_logger(&config)?;

        let crypto = VaultCrypto::new()?;
        let key_manager = KeyManager::new()?;

        if let Some(logger) = &audit_logger {
            logger.log(AuditEntry {
                timestamp: chrono::Utc::now(),
                event_type: AuditEventType::VaultOpened,
                description: "Vault opened".to_string(),
                model_name: None,
                version: None,
                success: true,
                metadata: None,
            })?;
        }

        // Initialize event bus with audit subscriber if logging is enabled
        let event_bus = EventBus::new();

        Ok(Self {
            config,
            storage,
            version_backend,
            audit_logger,
            crypto,
            key_manager,
            active_key: None,
            event_bus,
            metrics: None,
            operations_count: AtomicU64::new(0),
            unlocked_at: None,
        })
    }

    /// Get a snapshot of vault metrics (models stored/retrieved/deleted, bytes, errors).
    ///
    /// Returns None if the vault was created via Vault::new() without
    /// the VaultBuilder (which auto-wires the MetricsSubscriber).
    pub fn metrics(&self) -> Option<crate::traits::MetricsSnapshot> {
        self.metrics.as_ref().map(|m| m.snapshot())
    }

    /// Get the vault name from configuration.
    fn vault_name(&self) -> String {
        self.config.vault.default_vault.clone()
    }

    /// Unlock vault with passphrase
    ///
    /// The salt used for key derivation is persisted in the vault directory.
    /// This ensures the same passphrase always derives the same key across sessions.
    /// Constant sealed under the vault key so a wrong passphrase can be
    /// detected at unlock rather than at first decryption.
    const KEY_CHECK_MAGIC: &'static [u8] = b"IRONVAULT-KEY-CHECK-v1";

    /// Confirm `key` is the vault's key, or fail with [`VaultError::AuthenticationFailed`].
    ///
    /// Three cases, in order:
    ///
    /// 1. A `vault.keycheck` exists — decrypt it. AES-256-GCM authenticates, so
    ///    a wrong key cannot forge a passing result.
    /// 2. No keycheck but the vault holds data (a vault created before 6.2.1).
    ///    The key is proved against a real stored blob first, then the keycheck
    ///    is written. Writing it unconditionally would let a *wrong* first
    ///    unlock mint a keycheck for the wrong key and lock the owner out of
    ///    their own vault — a far worse bug than the one being fixed.
    /// 3. No keycheck and no data — any passphrase is legitimately correct for
    ///    an empty vault, so the keycheck is created and checked from then on.
    fn verify_key(&self, key: &SecureKey, vault_path: &Path) -> Result<()> {
        let check_file = vault_path.join("vault.keycheck");

        if check_file.exists() {
            let sealed = fs::read(&check_file)?;
            let opened = self
                .crypto
                .decrypt(&sealed, key)
                .map_err(|_| VaultError::AuthenticationFailed)?;
            return if opened == Self::KEY_CHECK_MAGIC {
                Ok(())
            } else {
                Err(VaultError::AuthenticationFailed)
            };
        }

        if let Some(existing) = self.any_stored_version() {
            self.storage
                .retrieve_auto(&existing, key, self.config.get_compression_algorithm())
                .map_err(|_| VaultError::AuthenticationFailed)?;
        }

        let sealed = self.crypto.encrypt(Self::KEY_CHECK_MAGIC, key)?;
        use std::io::Write;
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create_new(true);
        crate::permissions::set_create_mode(&mut opts);
        // A concurrent unlock may have written it first; that is not an error,
        // and the next unlock will check against it.
        if let Ok(mut f) = opts.open(&check_file) {
            f.write_all(&sealed)?;
            drop(f);
            crate::permissions::restrict_file(&check_file)?;
        }
        Ok(())
    }

    /// Path of any one stored blob, used to prove a key against real data.
    fn any_stored_version(&self) -> Option<String> {
        for name in self.version_backend.list_models() {
            if let Some(v) = self.version_backend.get_version(&name, None) {
                return Some(v.file_path.clone());
            }
        }
        None
    }

    pub fn unlock(&mut self, passphrase: Vec<u8>) -> Result<()> {
        let vault_path = self.config.get_vault_path(None);
        let salt_file = vault_path.join("vault.salt");

        // Load existing salt or generate a new one
        let existing_salt = if salt_file.exists() {
            Some(fs::read(&salt_file)?)
        } else {
            None
        };

        let (key, salt) = self.crypto.derive_key(passphrase, existing_salt)?;

        // Persist the salt if it's new
        if !salt_file.exists() {
            // Create with restrictive permissions atomically (no TOCTOU gap)
            use std::io::Write;
            let mut opts = fs::OpenOptions::new();
            opts.write(true).create_new(true);
            crate::permissions::set_create_mode(&mut opts);
            let mut f = opts.open(&salt_file)?;
            f.write_all(&salt)?;
            drop(f);
            crate::permissions::restrict_file(&salt_file)?;
        }

        // Prove the derived key before accepting it. Deriving a key always
        // succeeds -- Argon2 will happily stretch the wrong passphrase -- so
        // without this, `unlock` returned Ok for any input and the mistake only
        // surfaced when an AEAD tag failed on the first read. Everything that
        // did not touch ciphertext therefore worked with the wrong passphrase:
        // `iv list` and `iv stats` printed the inventory and exited 0, and
        // `POST /api/v1/auth/token` issued an admin JWT that could read the
        // model list, the audit log, the ACLs and the policies.
        self.verify_key(&key, &vault_path)?;

        self.active_key = Some(key);
        self.unlocked_at = Some(chrono::Utc::now());
        self.operations_count.store(0, Ordering::Relaxed);

        if let Some(logger) = &self.audit_logger {
            logger.log_auth(true, None)?;
        }

        // Emit unlock event
        self.event_bus.emit(&VaultEvent::VaultUnlocked {
            vault: self.vault_name(),
            timestamp: chrono::Utc::now(),
        });

        Ok(())
    }

    /// Lock vault (clear active key)
    pub fn lock(&mut self) {
        self.active_key = None;

        // Emit lock event
        self.event_bus.emit(&VaultEvent::VaultLocked {
            vault: self.vault_name(),
            timestamp: chrono::Utc::now(),
        });

        self.unlocked_at = None;
        self.operations_count.store(0, Ordering::Relaxed);
    }

    /// Check if vault is unlocked
    #[must_use]
    pub fn is_unlocked(&self) -> bool {
        self.active_key.is_some()
    }

    /// Store a model
    pub fn store_model(
        &mut self,
        name: &str,
        data: Vec<u8>,
        metadata: ModelMetadata,
        parent_version: Option<u32>,
    ) -> Result<ModelVersion> {
        let key = self
            .active_key
            .as_ref()
            .ok_or_else(|| VaultError::SecurityViolation("Vault is locked".to_string()))?;

        // Compute checksum before compression/encryption
        let checksum = hex::encode(VaultCrypto::hash_sha256(&data));

        // Generate filename
        let filename = format!("{}.vault", uuid::Uuid::new_v4());

        // Store data (compress + encrypt) — use streaming for large models
        let use_streaming = data.len() as u64 >= self.config.storage.streaming_threshold;

        let (original_size, compressed_size) = if use_streaming {
            self.storage.store_streamed(
                &filename,
                &data,
                key,
                self.config.get_compression_algorithm(),
                self.config.get_compression_level(),
            )?
        } else {
            self.storage.store(
                &filename,
                &data,
                key,
                self.config.get_compression_algorithm(),
                self.config.get_compression_level(),
            )?
        };

        // Convert metadata to version control format
        let mut version_metadata = HashMap::new();
        if let Some(desc) = metadata.description {
            version_metadata.insert("description".to_string(), desc);
        }
        if let Some(framework) = metadata.framework {
            version_metadata.insert("framework".to_string(), framework);
        }
        if let Some(task) = metadata.task {
            version_metadata.insert("task".to_string(), task);
        }
        version_metadata.extend(metadata.custom_fields);

        // Add version control entry
        let version = self.version_backend.add_version(
            name,
            &filename,
            metadata.format.name(),
            original_size,
            compressed_size,
            &checksum,
            Some(version_metadata),
            parent_version,
        )?;

        // Audit log
        if let Some(logger) = &self.audit_logger {
            logger.log_model_stored(name, version.version, true)?;
        }

        // Emit event
        self.event_bus.emit(&VaultEvent::ModelStored {
            vault: self.vault_name(),
            model: name.to_string(),
            version: version.version,
            format: metadata.format.name().to_string(),
            size: original_size,
            checksum: checksum.clone(),
            timestamp: chrono::Utc::now(),
        });
        self.operations_count.fetch_add(1, Ordering::Relaxed);

        // Auto-cleanup old versions if enabled
        if self.config.storage.auto_cleanup {
            let deleted = self
                .version_backend
                .cleanup_old_versions(name, self.config.storage.max_versions as usize)?;

            // Delete associated files
            for ver in deleted {
                if let Some(old_version) = self.version_backend.get_version(name, Some(ver)) {
                    let _ = self.storage.delete(&old_version.file_path);
                }
            }
        }

        Ok(version)
    }

    /// Retrieve a model
    ///
    /// Auto-detects chunked (streaming) encryption format and decrypts accordingly.
    pub fn get_model(&self, name: &str, version: Option<u32>) -> Result<Vec<u8>> {
        let key = self
            .active_key
            .as_ref()
            .ok_or_else(|| VaultError::SecurityViolation("Vault is locked".to_string()))?;

        let model_version = self
            .version_backend
            .get_version(name, version)
            .ok_or_else(|| {
                if let Some(v) = version {
                    VaultError::VersionNotFound(v, name.to_string())
                } else {
                    VaultError::ModelNotFound(name.to_string())
                }
            })?;

        // Retrieve data (decrypt + decompress) — auto-detects chunked format
        let data = self.storage.retrieve_auto(
            &model_version.file_path,
            key,
            self.config.get_compression_algorithm(),
        )?;

        // Verify integrity
        if !self
            .version_backend
            .verify_checksum(name, model_version.version, &data)
        {
            if let Some(logger) = &self.audit_logger {
                let _ = logger.log(AuditEntry {
                    timestamp: chrono::Utc::now(),
                    event_type: AuditEventType::IntegrityFailure,
                    description: format!(
                        "Integrity check failed for model '{}' version {}",
                        name, model_version.version
                    ),
                    model_name: Some(name.to_string()),
                    version: Some(model_version.version),
                    success: false,
                    metadata: None,
                });
            }

            // Emit integrity failure event
            self.event_bus.emit(&VaultEvent::IntegrityFailed {
                vault: self.vault_name(),
                model: name.to_string(),
                version: model_version.version,
                expected: model_version.checksum_sha256.clone(),
                actual: hex::encode(VaultCrypto::hash_sha256(&data)),
                timestamp: chrono::Utc::now(),
            });

            return Err(VaultError::IntegrityError(format!(
                "Checksum mismatch for model '{}' version {}",
                name, model_version.version
            )));
        }

        // Audit log
        if let Some(logger) = &self.audit_logger {
            logger.log_model_retrieved(name, model_version.version, true)?;
        }

        // Emit retrieval event
        self.event_bus.emit(&VaultEvent::ModelRetrieved {
            vault: self.vault_name(),
            model: name.to_string(),
            version: model_version.version,
            timestamp: chrono::Utc::now(),
        });
        self.operations_count.fetch_add(1, Ordering::Relaxed);

        Ok(data)
    }

    /// The size of a model's plaintext, without reading or decrypting it.
    ///
    /// Read this first, allocate exactly this much, then fill it with
    /// [`Vault::read_model_into`].
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::ModelNotFound`] or [`VaultError::VersionNotFound`]
    /// if there is no such model or version.
    pub fn model_plaintext_len(&self, name: &str, version: Option<u32>) -> Result<u64> {
        self.version_backend
            .get_version(name, version)
            .map(|v| v.size_bytes)
            .ok_or_else(|| match version {
                Some(v) => VaultError::VersionNotFound(v, name.to_string()),
                None => VaultError::ModelNotFound(name.to_string()),
            })
    }

    /// Decrypt a model directly into `dst`, holding one chunk at a time.
    ///
    /// [`Vault::get_model`] reads the whole ciphertext, decrypts it into a
    /// second allocation, and decompresses into a third — around 3× the model in
    /// peak residency, and the caller then usually copies it somewhere a fourth
    /// time. This writes the plaintext exactly once, into memory the caller
    /// already owns, and never holds more than one 4 MiB chunk.
    ///
    /// It exists for inference engines. IronWorks maps a model as one flat byte
    /// range, page-locks parts of it for host-to-device transfer, and captures
    /// CUDA graphs holding those host pointers, so the plaintext has to land in
    /// one contiguous buffer it controls. Given this call it can allocate that
    /// buffer page-locked up front and decrypt straight into it, which is both
    /// the cheapest way to load an encrypted model and the only way to run one
    /// without writing plaintext to disk.
    ///
    /// `dst` must be exactly [`Vault::model_plaintext_len`] bytes.
    ///
    /// # Integrity
    ///
    /// Equivalent to [`Vault::get_model`]: every chunk's GCM tag is checked as
    /// it is decrypted, the stream MAC is checked at the end (this reads to EOF
    /// deliberately, so truncation cannot pass), and the SHA-256 recorded for
    /// the version is verified over the filled buffer. A model that fails any of
    /// these leaves `dst` zeroed rather than partially populated, so a caller
    /// that ignores the error cannot run on half-decrypted weights.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::SecurityViolation`] if the vault is locked,
    /// [`VaultError::ModelNotFound`] / [`VaultError::VersionNotFound`] if it does
    /// not exist, [`VaultError::InvalidInput`] if `dst` is the wrong size, and
    /// [`VaultError::IntegrityError`] if any check fails.
    pub fn read_model_into(
        &self,
        name: &str,
        version: Option<u32>,
        dst: &mut [u8],
    ) -> Result<usize> {
        let key = self
            .active_key
            .as_ref()
            .ok_or_else(|| VaultError::SecurityViolation("Vault is locked".to_string()))?;

        let model_version = self
            .version_backend
            .get_version(name, version)
            .ok_or_else(|| match version {
                Some(v) => VaultError::VersionNotFound(v, name.to_string()),
                None => VaultError::ModelNotFound(name.to_string()),
            })?;

        // Name what we got, not just what we needed: a caller that sized its
        // buffer from a stale listing should be told both numbers.
        if dst.len() as u64 != model_version.size_bytes {
            return Err(VaultError::InvalidInput(format!(
                "destination is {} bytes but model '{}' version {} is {} bytes",
                dst.len(),
                name,
                model_version.version,
                model_version.size_bytes
            )));
        }

        let file_path = model_version.file_path.clone();
        let expected_checksum = model_version.checksum_sha256.clone();
        let model_version_number = model_version.version;

        let result = self
            .storage
            .retrieve_into(
                &file_path,
                key,
                self.config.get_compression_algorithm(),
                dst,
            )
            .and_then(|written| {
                let actual = VaultCrypto::hash_sha256_hex(dst);
                if actual == expected_checksum {
                    Ok(written)
                } else {
                    Err(VaultError::IntegrityError(format!(
                        "Checksum mismatch for model '{name}' version {model_version_number}"
                    )))
                }
            });

        if let Err(error) = result {
            // Never leave a caller holding plausible-looking half-decrypted
            // weights. A model that failed authentication must not be runnable.
            dst.fill(0);

            if matches!(error, VaultError::IntegrityError(_)) {
                if let Some(logger) = &self.audit_logger {
                    let _ = logger.log(AuditEntry {
                        timestamp: chrono::Utc::now(),
                        event_type: AuditEventType::IntegrityFailure,
                        description: format!(
                            "Integrity check failed for model '{name}' version {model_version_number}"
                        ),
                        model_name: Some(name.to_string()),
                        version: Some(model_version_number),
                        success: false,
                        metadata: None,
                    });
                }
                self.event_bus.emit(&VaultEvent::IntegrityFailed {
                    vault: self.vault_name(),
                    model: name.to_string(),
                    version: model_version_number,
                    expected: expected_checksum,
                    actual: "streamed".to_string(),
                    timestamp: chrono::Utc::now(),
                });
            }
            return Err(error);
        }

        if let Some(logger) = &self.audit_logger {
            logger.log_model_retrieved(name, model_version_number, true)?;
        }
        self.event_bus.emit(&VaultEvent::ModelRetrieved {
            vault: self.vault_name(),
            model: name.to_string(),
            version: model_version_number,
            timestamp: chrono::Utc::now(),
        });
        self.operations_count.fetch_add(1, Ordering::Relaxed);

        result
    }

    /// List all models in vault
    #[must_use]
    pub fn list_models(&self) -> Vec<String> {
        self.version_backend.list_models()
    }

    /// List versions of a model
    pub fn list_versions(&self, name: &str) -> Vec<&ModelVersion> {
        self.version_backend.list_versions(name)
    }

    /// Get model lineage/history
    pub fn get_lineage(&self, name: &str, version: u32) -> Vec<&ModelVersion> {
        self.version_backend.get_lineage(name, version)
    }

    /// Delete a specific version
    pub fn delete_version(&mut self, name: &str, version: u32) -> Result<bool> {
        if let Some(model_version) = self.version_backend.get_version(name, Some(version)) {
            let file_path = model_version.file_path.clone();

            // Delete from version control
            let deleted = self.version_backend.delete_version(name, version)?;

            if deleted {
                // Delete file
                self.storage.delete(&file_path)?;

                // Audit log
                if let Some(logger) = &self.audit_logger {
                    logger.log(AuditEntry {
                        timestamp: chrono::Utc::now(),
                        event_type: AuditEventType::VersionDeleted,
                        description: format!("Deleted model '{}' version {}", name, version),
                        model_name: Some(name.to_string()),
                        version: Some(version),
                        success: true,
                        metadata: None,
                    })?;
                }

                // Emit deletion event
                self.event_bus.emit(&VaultEvent::ModelDeleted {
                    vault: self.vault_name(),
                    model: name.to_string(),
                    version,
                    timestamp: chrono::Utc::now(),
                });
                self.operations_count.fetch_add(1, Ordering::Relaxed);
            }

            Ok(deleted)
        } else {
            Ok(false)
        }
    }

    /// Get vault statistics
    pub fn get_stats(&self) -> Result<VaultStats> {
        let storage_stats = self.storage.get_stats()?;
        let model_count = self.version_backend.model_count();
        let total_versions = self.version_backend.total_version_count();

        Ok(VaultStats {
            model_count,
            total_versions,
            total_size_bytes: storage_stats.total_size_bytes,
            file_count: storage_stats.file_count,
        })
    }

    /// Get vault configuration
    pub fn get_config(&self) -> &VaultConfig {
        &self.config
    }

    /// Get the key manager
    pub fn key_manager(&self) -> &KeyManager {
        &self.key_manager
    }

    /// Borrow the blockchain audit trail, if `security.blockchain_audit` is on.
    ///
    /// `None` means the chain was never enabled for this vault -- not that it
    /// is empty. Callers should say so rather than reporting a height of zero,
    /// which reads as "nothing has happened" when the truth is "nothing was
    /// being recorded".
    pub fn audit_chain(&self) -> Option<&std::sync::Mutex<crate::blockchain::BlockchainAudit>> {
        self.audit_logger.as_ref().and_then(AuditLogger::chain)
    }

    /// Change vault passphrase
    ///
    /// Re-derives and persists a new salt, then re-encrypts all stored model files.
    /// Auto-detects chunked format on read and uses streaming for large re-encryptions.
    pub fn change_passphrase(&mut self, new_passphrase: Vec<u8>) -> Result<usize> {
        let old_key = self
            .active_key
            .as_ref()
            .ok_or_else(|| VaultError::SecurityViolation("Vault is locked".to_string()))?
            .clone();

        // Derive new key (fresh salt)
        let (new_key, new_salt) = self.crypto.derive_key(new_passphrase, None)?;

        let compression_algo = self.config.get_compression_algorithm();

        // Re-encrypt every stored file
        let mut re_encrypted = 0usize;
        let all_versions = self.version_backend.all_model_versions();

        for (_model_name, versions) in &all_versions {
            for ver in versions {
                // Decrypt with old key (auto-detects chunked format)
                let data =
                    self.storage
                        .retrieve_auto(&ver.file_path, &old_key, compression_algo)?;

                // Delete old file & re-store with new key (streaming for large models)
                self.storage.delete(&ver.file_path)?;
                if data.len() as u64 >= self.config.storage.streaming_threshold {
                    self.storage.store_streamed(
                        &ver.file_path,
                        &data,
                        &new_key,
                        compression_algo,
                        self.config.get_compression_level(),
                    )?;
                } else {
                    self.storage.store(
                        &ver.file_path,
                        &data,
                        &new_key,
                        compression_algo,
                        self.config.get_compression_level(),
                    )?;
                }

                re_encrypted += 1;
            }
        }

        // Persist new salt
        let vault_path = self.config.get_vault_path(None);
        let salt_file = vault_path.join("vault.salt");
        fs::write(&salt_file, &new_salt)?;
        crate::permissions::restrict_file(&salt_file)?;

        // Re-seal the key check under the new key. Without this the vault
        // re-encrypts every blob and then refuses the new passphrase at the
        // next unlock, because the check still holds a constant sealed under
        // the old one — a passphrase change that locks the owner out.
        let check_file = vault_path.join("vault.keycheck");
        let sealed = self.crypto.encrypt(Self::KEY_CHECK_MAGIC, &new_key)?;
        fs::write(&check_file, &sealed)?;
        crate::permissions::restrict_file(&check_file)?;

        self.active_key = Some(new_key);

        if let Some(logger) = &self.audit_logger {
            logger.log(AuditEntry {
                timestamp: chrono::Utc::now(),
                event_type: AuditEventType::VaultOpened,
                description: format!("Passphrase changed, {} files re-encrypted", re_encrypted),
                model_name: None,
                version: None,
                success: true,
                metadata: None,
            })?;
        }

        // Emit passphrase changed event
        self.event_bus.emit(&VaultEvent::PassphraseChanged {
            vault: self.vault_name(),
            files_reencrypted: re_encrypted,
            timestamp: chrono::Utc::now(),
        });

        Ok(re_encrypted)
    }

    /// Update metadata for a specific model version
    pub fn update_version_metadata(
        &mut self,
        model_name: &str,
        version: u32,
        key: &str,
        value: String,
    ) -> Result<()> {
        self.version_backend
            .update_metadata(model_name, version, key, value)
    }

    /// Get metadata for a specific model version
    pub fn get_version_metadata(
        &self,
        model_name: &str,
        version: u32,
        key: &str,
    ) -> Option<String> {
        self.version_backend.get_metadata(model_name, version, key)
    }

    /// Get the current observable vault state.
    ///
    /// Agents can query this at any time to understand what the vault is doing.
    pub fn state(&self) -> VaultState {
        if self.active_key.is_some() {
            VaultState::Unlocked {
                vault_name: self.vault_name(),
                model_count: self.version_backend.model_count(),
                unlocked_at: self.unlocked_at.unwrap_or_else(chrono::Utc::now),
                operations_count: self.operations_count.load(Ordering::Relaxed),
            }
        } else {
            VaultState::Locked {
                vault_name: self.vault_name(),
                model_count: self.version_backend.model_count(),
            }
        }
    }

    /// Get mutable reference to the event bus for registering subscribers.
    pub fn event_bus_mut(&mut self) -> &mut EventBus {
        &mut self.event_bus
    }

    /// Get reference to the event bus.
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Store a model from an iterator of chunks (streaming ingest).
    ///
    /// Collects chunks into a contiguous buffer, then encrypts and stores.
    /// For models that are too large to hold entirely in memory at the call
    /// site, the caller can feed data in increments.
    pub fn store_model_streamed<I>(
        &mut self,
        name: &str,
        chunks: I,
        metadata: ModelMetadata,
        parent_version: Option<u32>,
    ) -> Result<ModelVersion>
    where
        I: IntoIterator<Item = Vec<u8>>,
    {
        let mut buf = Vec::new();
        for chunk in chunks {
            buf.extend_from_slice(&chunk);
        }
        self.store_model(name, buf, metadata, parent_version)
    }

    /// Retrieve a model as fixed-size chunks (streaming retrieval).
    ///
    /// Decrypts the full model, then returns a `ModelStream` that yields
    /// `chunk_size`-byte pieces.  This avoids handing Python a single
    /// multi-GB `bytes` object.
    pub fn get_model_chunked(
        &self,
        name: &str,
        version: Option<u32>,
        chunk_size: usize,
    ) -> Result<ModelStream> {
        let data = self.get_model(name, version)?;
        Ok(ModelStream::new(data, chunk_size))
    }

    /// Get the version backend type name (for diagnostics).
    #[must_use]
    pub fn version_backend_name(&self) -> &'static str {
        match &self.version_backend {
            VersionBackend::Json(_) => "json",
            #[cfg(feature = "sqlite")]
            VersionBackend::Sqlite(_) => "sqlite",
        }
    }
}

/// Builder for configuring a [`Vault`] with optional backends.
///
/// # Example
///
/// ```rust,no_run
/// use ironvault::VaultBuilder;
///
/// let vault = VaultBuilder::new()
///     .build()
///     .expect("Failed to create vault");
/// ```
pub struct VaultBuilder {
    config: Option<VaultConfig>,
    use_sqlite: bool,
    /// Custom event subscribers to register.
    subscribers: Vec<Box<dyn crate::traits::EventSubscriber>>,
    /// Whether to auto-register built-in subscribers (audit + metrics).
    /// Defaults to true.
    default_subscribers: bool,
}

impl VaultBuilder {
    /// Create a new builder with defaults (JSON version backend).
    pub fn new() -> Self {
        Self {
            config: None,
            use_sqlite: false,
            subscribers: Vec::new(),
            default_subscribers: true,
        }
    }

    /// Set a custom [`VaultConfig`].
    pub fn config(mut self, config: VaultConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Use SQLite for version storage instead of JSON.
    ///
    /// Requires the `sqlite` feature. Automatically migrates existing
    /// `versions.json` data on first open.
    #[cfg(feature = "sqlite")]
    pub fn sqlite_versions(mut self) -> Self {
        self.use_sqlite = true;
        self
    }

    /// Register a custom [EventSubscriber](crate::traits::EventSubscriber).
    ///
    /// Subscribers receive domain events (model stored, retrieved, etc.)
    /// and can be used for custom audit logging, metrics, or integrations.
    pub fn subscriber(mut self, sub: Box<dyn crate::traits::EventSubscriber>) -> Self {
        self.subscribers.push(sub);
        self
    }

    /// Disable the built-in audit and metrics subscribers.
    ///
    /// By default, `VaultBuilder` registers an `AuditLogSubscriber` (when
    /// audit logging is enabled in config) and a `MetricsSubscriber`. Call
    /// this to start with a clean event bus.
    pub fn no_default_subscribers(mut self) -> Self {
        self.default_subscribers = false;
        self
    }

    /// Build the vault.
    pub fn build(self) -> Result<Vault> {
        let config = match self.config {
            Some(c) => c,
            None => VaultConfig::new()?,
        };

        let vault_path = config.get_vault_path(None);

        // Ensure vault directory exists
        if !vault_path.exists() {
            fs::create_dir_all(&vault_path)?;
            crate::permissions::restrict_dir(&vault_path)?;
        }

        let storage = Storage::new(&vault_path)?;

        let version_backend = if self.use_sqlite {
            #[cfg(feature = "sqlite")]
            {
                VersionBackend::Sqlite(crate::version_sqlite::SqliteVersionRepo::new(&vault_path)?)
            }
            #[cfg(not(feature = "sqlite"))]
            {
                return Err(VaultError::ConfigError(
                    "SQLite version backend requires the `sqlite` feature".to_string(),
                ));
            }
        } else {
            VersionBackend::Json(VersionControl::new(&vault_path)?)
        };

        let audit_logger = build_audit_logger(&config)?;

        let crypto = VaultCrypto::new()?;
        let key_manager = KeyManager::new()?;

        if let Some(logger) = &audit_logger {
            logger.log(AuditEntry {
                timestamp: chrono::Utc::now(),
                event_type: AuditEventType::VaultOpened,
                description: "Vault opened".to_string(),
                model_name: None,
                version: None,
                success: true,
                metadata: None,
            })?;
        }

        let mut shared_metrics: Option<std::sync::Arc<crate::traits::VaultMetrics>> = None;
        let event_bus = {
            let mut bus = EventBus::new();

            if self.default_subscribers {
                // Wire AuditLogSubscriber when audit logging is active
                if config.security.audit_log {
                    use crate::traits::AuditLogSubscriber;
                    if let Ok(sink_logger) = AuditLogger::new(&config.get_audit_log_path()) {
                        bus.subscribe(Box::new(AuditLogSubscriber::new(Box::new(sink_logger))));
                    }
                }

                // Wire MetricsSubscriber for observability
                use crate::traits::MetricsSubscriber;
                let metrics_arc = std::sync::Arc::new(crate::traits::VaultMetrics::new());
                bus.subscribe(Box::new(MetricsSubscriber::new(metrics_arc.clone())));
                shared_metrics = Some(metrics_arc);
            }

            // Register any custom subscribers from the builder
            for sub in self.subscribers {
                bus.subscribe(sub);
            }

            bus
        };

        Ok(Vault {
            config,
            storage,
            version_backend,
            audit_logger,
            crypto,
            key_manager,
            active_key: None,
            event_bus,
            metrics: shared_metrics,
            operations_count: AtomicU64::new(0),
            unlocked_at: None,
        })
    }
}

impl Default for VaultBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator that yields fixed-size chunks of decrypted model data.
///
/// Created by [`Vault::get_model_chunked`].
pub struct ModelStream {
    data: Vec<u8>,
    offset: usize,
    chunk_size: usize,
}

impl ModelStream {
    /// Create a new `ModelStream`.
    pub fn new(data: Vec<u8>, chunk_size: usize) -> Self {
        let chunk_size = if chunk_size == 0 { 1 << 20 } else { chunk_size };
        Self {
            data,
            offset: 0,
            chunk_size,
        }
    }

    /// Total size of the underlying data in bytes.
    #[must_use]
    pub fn total_size(&self) -> usize {
        self.data.len()
    }

    /// Number of bytes remaining.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.offset)
    }
}

impl Iterator for ModelStream {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }
        let end = (self.offset + self.chunk_size).min(self.data.len());
        let chunk = self.data[self.offset..end].to_vec();
        self.offset = end;
        Some(chunk)
    }
}

/// Vault statistics
#[derive(Debug, Clone)]
pub struct VaultStats {
    pub model_count: usize,
    pub total_versions: usize,
    pub total_size_bytes: u64,
    pub file_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::ModelFormat;
    use tempfile::tempdir;

    #[test]
    fn test_vault_operations() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };

        let config = VaultConfig::with_dirs(dirs).unwrap();
        let mut vault = Vault::new(Some(config)).unwrap();

        // Unlock vault
        let passphrase = b"test_passphrase_with_sufficient_entropy".to_vec();
        vault.unlock(passphrase).unwrap();

        // Store model
        let data = b"Test model data".to_vec();
        let metadata = ModelMetadata::new("test_model".to_string(), ModelFormat::PyTorch)
            .with_description("Test model".to_string());

        let version = vault
            .store_model("test_model", data.clone(), metadata, None)
            .unwrap();
        assert_eq!(version.version, 1);

        // Retrieve model
        let retrieved = vault.get_model("test_model", None).unwrap();
        assert_eq!(data, retrieved);

        // List models
        let models = vault.list_models();
        assert_eq!(models.len(), 1);
        assert!(models.contains(&"test_model".to_string()));
    }

    #[test]
    fn test_vault_builder_default() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };

        let config = VaultConfig::with_dirs(dirs).unwrap();
        let vault = VaultBuilder::new().config(config).build().unwrap();
        assert_eq!(vault.version_backend_name(), "json");
    }

    #[test]
    fn test_streaming_store_retrieve() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };

        let mut config = VaultConfig::with_dirs(dirs).unwrap();
        // Set threshold to 0 so even small data uses streaming
        config.storage.streaming_threshold = 0;

        let mut vault = Vault::new(Some(config)).unwrap();
        vault
            .unlock(b"test_passphrase_with_sufficient_entropy".to_vec())
            .unwrap();

        let data = b"Streaming encrypted model data for testing".to_vec();
        let metadata = ModelMetadata::new("stream_test".to_string(), ModelFormat::Safetensors);

        let version = vault
            .store_model("stream_test", data.clone(), metadata, None)
            .unwrap();
        assert_eq!(version.version, 1);

        // Retrieve — auto-detects chunked format
        let retrieved = vault.get_model("stream_test", None).unwrap();
        assert_eq!(data, retrieved);
    }

    #[test]
    fn test_store_model_streamed_and_get_chunked() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };

        let config = VaultConfig::with_dirs(dirs).unwrap();
        let mut vault = Vault::new(Some(config)).unwrap();
        vault
            .unlock(b"test_passphrase_for_streamed_ops".to_vec())
            .unwrap();

        // Store via streamed API (chunks)
        let data = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".to_vec();
        let chunks: Vec<Vec<u8>> = data.chunks(10).map(|c| c.to_vec()).collect();
        let metadata = ModelMetadata::new("chunked_model".to_string(), ModelFormat::ONNX);

        let version = vault
            .store_model_streamed("chunked_model", chunks, metadata, None)
            .unwrap();
        assert_eq!(version.version, 1);

        // Retrieve via chunked API
        let stream = vault.get_model_chunked("chunked_model", None, 8).unwrap();
        let mut reassembled = Vec::new();
        for chunk in stream {
            reassembled.extend_from_slice(&chunk);
        }
        assert_eq!(data, reassembled);
    }

    #[test]
    fn test_vault_builder_with_subscriber() {
        use crate::traits::{EventSubscriber, VaultEvent};
        use std::sync::{Arc, Mutex};

        struct TestSubscriber {
            events: Arc<Mutex<Vec<String>>>,
        }

        impl EventSubscriber for TestSubscriber {
            fn on_event(&self, event: &VaultEvent) -> crate::error::Result<()> {
                let name = match event {
                    VaultEvent::ModelStored { .. } => "stored",
                    VaultEvent::ModelRetrieved { .. } => "retrieved",
                    _ => "other",
                };
                self.events.lock().unwrap().push(name.to_string());
                Ok(())
            }
            fn name(&self) -> &str {
                "TestSubscriber"
            }
        }

        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };

        let config = VaultConfig::with_dirs(dirs).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let sub = TestSubscriber {
            events: events.clone(),
        };

        let mut vault = VaultBuilder::new()
            .config(config)
            .no_default_subscribers()
            .subscriber(Box::new(sub))
            .build()
            .unwrap();

        vault
            .unlock(b"test_passphrase_subscriber_test".to_vec())
            .unwrap();

        let data = b"subscriber test data".to_vec();
        let metadata = ModelMetadata::new("sub_model".to_string(), ModelFormat::PyTorch);
        vault
            .store_model("sub_model", data, metadata, None)
            .unwrap();
        vault.get_model("sub_model", None).unwrap();

        let captured = events.lock().unwrap();
        assert!(captured.contains(&"stored".to_string()));
        assert!(captured.contains(&"retrieved".to_string()));
    }

    #[test]
    fn test_vault_version_backend_name() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };

        let config = VaultConfig::with_dirs(dirs).unwrap();
        let vault = VaultBuilder::new().config(config).build().unwrap();
        assert_eq!(vault.version_backend_name(), "json");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_vault_sqlite_version_backend() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };

        let config = VaultConfig::with_dirs(dirs).unwrap();
        let mut vault = VaultBuilder::new()
            .config(config)
            .sqlite_versions()
            .build()
            .unwrap();
        assert_eq!(vault.version_backend_name(), "sqlite");

        // Exercise SQLite version backend through vault operations
        vault
            .unlock(b"test_sqlite_vault_passphrase".to_vec())
            .unwrap();

        let data = b"sqlite backend model data".to_vec();
        let metadata = ModelMetadata::new("sqlite_model".to_string(), ModelFormat::Safetensors);

        let version = vault
            .store_model("sqlite_model", data.clone(), metadata, None)
            .unwrap();
        assert_eq!(version.version, 1);

        let models = vault.list_models();
        assert!(models.contains(&"sqlite_model".to_string()));

        let versions = vault.list_versions("sqlite_model");
        assert_eq!(versions.len(), 1);

        let retrieved = vault.get_model("sqlite_model", Some(1)).unwrap();
        assert_eq!(data, retrieved);

        // Store second version
        let data2 = b"sqlite backend model data v2".to_vec();
        let meta2 = ModelMetadata::new("sqlite_model".to_string(), ModelFormat::Safetensors);
        let v2 = vault
            .store_model("sqlite_model", data2.clone(), meta2, Some(1))
            .unwrap();
        assert_eq!(v2.version, 2);

        // Delete version
        assert!(vault.delete_version("sqlite_model", 1).unwrap());

        let versions = vault.list_versions("sqlite_model");
        assert_eq!(versions.len(), 1);
    }

    #[test]
    fn test_vault_lock_unlock_state() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        let mut vault = VaultBuilder::new().config(config).build().unwrap();

        // Initially locked
        assert!(!vault.is_unlocked());
        match vault.state() {
            crate::traits::VaultState::Locked {
                vault_name,
                model_count,
            } => {
                assert_eq!(vault_name, "default");
                assert_eq!(model_count, 0);
            }
            _ => panic!("Expected Locked state"),
        }

        // Unlock
        vault
            .unlock(b"test_passphrase_lock_unlock".to_vec())
            .unwrap();
        assert!(vault.is_unlocked());
        match vault.state() {
            crate::traits::VaultState::Unlocked {
                vault_name,
                model_count,
                operations_count,
                ..
            } => {
                assert_eq!(vault_name, "default");
                assert_eq!(model_count, 0);
                assert_eq!(operations_count, 0);
            }
            _ => panic!("Expected Unlocked state"),
        }

        // Lock
        vault.lock();
        assert!(!vault.is_unlocked());
        match vault.state() {
            crate::traits::VaultState::Locked { .. } => {}
            _ => panic!("Expected Locked state after lock"),
        }
    }

    #[test]
    fn test_vault_get_stats() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        let mut vault = Vault::new(Some(config)).unwrap();
        vault.unlock(b"test_passphrase_stats".to_vec()).unwrap();

        let stats = vault.get_stats().unwrap();
        assert_eq!(stats.model_count, 0);
        assert_eq!(stats.total_versions, 0);

        // Store a model and check stats again
        let data = b"Stats test model data".to_vec();
        let metadata = ModelMetadata::new("stats_model".to_string(), ModelFormat::Safetensors);
        vault
            .store_model("stats_model", data, metadata, None)
            .unwrap();

        let stats = vault.get_stats().unwrap();
        assert_eq!(stats.model_count, 1);
        assert_eq!(stats.total_versions, 1);
        assert!(stats.total_size_bytes > 0);
        assert!(stats.file_count > 0);
    }

    #[test]
    fn test_vault_delete_version() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        let mut vault = Vault::new(Some(config)).unwrap();
        vault.unlock(b"test_passphrase_delete".to_vec()).unwrap();

        let data = b"Delete test model".to_vec();
        let metadata = ModelMetadata::new("del_model".to_string(), ModelFormat::PyTorch);
        vault
            .store_model("del_model", data, metadata, None)
            .unwrap();

        // Delete existing version
        let deleted = vault.delete_version("del_model", 1).unwrap();
        assert!(deleted);

        // Delete non-existent version
        let deleted = vault.delete_version("del_model", 999).unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_vault_change_passphrase() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        let mut vault = Vault::new(Some(config)).unwrap();
        vault
            .unlock(b"original_passphrase_for_change_test".to_vec())
            .unwrap();

        let data = b"Change passphrase test data".to_vec();
        let metadata = ModelMetadata::new("cp_model".to_string(), ModelFormat::Safetensors);
        vault
            .store_model("cp_model", data.clone(), metadata, None)
            .unwrap();

        // Change passphrase
        let re_encrypted = vault
            .change_passphrase(b"new_passphrase_for_change_test".to_vec())
            .unwrap();
        assert_eq!(re_encrypted, 1);

        // Verify model is still retrievable
        let retrieved = vault.get_model("cp_model", None).unwrap();
        assert_eq!(data, retrieved);
    }

    #[test]
    fn test_vault_metrics() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        let mut vault = VaultBuilder::new().config(config).build().unwrap();

        // Metrics should be available via VaultBuilder (which wires MetricsSubscriber)
        let snapshot = vault.metrics();
        assert!(snapshot.is_some());
        let snap = snapshot.unwrap();
        assert_eq!(snap.models_stored_total, 0);

        // Store a model
        vault.unlock(b"metrics_test_passphrase".to_vec()).unwrap();
        let data = b"Metrics test model".to_vec();
        let metadata = ModelMetadata::new("metric_model".to_string(), ModelFormat::PyTorch);
        vault
            .store_model("metric_model", data, metadata, None)
            .unwrap();

        let snap = vault.metrics().unwrap();
        assert_eq!(snap.models_stored_total, 1);
    }

    #[test]
    fn test_vault_model_stream() {
        let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut stream = ModelStream::new(data.clone(), 3);
        assert_eq!(stream.total_size(), 10);
        assert_eq!(stream.remaining(), 10);

        let chunk1 = stream.next().unwrap();
        assert_eq!(chunk1, vec![1, 2, 3]);
        assert_eq!(stream.remaining(), 7);

        let chunk2 = stream.next().unwrap();
        assert_eq!(chunk2, vec![4, 5, 6]);

        let chunk3 = stream.next().unwrap();
        assert_eq!(chunk3, vec![7, 8, 9]);

        let chunk4 = stream.next().unwrap();
        assert_eq!(chunk4, vec![10]);

        assert!(stream.next().is_none());
        assert_eq!(stream.remaining(), 0);
    }

    #[test]
    fn test_vault_model_stream_zero_chunk() {
        // chunk_size=0 should default to 1MB
        let data = vec![0u8; 10];
        let stream = ModelStream::new(data, 0);
        assert_eq!(stream.chunk_size, 1 << 20);
    }

    #[test]
    fn test_vault_get_config_and_key_manager() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        let vault = Vault::new(Some(config)).unwrap();

        let cfg = vault.get_config();
        assert_eq!(cfg.vault.default_vault, "default");

        let _km = vault.key_manager();
    }

    #[test]
    fn test_vault_store_locked_error() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        let mut vault = Vault::new(Some(config)).unwrap();
        // Don't unlock — vault is locked
        let metadata = ModelMetadata::new("locked_model".to_string(), ModelFormat::PyTorch);
        let result = vault.store_model("locked_model", b"data".to_vec(), metadata, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_vault_get_model_locked_error() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        let vault = Vault::new(Some(config)).unwrap();
        let result = vault.get_model("nonexist", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_vault_event_bus_access() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        let mut vault = VaultBuilder::new()
            .config(config)
            .no_default_subscribers()
            .build()
            .unwrap();
        let _bus = vault.event_bus();
        let _bus_mut = vault.event_bus_mut();
    }

    #[test]
    fn test_vault_update_get_version_metadata() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        let mut vault = Vault::new(Some(config)).unwrap();
        vault.unlock(b"test_passphrase_metadata".to_vec()).unwrap();

        let data = b"Metadata test model".to_vec();
        let metadata = ModelMetadata::new("meta_model".to_string(), ModelFormat::Safetensors);
        vault
            .store_model("meta_model", data, metadata, None)
            .unwrap();

        vault
            .update_version_metadata("meta_model", 1, "author", "test_author".to_string())
            .unwrap();
        let val = vault.get_version_metadata("meta_model", 1, "author");
        assert_eq!(val, Some("test_author".to_string()));
    }

    #[test]
    fn test_vault_lineage_and_versions() {
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        let mut vault = Vault::new(Some(config)).unwrap();
        vault.unlock(b"test_passphrase_lineage".to_vec()).unwrap();

        let meta1 = ModelMetadata::new("lin_model".to_string(), ModelFormat::PyTorch);
        vault
            .store_model("lin_model", b"v1".to_vec(), meta1, None)
            .unwrap();

        let meta2 = ModelMetadata::new("lin_model".to_string(), ModelFormat::PyTorch);
        vault
            .store_model("lin_model", b"v2".to_vec(), meta2, Some(1))
            .unwrap();

        let versions = vault.list_versions("lin_model");
        assert_eq!(versions.len(), 2);

        let lineage = vault.get_lineage("lin_model", 2);
        assert!(!lineage.is_empty());
    }

    #[test]
    fn test_vault_builder_default_trait() {
        let builder = VaultBuilder::default();
        // Should be equivalent to VaultBuilder::new()
        let _ = builder;
    }

    #[test]
    fn test_vault_store_with_rich_metadata() {
        // Covers metadata insertion branches: description, framework, task
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        let mut vault = VaultBuilder::new()
            .config(config)
            .no_default_subscribers()
            .build()
            .unwrap();
        vault.unlock(b"rich_meta_pass".to_vec()).unwrap();

        let mut metadata = ModelMetadata::new("rich_model".to_string(), ModelFormat::Safetensors);
        metadata.description = Some("A rich test model".to_string());
        metadata.framework = Some("pytorch".to_string());
        metadata.task = Some("text-generation".to_string());

        let version = vault
            .store_model("rich_model", vec![10, 20, 30], metadata, None)
            .unwrap();
        assert_eq!(version.version, 1);

        // Verify metadata was stored
        let desc = vault.get_version_metadata("rich_model", 1, "description");
        assert_eq!(desc, Some("A rich test model".to_string()));
        let fw = vault.get_version_metadata("rich_model", 1, "framework");
        assert_eq!(fw, Some("pytorch".to_string()));
        let task = vault.get_version_metadata("rich_model", 1, "task");
        assert_eq!(task, Some("text-generation".to_string()));
    }

    #[test]
    fn test_vault_auto_cleanup() {
        // Covers auto_cleanup path in store_model
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let mut config = VaultConfig::with_dirs(dirs).unwrap();
        config.storage.auto_cleanup = true;
        config.storage.max_versions = 2;

        let mut vault = VaultBuilder::new()
            .config(config)
            .no_default_subscribers()
            .build()
            .unwrap();
        vault.unlock(b"cleanup_pass".to_vec()).unwrap();

        // Store 3 versions of the same model
        let m1 = ModelMetadata::new("cleanup_model".to_string(), ModelFormat::Safetensors);
        let v1 = vault
            .store_model("cleanup_model", vec![1u8], m1, None)
            .unwrap();
        assert_eq!(v1.version, 1);

        let m2 = ModelMetadata::new("cleanup_model".to_string(), ModelFormat::Safetensors);
        let v2 = vault
            .store_model("cleanup_model", vec![2u8], m2, Some(1))
            .unwrap();
        assert_eq!(v2.version, 2);

        let m3 = ModelMetadata::new("cleanup_model".to_string(), ModelFormat::Safetensors);
        let v3 = vault
            .store_model("cleanup_model", vec![3u8], m3, Some(2))
            .unwrap();
        assert_eq!(v3.version, 3);

        // With max_versions=2, version 1 should be cleaned up
        let versions = vault.list_versions("cleanup_model");
        assert!(
            versions.len() <= 2,
            "Expected at most 2 versions after auto-cleanup, got {}",
            versions.len()
        );
    }

    #[test]
    fn test_vault_sqlite_comprehensive_coverage() {
        // Covers Sqlite variant arms: get_lineage (L88), cleanup_old_versions (L103-106),
        // verify_checksum (L112), update_metadata (L126), get_metadata (L134),
        // list_models/model_count/total_version_count/all_model_versions (L157-197)
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };

        let mut config = VaultConfig::with_dirs(dirs).unwrap();
        config.storage.auto_cleanup = false;

        let mut vault = VaultBuilder::new()
            .config(config)
            .sqlite_versions()
            .no_default_subscribers()
            .build()
            .unwrap();
        vault.unlock(b"sqlite_comprehensive_pass".to_vec()).unwrap();

        // Store model v1
        let data1 = b"sqlite comprehensive model v1".to_vec();
        let meta1 = ModelMetadata::new("sc_model".to_string(), ModelFormat::Safetensors);
        let v1 = vault
            .store_model("sc_model", data1.clone(), meta1, None)
            .unwrap();
        assert_eq!(v1.version, 1);

        // Store model v2 with parent
        let data2 = b"sqlite comprehensive model v2".to_vec();
        let meta2 = ModelMetadata::new("sc_model".to_string(), ModelFormat::Safetensors);
        let v2 = vault
            .store_model("sc_model", data2.clone(), meta2, Some(1))
            .unwrap();
        assert_eq!(v2.version, 2);

        // Store a second model
        let data3 = b"second model data".to_vec();
        let meta3 = ModelMetadata::new("sc_model2".to_string(), ModelFormat::PyTorch);
        vault.store_model("sc_model2", data3, meta3, None).unwrap();

        // get_lineage via Sqlite — L88
        let lineage = vault.get_lineage("sc_model", 2);
        assert!(!lineage.is_empty());

        // verify_checksum via Sqlite — L112
        let retrieved = vault.get_model("sc_model", Some(1)).unwrap();
        assert_eq!(retrieved, data1);

        // update_metadata via Sqlite — L126
        vault
            .update_version_metadata("sc_model", 1, "custom_key", "custom_value".to_string())
            .unwrap();

        // get_metadata via Sqlite — L134
        let val = vault.get_version_metadata("sc_model", 1, "custom_key");
        assert_eq!(val, Some("custom_value".to_string()));

        // list_models via Sqlite — L157-159
        let models = vault.list_models();
        assert!(models.len() >= 2);

        // model_count via Sqlite — L169-174
        let stats = vault.get_stats().unwrap();
        assert!(stats.model_count >= 2);

        // total_version_count via Sqlite — L188-197
        assert!(stats.total_versions >= 3);

        // all_model_versions via Sqlite — L200-215 (used implicitly by change_passphrase)
        // We'll test change_passphrase which calls all_model_versions
        let re_encrypted = vault
            .change_passphrase(b"sqlite_new_pass".to_vec())
            .unwrap();
        assert_eq!(re_encrypted, 3); // 2 versions of sc_model + 1 of sc_model2

        // cleanup_old_versions via Sqlite — L103-106
        vault
            .version_backend
            .cleanup_old_versions("sc_model", 1)
            .unwrap();
        let versions = vault.list_versions("sc_model");
        assert_eq!(versions.len(), 1);
    }

    #[test]
    fn test_vault_unlock_existing_salt() {
        // Covers L313 — fs::read(&salt_file) for existing salt on second unlock
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        let mut vault = VaultBuilder::new()
            .config(config)
            .no_default_subscribers()
            .build()
            .unwrap();

        // First unlock — generates and saves new salt
        vault.unlock(b"salt_test_passphrase".to_vec()).unwrap();
        assert!(vault.is_unlocked());

        // Store a model while unlocked
        let data = b"salt reuse test data".to_vec();
        let meta = ModelMetadata::new("salt_model".to_string(), ModelFormat::Safetensors);
        vault
            .store_model("salt_model", data.clone(), meta, None)
            .unwrap();

        // Lock, then unlock again — reads existing salt from file (L313)
        vault.lock();
        assert!(!vault.is_unlocked());
        vault.unlock(b"salt_test_passphrase".to_vec()).unwrap();
        assert!(vault.is_unlocked());

        // Verify data is still retrievable with same derived key
        let retrieved = vault.get_model("salt_model", None).unwrap();
        assert_eq!(data, retrieved);
    }

    #[test]
    fn test_vault_change_passphrase_streaming() {
        // Covers L659-L661, L664 — streaming re-encryption in change_passphrase
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let mut config = VaultConfig::with_dirs(dirs).unwrap();
        // Set very low streaming threshold so data triggers streaming path
        config.storage.streaming_threshold = 1;

        let mut vault = VaultBuilder::new()
            .config(config)
            .no_default_subscribers()
            .build()
            .unwrap();
        vault.unlock(b"stream_change_pass".to_vec()).unwrap();

        // Store a model (any size > 1 byte triggers streaming)
        let data = b"streaming change passphrase test data".to_vec();
        let meta = ModelMetadata::new("stream_cp_model".to_string(), ModelFormat::Safetensors);
        vault
            .store_model("stream_cp_model", data.clone(), meta, None)
            .unwrap();

        // Change passphrase — should go through streaming re-encryption branch
        let re_encrypted = vault
            .change_passphrase(b"stream_new_pass".to_vec())
            .unwrap();
        assert_eq!(re_encrypted, 1);

        // Verify model is still retrievable
        let retrieved = vault.get_model("stream_cp_model", None).unwrap();
        assert_eq!(data, retrieved);
    }

    #[test]
    fn test_vault_get_model_not_found_errors() {
        // Covers L481-L482 (VersionNotFound) and L484 (ModelNotFound)
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let config = VaultConfig::with_dirs(dirs).unwrap();
        let mut vault = VaultBuilder::new()
            .config(config)
            .no_default_subscribers()
            .build()
            .unwrap();
        vault.unlock(b"not_found_test".to_vec()).unwrap();

        // Store one model
        let data = b"exists model".to_vec();
        let meta = ModelMetadata::new("exists_model".to_string(), ModelFormat::Safetensors);
        vault.store_model("exists_model", data, meta, None).unwrap();

        // ModelNotFound — no version specified, model doesn't exist
        let err = vault.get_model("nonexistent_model", None).unwrap_err();
        assert!(format!("{}", err).contains("nonexistent_model"));

        // VersionNotFound — specific version doesn't exist
        let err = vault.get_model("exists_model", Some(999)).unwrap_err();
        assert!(format!("{}", err).contains("999"));
    }

    #[test]
    fn test_vault_builder_no_config() {
        // Covers L886 — VaultConfig::new()? in VaultBuilder::build() when config is None
        // VaultConfig::new() tries to load from XDG dirs, which should succeed on any system
        // We just verify the builder works with no explicit config
        let result = VaultBuilder::new().build();
        // This should succeed (VaultConfig::new() creates default dirs)
        assert!(result.is_ok());
    }

    #[test]
    fn test_vault_integrity_failure_path() {
        // Covers L500-L525 — checksum mismatch triggers audit log + event + error
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let mut config = VaultConfig::with_dirs(dirs).unwrap();
        config.security.audit_log = true;

        let mut vault = VaultBuilder::new().config(config).build().unwrap();
        vault.unlock(b"integrity_test_pass".to_vec()).unwrap();

        // Store a model
        let data = b"valid model data for integrity test".to_vec();
        let meta = ModelMetadata::new("integrity_model".to_string(), ModelFormat::Safetensors);
        vault
            .store_model("integrity_model", data, meta, None)
            .unwrap();

        // Corrupt the stored checksum in the version backend
        if let VersionBackend::Json(vc) = &mut vault.version_backend {
            vc.versions.get_mut("integrity_model").unwrap()[0].checksum_sha256 =
                "bad_checksum_value".to_string();
        }

        // Attempt to retrieve — should trigger integrity failure path
        let err = vault.get_model("integrity_model", Some(1)).unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("Checksum mismatch"),
            "Expected integrity error, got: {}",
            msg
        );
    }

    #[test]
    fn test_vault_new_none_config() {
        // Covers L231 — VaultConfig::new()? in Vault::new(None)
        let result = Vault::new(None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_vault_builder_with_audit_log() {
        // Covers L923/L254 — audit_log enabled in VaultBuilder::build() + Vault::new()
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let mut config = VaultConfig::with_dirs(dirs).unwrap();
        config.security.audit_log = true;

        let vault = VaultBuilder::new().config(config).build().unwrap();
        // audit_logger should be Some when audit_log=true
        assert!(vault.audit_logger.is_some());
    }

    #[test]
    fn test_vault_new_with_audit_log() {
        // Covers L254 — audit_logger branch in Vault::new()
        let temp_dir = tempdir().unwrap();
        let dirs = crate::config::DirectoryPaths {
            config_dir: temp_dir.path().join("config"),
            data_dir: temp_dir.path().join("data"),
            cache_dir: temp_dir.path().join("cache"),
            vault_dir: temp_dir.path().join("data/vaults"),
            log_dir: temp_dir.path().join("data/logs"),
            backends_dir: temp_dir.path().join("config/backends"),
            utilities_dir: temp_dir.path().join("config/utilities"),
            databases_dir: temp_dir.path().join("config/databases"),
        };
        let mut config = VaultConfig::with_dirs(dirs).unwrap();
        config.security.audit_log = true;

        let vault = Vault::new(Some(config)).unwrap();
        assert!(vault.audit_logger.is_some());
    }
}
