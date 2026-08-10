//! Core trait definitions for IronVault architecture v2.
//!
//! These traits define the boundaries between subsystems, enabling:
//! - Dependency injection (concrete types injected, never hard-coded)
//! - Testability (mock implementations for unit tests)
//! - Extensibility (new backends without changing core logic)
//! - Composability (middleware/decorator pattern)
//!
//! # Architecture Layers
//!
//! ```text
//! interface → service → domain ← infra
//!                          ↑
//!                       platform
//! ```
//!
//! - **domain** traits define WHAT operations exist
//! - **infra** modules provide HOW they're implemented

use std::collections::HashMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::crypto::SecureKey;
use crate::error::{Result, VaultError};

// ──────────────────────────────────────────────────────────────
// Layer 3: Domain Types
// ──────────────────────────────────────────────────────────────

/// Observable vault state — agents can query this at any time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VaultState {
    /// Vault directory does not exist.
    Uninitialized,
    /// Vault exists but is locked (key not loaded).
    Locked {
        vault_name: String,
        model_count: usize,
    },
    /// Vault exists and is unlocked (key in memory).
    Unlocked {
        vault_name: String,
        model_count: usize,
        unlocked_at: DateTime<Utc>,
        operations_count: u64,
    },
    /// Vault is in an error state.
    Error { message: String },
}

impl fmt::Display for VaultState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VaultState::Uninitialized => write!(f, "Uninitialized"),
            VaultState::Locked { vault_name, .. } => write!(f, "Locked({})", vault_name),
            VaultState::Unlocked { vault_name, .. } => write!(f, "Unlocked({})", vault_name),
            VaultState::Error { message } => write!(f, "Error({})", message),
        }
    }
}

/// Parsed AIMV URI with typed components.
///
/// Format: `aimv://{vault}/{model}@{version}/{resource}?{query}`
///
/// Examples:
/// - `aimv://` — root (list vaults)
/// - `aimv://default/` — vault "default"
/// - `aimv://default/llama-3` — model latest
/// - `aimv://default/llama-3@3` — model version 3
/// - `aimv://default/llama-3@3/card` — model card
/// - `aimv://default/_stats` — vault statistics
/// - `aimv://default/_events?since=2026-01-01` — filtered event log
#[derive(Debug, Clone, PartialEq)]
pub struct AimvUri {
    /// Vault name (None for root).
    pub vault: Option<String>,
    /// Model name (None for vault-level resources).
    pub model: Option<String>,
    /// Version number (None for latest).
    pub version: Option<u32>,
    /// Sub-resource path (e.g., "card", "lineage", "_stats").
    pub resource: Option<String>,
    /// Query parameters.
    pub query: HashMap<String, String>,
}

impl AimvUri {
    /// URI scheme prefix.
    ///
    /// Deliberately **not** renamed for IronVault. The scheme is a published
    /// interface: it appears in `.well-known/ontology.jsonld`, in the JSON-LD
    /// `aimv:` term prefix, and in URIs that callers have stored. Renaming it
    /// would silently invalidate every `aimv://` reference in the wild for
    /// cosmetic gain. A new scheme, if it is ever wanted, belongs alongside
    /// this one rather than in place of it.
    pub const SCHEME: &'static str = "aimv://";

    /// Parse an AIMV URI string.
    ///
    /// # Examples
    /// ```
    /// use ironvault::traits::AimvUri;
    ///
    /// let uri = AimvUri::parse("aimv://default/llama-3@3/card").unwrap();
    /// assert_eq!(uri.vault, Some("default".into()));
    /// assert_eq!(uri.model, Some("llama-3".into()));
    /// assert_eq!(uri.version, Some(3));
    /// assert_eq!(uri.resource, Some("card".into()));
    /// ```
    pub fn parse(uri: &str) -> Result<Self> {
        let stripped = uri.strip_prefix(Self::SCHEME).ok_or_else(|| {
            VaultError::InvalidInput(format!("URI must start with '{}': {}", Self::SCHEME, uri))
        })?;

        // Split off query string
        let (path, query) = if let Some(idx) = stripped.find('?') {
            (&stripped[..idx], Self::parse_query(&stripped[idx + 1..]))
        } else {
            (stripped, HashMap::new())
        };

        // Split path into segments
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        let (vault, model_part, resource) = match segments.len() {
            0 => (None, None, None),
            1 => (Some(segments[0].to_string()), None, None),
            2 => (Some(segments[0].to_string()), Some(segments[1]), None),
            3 => (
                Some(segments[0].to_string()),
                Some(segments[1]),
                Some(segments[2].to_string()),
            ),
            _ => {
                return Err(VaultError::InvalidInput(format!(
                    "URI has too many path segments: {}",
                    uri
                )));
            }
        };

        // Parse model@version
        let (model, version) = if let Some(m) = model_part {
            if let Some(idx) = m.find('@') {
                let model_name = &m[..idx];
                let ver_str = &m[idx + 1..];
                let ver = ver_str.parse::<u32>().map_err(|_| {
                    VaultError::InvalidInput(format!("Invalid version number: {}", ver_str))
                })?;
                (Some(model_name.to_string()), Some(ver))
            } else {
                (Some(m.to_string()), None)
            }
        } else {
            (None, None)
        };

        Ok(Self {
            vault,
            model,
            version,
            resource,
            query,
        })
    }

    /// Parse query string into key-value pairs.
    fn parse_query(query: &str) -> HashMap<String, String> {
        query
            .split('&')
            .filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let key = parts.next()?;
                let value = parts.next().unwrap_or("");
                if key.is_empty() {
                    None
                } else {
                    Some((key.to_string(), value.to_string()))
                }
            })
            .collect()
    }
}

impl fmt::Display for AimvUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(Self::SCHEME)?;

        if let Some(vault) = &self.vault {
            f.write_str(vault)?;

            if let Some(model) = &self.model {
                f.write_str("/")?;
                f.write_str(model)?;

                if let Some(ver) = self.version {
                    write!(f, "@{}", ver)?;
                }

                if let Some(resource) = &self.resource {
                    f.write_str("/")?;
                    f.write_str(resource)?;
                }
            }
        }

        if !self.query.is_empty() {
            f.write_str("?")?;
            let pairs: Vec<String> = self
                .query
                .iter()
                .map(|(k, v)| {
                    if v.is_empty() {
                        k.clone()
                    } else {
                        format!("{}={}", k, v)
                    }
                })
                .collect();
            f.write_str(&pairs.join("&"))?;
        }

        Ok(())
    }
}

// ──────────────────────────────────────────────────────────────
// Layer 2: Infrastructure Traits
// ──────────────────────────────────────────────────────────────

/// Crypto operations — encrypt, decrypt, hash, derive keys.
///
/// Implementations:
/// - `FipsCrypto` (default: AES-256-GCM + Argon2id)
/// - Test mocks
pub trait CryptoProvider: Send + Sync {
    /// Derive an encryption key from a passphrase.
    ///
    /// Returns `(key, salt)` — the salt must be persisted for deterministic re-derivation.
    fn derive_key(
        &self,
        passphrase: Vec<u8>,
        salt: Option<Vec<u8>>,
    ) -> Result<(SecureKey, Vec<u8>)>;

    /// Encrypt data. Returns `nonce || ciphertext || auth_tag`.
    fn encrypt(&self, data: &[u8], key: &SecureKey) -> Result<Vec<u8>>;

    /// Decrypt data. Verifies authentication tag.
    fn decrypt(&self, encrypted_data: &[u8], key: &SecureKey) -> Result<Vec<u8>>;

    /// Compute SHA-256 hash.
    fn hash(&self, data: &[u8]) -> Vec<u8>;

    /// Compute SHA-256 hash as hex string.
    fn hash_hex(&self, data: &[u8]) -> String {
        hex::encode(self.hash(data))
    }

    /// Generate cryptographically secure random bytes.
    fn random_bytes(&self, length: usize) -> Vec<u8>;
}

/// Blob storage — content-addressable, backend-agnostic.
///
/// Both local filesystem and cloud backends implement this trait.
/// This is the sync version; async operations use `StorageBackend`.
///
/// Implementations:
/// - `Storage` (local filesystem, compress + encrypt)
/// - Cloud backends via `StorageBackend` (async)
/// - `MemoryBlobStore` (for testing)
pub trait BlobStore: Send + Sync {
    /// Store data under the given key. Returns `(original_size, stored_size)`.
    fn put(&self, key: &str, data: &[u8], encryption_key: &SecureKey) -> Result<(u64, u64)>;

    /// Retrieve data by key.
    fn get(&self, key: &str, encryption_key: &SecureKey) -> Result<Vec<u8>>;

    /// Delete data by key. Returns true if the key existed.
    fn remove(&self, key: &str) -> Result<bool>;

    /// Check if a key exists.
    fn exists(&self, key: &str) -> bool;

    /// Get the size of stored data.
    fn size(&self, key: &str) -> Result<u64>;

    /// List all keys.
    fn list_keys(&self) -> Result<Vec<String>>;

    /// Get storage statistics.
    fn stats(&self) -> Result<BlobStoreStats>;
}

/// Statistics returned by `BlobStore::stats()`.
#[derive(Debug, Clone, Serialize)]
pub struct BlobStoreStats {
    pub total_size_bytes: u64,
    pub file_count: usize,
}

/// Receipt returned after a successful async blob put.
#[derive(Debug, Clone, Serialize)]
pub struct BlobReceipt {
    /// Storage key.
    pub key: String,
    /// Size in bytes of the stored data.
    pub size_bytes: u64,
    /// Timestamp of storage.
    pub stored_at: DateTime<Utc>,
}

/// Metadata about a stored blob.
#[derive(Debug, Clone, Serialize)]
pub struct BlobInfo {
    /// Storage key.
    pub key: String,
    /// Size in bytes.
    pub size_bytes: u64,
}

/// Async blob storage — unified trait for local and cloud backends.
///
/// This is the async counterpart of [`BlobStore`]. Cloud backends
/// (S3, Azure, GCS) and the local async backend implement this trait.
///
/// Implementations:
/// - `StorageBackend` wrappers (via `AsyncBlobStoreAdapter`)
/// - Can bridge to sync `BlobStore` via `tokio::runtime::Runtime::block_on`
///
/// This trait unifies the previously separate `BlobStore` (sync, local-only)
/// and `StorageBackend` (async, cloud) abstractions.
#[async_trait::async_trait]
pub trait AsyncBlobStore: Send + Sync {
    /// Store data under the given key.
    async fn put(&self, key: &str, data: &[u8]) -> Result<BlobReceipt>;

    /// Retrieve data by key.
    async fn get(&self, key: &str) -> Result<Vec<u8>>;

    /// Delete data by key. Returns true if the key existed.
    async fn delete(&self, key: &str) -> Result<bool>;

    /// Check if a key exists.
    async fn exists(&self, key: &str) -> Result<bool>;

    /// List all keys, optionally filtered by prefix.
    async fn list(&self, prefix: Option<&str>) -> Result<Vec<BlobInfo>>;

    /// Get metadata about a stored blob.
    async fn stat(&self, key: &str) -> Result<BlobInfo>;
}

/// Adapter that wraps the existing `StorageBackend` trait as an `AsyncBlobStore`.
///
/// This bridges the legacy `StorageBackend` implementations (S3, Azure, Local)
/// to the new unified `AsyncBlobStore` trait.
pub struct AsyncBlobStoreAdapter<B: crate::storage::StorageBackend> {
    inner: B,
}

impl<B: crate::storage::StorageBackend> AsyncBlobStoreAdapter<B> {
    /// Wrap an existing `StorageBackend` implementation.
    pub fn new(backend: B) -> Self {
        Self { inner: backend }
    }
}

#[async_trait::async_trait]
impl<B: crate::storage::StorageBackend + 'static> AsyncBlobStore for AsyncBlobStoreAdapter<B> {
    async fn put(&self, key: &str, data: &[u8]) -> Result<BlobReceipt> {
        self.inner.upload(key, data).await?;
        Ok(BlobReceipt {
            key: key.to_string(),
            size_bytes: data.len() as u64,
            stored_at: Utc::now(),
        })
    }

    async fn get(&self, key: &str) -> Result<Vec<u8>> {
        self.inner.download(key).await
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        self.inner.delete(key).await
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        self.inner.exists(key).await
    }

    async fn list(&self, _prefix: Option<&str>) -> Result<Vec<BlobInfo>> {
        let keys = self.inner.list().await?;
        let mut infos = Vec::with_capacity(keys.len());
        for key in keys {
            let size = self.inner.size(&key).await.unwrap_or(0);
            infos.push(BlobInfo {
                key,
                size_bytes: size,
            });
        }
        Ok(infos)
    }

    async fn stat(&self, key: &str) -> Result<BlobInfo> {
        let size = self.inner.size(key).await?;
        Ok(BlobInfo {
            key: key.to_string(),
            size_bytes: size,
        })
    }
}

/// Version metadata repository.
///
/// Implementations:
/// - `VersionControl` (JSON file backend — current)
/// - Future: `SqliteVersionRepo` (indexed, concurrent, ACID) — see `version_sqlite` module
/// - `MemoryVersionRepo` (for testing)
#[allow(clippy::too_many_arguments)]
pub trait VersionRepo: Send + Sync {
    /// Add a new version for a model.
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
    ) -> Result<crate::version::ModelVersion>;

    /// Get a specific version, or the latest if version is None.
    fn get_version(
        &self,
        model: &str,
        version: Option<u32>,
    ) -> Option<&crate::version::ModelVersion>;

    /// List all versions of a model, sorted by version number.
    fn list_versions(&self, model: &str) -> Vec<&crate::version::ModelVersion>;

    /// Get the lineage chain for a version.
    fn get_lineage(&self, model: &str, version: u32) -> Vec<&crate::version::ModelVersion>;

    /// Delete a specific version. Returns true if it existed.
    fn delete_version(&mut self, model: &str, version: u32) -> Result<bool>;

    /// Clean up old versions, keeping `keep_count` most recent.
    fn cleanup_old_versions(&mut self, model: &str, keep_count: usize) -> Result<Vec<u32>>;

    /// Verify data checksum against stored checksum.
    fn verify_checksum(&self, model: &str, version: u32, data: &[u8]) -> bool;

    /// Update metadata for a version.
    fn update_metadata(
        &mut self,
        model: &str,
        version: u32,
        key: &str,
        value: String,
    ) -> Result<()>;

    /// Get metadata for a version.
    fn get_metadata(&self, model: &str, version: u32, key: &str) -> Option<String>;

    /// List all model names.
    fn list_models(&self) -> Vec<String>;
}

/// Audit event sink.
///
/// Implementations:
/// - `AuditLogger` (append-only file)
/// - `BlockchainAudit` (Merkle-chained blocks)
/// - `EventBusAuditSink` (forwards to event bus)
/// - `NullAuditSink` (for testing / disabled audit)
pub trait AuditSink: Send + Sync {
    /// Emit an audit event.
    fn emit(&self, entry: crate::audit::AuditEntry) -> Result<()>;

    /// Query audit entries with optional limit.
    fn query(&self, limit: Option<usize>) -> Result<Vec<crate::audit::AuditEntry>>;
}

// ──────────────────────────────────────────────────────────────
// Event System
// ──────────────────────────────────────────────────────────────

/// Domain events — the canonical record of "what happened."
///
/// Every state-changing operation emits a `VaultEvent`. Subscribers
/// (audit logger, metrics, webhooks, agents) react to these events.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum VaultEvent {
    VaultCreated {
        vault: String,
        timestamp: DateTime<Utc>,
    },
    VaultUnlocked {
        vault: String,
        timestamp: DateTime<Utc>,
    },
    VaultLocked {
        vault: String,
        timestamp: DateTime<Utc>,
    },
    ModelStored {
        vault: String,
        model: String,
        version: u32,
        format: String,
        size: u64,
        checksum: String,
        timestamp: DateTime<Utc>,
    },
    ModelRetrieved {
        vault: String,
        model: String,
        version: u32,
        timestamp: DateTime<Utc>,
    },
    ModelDeleted {
        vault: String,
        model: String,
        version: u32,
        timestamp: DateTime<Utc>,
    },
    PassphraseChanged {
        vault: String,
        files_reencrypted: usize,
        timestamp: DateTime<Utc>,
    },
    IntegrityFailed {
        vault: String,
        model: String,
        version: u32,
        expected: String,
        actual: String,
        timestamp: DateTime<Utc>,
    },
    ComplianceChecked {
        vault: String,
        passed: bool,
        timestamp: DateTime<Utc>,
    },
}

impl VaultEvent {
    /// Get the timestamp for any event variant.
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            VaultEvent::VaultCreated { timestamp, .. }
            | VaultEvent::VaultUnlocked { timestamp, .. }
            | VaultEvent::VaultLocked { timestamp, .. }
            | VaultEvent::ModelStored { timestamp, .. }
            | VaultEvent::ModelRetrieved { timestamp, .. }
            | VaultEvent::ModelDeleted { timestamp, .. }
            | VaultEvent::PassphraseChanged { timestamp, .. }
            | VaultEvent::IntegrityFailed { timestamp, .. }
            | VaultEvent::ComplianceChecked { timestamp, .. } => *timestamp,
        }
    }

    /// Get the vault name for any event variant.
    pub fn vault_name(&self) -> &str {
        match self {
            VaultEvent::VaultCreated { vault, .. }
            | VaultEvent::VaultUnlocked { vault, .. }
            | VaultEvent::VaultLocked { vault, .. }
            | VaultEvent::ModelStored { vault, .. }
            | VaultEvent::ModelRetrieved { vault, .. }
            | VaultEvent::ModelDeleted { vault, .. }
            | VaultEvent::PassphraseChanged { vault, .. }
            | VaultEvent::IntegrityFailed { vault, .. }
            | VaultEvent::ComplianceChecked { vault, .. } => vault,
        }
    }

    /// Get a human-readable event type name.
    pub fn event_type(&self) -> &'static str {
        match self {
            VaultEvent::VaultCreated { .. } => "vault_created",
            VaultEvent::VaultUnlocked { .. } => "vault_unlocked",
            VaultEvent::VaultLocked { .. } => "vault_locked",
            VaultEvent::ModelStored { .. } => "model_stored",
            VaultEvent::ModelRetrieved { .. } => "model_retrieved",
            VaultEvent::ModelDeleted { .. } => "model_deleted",
            VaultEvent::PassphraseChanged { .. } => "passphrase_changed",
            VaultEvent::IntegrityFailed { .. } => "integrity_failed",
            VaultEvent::ComplianceChecked { .. } => "compliance_checked",
        }
    }
}

impl fmt::Display for VaultEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}",
            self.timestamp().format("%Y-%m-%dT%H:%M:%SZ"),
            self.event_type()
        )
    }
}

/// Event subscriber receives vault events.
///
/// Subscribers are registered with `EventBus` and called for each event.
/// Errors from subscribers are logged but do NOT block the operation that emitted the event.
pub trait EventSubscriber: Send + Sync {
    /// Filter: return true to receive this event type.
    /// Default: accept all events.
    fn accepts(&self, _event: &VaultEvent) -> bool {
        true
    }

    /// Handle the event. Errors are logged but don't block the operation.
    fn on_event(&self, event: &VaultEvent) -> Result<()>;

    /// Human-readable name for logging.
    fn name(&self) -> &str;
}

/// Event bus dispatches events to registered subscribers.
pub struct EventBus {
    subscribers: Vec<Box<dyn EventSubscriber>>,
}

impl EventBus {
    /// Create a new event bus with no subscribers.
    pub fn new() -> Self {
        Self {
            subscribers: Vec::new(),
        }
    }

    /// Register a subscriber.
    pub fn subscribe(&mut self, subscriber: Box<dyn EventSubscriber>) {
        self.subscribers.push(subscriber);
    }

    /// Dispatch an event to all accepting subscribers.
    ///
    /// Errors from subscribers are collected and logged but do NOT propagate
    /// to the caller — audit/telemetry failures must never block vault operations.
    pub fn emit(&self, event: &VaultEvent) {
        for subscriber in &self.subscribers {
            if subscriber.accepts(event) {
                if let Err(e) = subscriber.on_event(event) {
                    eprintln!(
                        "[EventBus] subscriber '{}' failed on {}: {}",
                        subscriber.name(),
                        event.event_type(),
                        e
                    );
                }
            }
        }
    }

    /// Number of registered subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────────────────────
// Built-in Subscribers
// ──────────────────────────────────────────────────────────────

/// Forwards vault events to an `AuditSink` by converting them to `AuditEntry`.
pub struct AuditLogSubscriber {
    sink: Box<dyn AuditSink>,
}

impl AuditLogSubscriber {
    pub fn new(sink: Box<dyn AuditSink>) -> Self {
        Self { sink }
    }
}

impl EventSubscriber for AuditLogSubscriber {
    fn on_event(&self, event: &VaultEvent) -> Result<()> {
        use crate::audit::{AuditEntry, AuditEventType};

        let (event_type, description, model_name, version, success) = match event {
            VaultEvent::VaultCreated { vault, .. } => (
                AuditEventType::VaultCreated,
                format!("Vault '{}' created", vault),
                None,
                None,
                true,
            ),
            VaultEvent::VaultUnlocked { vault, .. } => (
                AuditEventType::AuthSuccess,
                format!("Vault '{}' unlocked", vault),
                None,
                None,
                true,
            ),
            VaultEvent::VaultLocked { vault, .. } => (
                AuditEventType::VaultOpened,
                format!("Vault '{}' locked", vault),
                None,
                None,
                true,
            ),
            VaultEvent::ModelStored {
                model, version: v, ..
            } => (
                AuditEventType::ModelStored,
                format!("Model '{}' version {} stored", model, v),
                Some(model.clone()),
                Some(*v),
                true,
            ),
            VaultEvent::ModelRetrieved {
                model, version: v, ..
            } => (
                AuditEventType::ModelRetrieved,
                format!("Model '{}' version {} retrieved", model, v),
                Some(model.clone()),
                Some(*v),
                true,
            ),
            VaultEvent::ModelDeleted {
                model, version: v, ..
            } => (
                AuditEventType::VersionDeleted,
                format!("Model '{}' version {} deleted", model, v),
                Some(model.clone()),
                Some(*v),
                true,
            ),
            VaultEvent::PassphraseChanged {
                files_reencrypted, ..
            } => (
                AuditEventType::ConfigChanged,
                format!(
                    "Passphrase changed, {} files re-encrypted",
                    files_reencrypted
                ),
                None,
                None,
                true,
            ),
            VaultEvent::IntegrityFailed {
                model,
                version: v,
                expected,
                actual,
                ..
            } => (
                AuditEventType::IntegrityFailure,
                format!(
                    "Integrity check failed for model '{}' version {}: expected={}, actual={}",
                    model, v, expected, actual
                ),
                Some(model.clone()),
                Some(*v),
                false,
            ),
            VaultEvent::ComplianceChecked { passed, .. } => (
                AuditEventType::SecurityViolation,
                format!(
                    "Compliance check: {}",
                    if *passed { "PASSED" } else { "FAILED" }
                ),
                None,
                None,
                *passed,
            ),
        };

        self.sink.emit(AuditEntry {
            timestamp: event.timestamp(),
            event_type,
            description,
            model_name,
            version,
            success,
            metadata: None,
        })
    }

    fn name(&self) -> &str {
        "AuditLogSubscriber"
    }
}

/// Collects vault metrics in memory for observability.
///
/// Agents can query these metrics via REST API, MCP, or CLI.
pub struct VaultMetrics {
    // Counters
    pub models_stored_total: std::sync::atomic::AtomicU64,
    pub models_retrieved_total: std::sync::atomic::AtomicU64,
    pub models_deleted_total: std::sync::atomic::AtomicU64,
    pub bytes_stored_total: std::sync::atomic::AtomicU64,
    pub errors_total: std::sync::atomic::AtomicU64,

    // Gauges (updated via events)
    pub vault_unlocked: std::sync::atomic::AtomicBool,
}

impl VaultMetrics {
    pub fn new() -> Self {
        Self {
            models_stored_total: std::sync::atomic::AtomicU64::new(0),
            models_retrieved_total: std::sync::atomic::AtomicU64::new(0),
            models_deleted_total: std::sync::atomic::AtomicU64::new(0),
            bytes_stored_total: std::sync::atomic::AtomicU64::new(0),
            errors_total: std::sync::atomic::AtomicU64::new(0),
            vault_unlocked: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Get a serializable snapshot of metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        use std::sync::atomic::Ordering::Relaxed;
        MetricsSnapshot {
            models_stored_total: self.models_stored_total.load(Relaxed),
            models_retrieved_total: self.models_retrieved_total.load(Relaxed),
            models_deleted_total: self.models_deleted_total.load(Relaxed),
            bytes_stored_total: self.bytes_stored_total.load(Relaxed),
            errors_total: self.errors_total.load(Relaxed),
            vault_unlocked: self.vault_unlocked.load(Relaxed),
        }
    }
}

impl Default for VaultMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Serializable snapshot of `VaultMetrics`.
#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    pub models_stored_total: u64,
    pub models_retrieved_total: u64,
    pub models_deleted_total: u64,
    pub bytes_stored_total: u64,
    pub errors_total: u64,
    pub vault_unlocked: bool,
}

/// Event subscriber that updates `VaultMetrics` counters.
pub struct MetricsSubscriber {
    metrics: std::sync::Arc<VaultMetrics>,
}

impl MetricsSubscriber {
    pub fn new(metrics: std::sync::Arc<VaultMetrics>) -> Self {
        Self { metrics }
    }
}

impl EventSubscriber for MetricsSubscriber {
    fn on_event(&self, event: &VaultEvent) -> Result<()> {
        use std::sync::atomic::Ordering::Relaxed;

        match event {
            VaultEvent::ModelStored { size, .. } => {
                self.metrics.models_stored_total.fetch_add(1, Relaxed);
                self.metrics.bytes_stored_total.fetch_add(*size, Relaxed);
            }
            VaultEvent::ModelRetrieved { .. } => {
                self.metrics.models_retrieved_total.fetch_add(1, Relaxed);
            }
            VaultEvent::ModelDeleted { .. } => {
                self.metrics.models_deleted_total.fetch_add(1, Relaxed);
            }
            VaultEvent::VaultUnlocked { .. } => {
                self.metrics.vault_unlocked.store(true, Relaxed);
            }
            VaultEvent::VaultLocked { .. } => {
                self.metrics.vault_unlocked.store(false, Relaxed);
            }
            VaultEvent::IntegrityFailed { .. } => {
                self.metrics.errors_total.fetch_add(1, Relaxed);
            }
            _ => {}
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "MetricsSubscriber"
    }
}

/// Null audit sink — discards all events. Used when audit logging is disabled.
pub struct NullAuditSink;

impl AuditSink for NullAuditSink {
    fn emit(&self, _entry: crate::audit::AuditEntry) -> Result<()> {
        Ok(())
    }

    fn query(&self, _limit: Option<usize>) -> Result<Vec<crate::audit::AuditEntry>> {
        Ok(Vec::new())
    }
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aimv_uri_root() {
        let uri = AimvUri::parse("aimv://").unwrap();
        assert_eq!(uri.vault, None);
        assert_eq!(uri.model, None);
        assert_eq!(uri.version, None);
        assert_eq!(uri.resource, None);
    }

    #[test]
    fn test_aimv_uri_vault() {
        let uri = AimvUri::parse("aimv://default/").unwrap();
        assert_eq!(uri.vault, Some("default".into()));
        assert_eq!(uri.model, None);
    }

    #[test]
    fn test_aimv_uri_model_latest() {
        let uri = AimvUri::parse("aimv://default/llama-3").unwrap();
        assert_eq!(uri.vault, Some("default".into()));
        assert_eq!(uri.model, Some("llama-3".into()));
        assert_eq!(uri.version, None);
    }

    #[test]
    fn test_aimv_uri_model_version() {
        let uri = AimvUri::parse("aimv://default/llama-3@3").unwrap();
        assert_eq!(uri.vault, Some("default".into()));
        assert_eq!(uri.model, Some("llama-3".into()));
        assert_eq!(uri.version, Some(3));
    }

    #[test]
    fn test_aimv_uri_with_resource() {
        let uri = AimvUri::parse("aimv://default/llama-3@3/card").unwrap();
        assert_eq!(uri.vault, Some("default".into()));
        assert_eq!(uri.model, Some("llama-3".into()));
        assert_eq!(uri.version, Some(3));
        assert_eq!(uri.resource, Some("card".into()));
    }

    #[test]
    fn test_aimv_uri_stats_resource() {
        let uri = AimvUri::parse("aimv://default/_stats").unwrap();
        assert_eq!(uri.vault, Some("default".into()));
        assert_eq!(uri.model, Some("_stats".into()));
    }

    #[test]
    fn test_aimv_uri_with_query() {
        let uri = AimvUri::parse("aimv://default/_events?since=2026-01-01&limit=100").unwrap();
        assert_eq!(uri.vault, Some("default".into()));
        assert_eq!(uri.model, Some("_events".into()));
        assert_eq!(uri.query.get("since"), Some(&"2026-01-01".into()));
        assert_eq!(uri.query.get("limit"), Some(&"100".into()));
    }

    #[test]
    fn test_aimv_uri_roundtrip() {
        let uri = AimvUri::parse("aimv://default/llama-3@3/card").unwrap();
        let s = uri.to_string();
        assert_eq!(s, "aimv://default/llama-3@3/card");
    }

    #[test]
    fn test_aimv_uri_invalid_scheme() {
        assert!(AimvUri::parse("http://default/").is_err());
    }

    #[test]
    fn test_event_bus() {
        use std::sync::{Arc, Mutex};

        struct TestSubscriber {
            events: Arc<Mutex<Vec<String>>>,
        }

        impl EventSubscriber for TestSubscriber {
            fn on_event(&self, event: &VaultEvent) -> Result<()> {
                self.events
                    .lock()
                    .unwrap()
                    .push(event.event_type().to_string());
                Ok(())
            }
            fn name(&self) -> &str {
                "TestSubscriber"
            }
        }

        let events = Arc::new(Mutex::new(Vec::new()));
        let mut bus = EventBus::new();
        bus.subscribe(Box::new(TestSubscriber {
            events: events.clone(),
        }));

        bus.emit(&VaultEvent::VaultCreated {
            vault: "test".into(),
            timestamp: Utc::now(),
        });
        bus.emit(&VaultEvent::ModelStored {
            vault: "test".into(),
            model: "m1".into(),
            version: 1,
            format: "pytorch".into(),
            size: 1024,
            checksum: "abc".into(),
            timestamp: Utc::now(),
        });

        let collected = events.lock().unwrap();
        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0], "vault_created");
        assert_eq!(collected[1], "model_stored");
    }

    #[test]
    fn test_metrics_subscriber() {
        let metrics = std::sync::Arc::new(VaultMetrics::new());
        let sub = MetricsSubscriber::new(metrics.clone());
        let bus_sub: Box<dyn EventSubscriber> = Box::new(sub);

        let mut bus = EventBus::new();
        bus.subscribe(bus_sub);

        bus.emit(&VaultEvent::ModelStored {
            vault: "test".into(),
            model: "m1".into(),
            version: 1,
            format: "pytorch".into(),
            size: 2048,
            checksum: "abc".into(),
            timestamp: Utc::now(),
        });
        bus.emit(&VaultEvent::ModelRetrieved {
            vault: "test".into(),
            model: "m1".into(),
            version: 1,
            timestamp: Utc::now(),
        });

        let snap = metrics.snapshot();
        assert_eq!(snap.models_stored_total, 1);
        assert_eq!(snap.models_retrieved_total, 1);
        assert_eq!(snap.bytes_stored_total, 2048);
    }

    #[test]
    fn test_vault_state_display() {
        let s = VaultState::Locked {
            vault_name: "default".into(),
            model_count: 5,
        };
        assert!(format!("{}", s).contains("default"));
    }

    #[test]
    fn test_aimv_uri_model_without_version_to_string() {
        // Covers line 194 — model without version in to_string()
        let uri = AimvUri {
            vault: Some("default".into()),
            model: Some("llama-3".into()),
            version: None,
            resource: None,
            query: HashMap::new(),
        };
        let s = uri.to_string();
        assert_eq!(s, "aimv://default/llama-3");
    }

    #[test]
    fn test_aimv_uri_with_empty_value_query() {
        // Covers line 217 — key-only query param (v.is_empty())
        let mut query = HashMap::new();
        query.insert("verbose".into(), String::new());
        let uri = AimvUri {
            vault: Some("default".into()),
            model: Some("test".into()),
            version: None,
            resource: None,
            query,
        };
        let s = uri.to_string();
        assert!(s.contains("?verbose"));
        assert!(!s.contains('='));
    }

    #[test]
    fn test_aimv_uri_display_trait() {
        let uri = AimvUri::parse("aimv://v1/m1@1").unwrap();
        let displayed = format!("{}", uri);
        assert_eq!(displayed, "aimv://v1/m1@1");
    }

    #[test]
    fn test_crypto_provider_hash_hex() {
        // Covers line 256 — hash_hex default implementation via FipsCrypto
        use crate::crypto::FipsCrypto;
        let crypto = FipsCrypto::new().unwrap();
        let hex_hash = crypto.hash_hex(b"hello");
        assert_eq!(hex_hash.len(), 64);
        assert!(hex_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_integrity_failed_event_through_audit_subscriber() {
        // Covers lines 602, 603 — IntegrityFailed arm in AuditLogSubscriber
        use std::sync::{Arc, Mutex};

        struct CollectingSink {
            entries: Arc<Mutex<Vec<crate::audit::AuditEntry>>>,
        }
        impl AuditSink for CollectingSink {
            fn emit(&self, entry: crate::audit::AuditEntry) -> Result<()> {
                self.entries.lock().unwrap().push(entry);
                Ok(())
            }
            fn query(&self, _limit: Option<usize>) -> Result<Vec<crate::audit::AuditEntry>> {
                Ok(self.entries.lock().unwrap().clone())
            }
        }

        let entries = Arc::new(Mutex::new(Vec::new()));
        let sink = CollectingSink {
            entries: entries.clone(),
        };
        let sub = AuditLogSubscriber::new(Box::new(sink));

        let mut bus = EventBus::new();
        bus.subscribe(Box::new(sub));

        bus.emit(&VaultEvent::IntegrityFailed {
            vault: "test".into(),
            model: "bad_model".into(),
            version: 1,
            expected: "abc123".into(),
            actual: "def456".into(),
            timestamp: Utc::now(),
        });

        let collected = entries.lock().unwrap();
        assert_eq!(collected.len(), 1);
        assert!(!collected[0].success);
        assert!(collected[0].description.contains("bad_model"));
    }

    #[test]
    fn test_metrics_subscriber_integrity_failed() {
        // Covers lines 860, 863 — IntegrityFailed arm in MetricsSubscriber
        let metrics = std::sync::Arc::new(VaultMetrics::new());
        let sub = MetricsSubscriber::new(metrics.clone());
        let mut bus = EventBus::new();
        bus.subscribe(Box::new(sub));

        bus.emit(&VaultEvent::IntegrityFailed {
            vault: "test".into(),
            model: "bad".into(),
            version: 1,
            expected: "a".into(),
            actual: "b".into(),
            timestamp: Utc::now(),
        });

        let snap = metrics.snapshot();
        assert_eq!(snap.errors_total, 1);
    }

    #[test]
    fn test_metrics_subscriber_lock_unlock() {
        let metrics = std::sync::Arc::new(VaultMetrics::new());
        let sub = MetricsSubscriber::new(metrics.clone());
        let mut bus = EventBus::new();
        bus.subscribe(Box::new(sub));

        bus.emit(&VaultEvent::VaultUnlocked {
            vault: "test".into(),
            timestamp: Utc::now(),
        });
        assert!(metrics.snapshot().vault_unlocked);

        bus.emit(&VaultEvent::VaultLocked {
            vault: "test".into(),
            timestamp: Utc::now(),
        });
        assert!(!metrics.snapshot().vault_unlocked);
    }

    #[test]
    fn test_metrics_subscriber_delete() {
        let metrics = std::sync::Arc::new(VaultMetrics::new());
        let sub = MetricsSubscriber::new(metrics.clone());
        let mut bus = EventBus::new();
        bus.subscribe(Box::new(sub));

        bus.emit(&VaultEvent::ModelDeleted {
            vault: "test".into(),
            model: "m1".into(),
            version: 1,
            timestamp: Utc::now(),
        });

        assert_eq!(metrics.snapshot().models_deleted_total, 1);
    }

    #[test]
    fn test_null_audit_sink() {
        let sink = NullAuditSink;
        let entry = crate::audit::AuditEntry {
            timestamp: Utc::now(),
            event_type: crate::audit::AuditEventType::VaultCreated,
            description: "test".into(),
            model_name: None,
            version: None,
            success: true,
            metadata: None,
        };
        assert!(sink.emit(entry).is_ok());
        assert!(sink.query(Some(10)).unwrap().is_empty());
    }

    #[test]
    fn test_audit_log_subscriber_all_event_types() {
        // Cover all AuditLogSubscriber match arms
        use std::sync::{Arc, Mutex};

        struct RecordSink {
            entries: Arc<Mutex<Vec<crate::audit::AuditEntry>>>,
        }
        impl AuditSink for RecordSink {
            fn emit(&self, entry: crate::audit::AuditEntry) -> Result<()> {
                self.entries.lock().unwrap().push(entry);
                Ok(())
            }
            fn query(&self, _: Option<usize>) -> Result<Vec<crate::audit::AuditEntry>> {
                Ok(Vec::new())
            }
        }

        let entries = Arc::new(Mutex::new(Vec::new()));
        let sink = RecordSink {
            entries: entries.clone(),
        };
        let sub = AuditLogSubscriber::new(Box::new(sink));
        let mut bus = EventBus::new();
        bus.subscribe(Box::new(sub));

        let now = Utc::now();
        // Emit all event variants
        bus.emit(&VaultEvent::VaultCreated {
            vault: "v".into(),
            timestamp: now,
        });
        bus.emit(&VaultEvent::VaultUnlocked {
            vault: "v".into(),
            timestamp: now,
        });
        bus.emit(&VaultEvent::VaultLocked {
            vault: "v".into(),
            timestamp: now,
        });
        bus.emit(&VaultEvent::ModelStored {
            vault: "v".into(),
            model: "m".into(),
            version: 1,
            format: "pt".into(),
            size: 100,
            checksum: "x".into(),
            timestamp: now,
        });
        bus.emit(&VaultEvent::ModelRetrieved {
            vault: "v".into(),
            model: "m".into(),
            version: 1,
            timestamp: now,
        });
        bus.emit(&VaultEvent::ModelDeleted {
            vault: "v".into(),
            model: "m".into(),
            version: 1,
            timestamp: now,
        });
        bus.emit(&VaultEvent::PassphraseChanged {
            vault: "v".into(),
            files_reencrypted: 3,
            timestamp: now,
        });
        bus.emit(&VaultEvent::ComplianceChecked {
            vault: "v".into(),
            passed: false,
            timestamp: now,
        });

        let collected = entries.lock().unwrap();
        assert_eq!(collected.len(), 8);
    }

    #[test]
    fn test_aimv_uri_to_string_with_version_and_query() {
        let uri = AimvUri {
            vault: Some("myvault".into()),
            model: Some("llama".into()),
            version: Some(3),
            resource: Some("weights".into()),
            query: {
                let mut m = std::collections::HashMap::new();
                m.insert("format".into(), "gguf".into());
                m.insert("flag".into(), String::new());
                m
            },
        };
        let s = uri.to_string();
        assert!(s.starts_with("aimv://"));
        assert!(s.contains("myvault"));
        assert!(s.contains("llama"));
        assert!(s.contains("@3"));
        assert!(s.contains("weights"));
        assert!(s.contains("format=gguf"));
        assert!(s.contains("flag"));
    }

    #[test]
    fn test_vault_state_display_all_variants() {
        let states = vec![
            (VaultState::Uninitialized, "Uninitialized"),
            (
                VaultState::Locked {
                    vault_name: "v1".into(),
                    model_count: 3,
                },
                "Locked(v1)",
            ),
            (
                VaultState::Unlocked {
                    vault_name: "v2".into(),
                    model_count: 5,
                    unlocked_at: Utc::now(),
                    operations_count: 10,
                },
                "Unlocked(v2)",
            ),
            (
                VaultState::Error {
                    message: "broken".into(),
                },
                "Error(broken)",
            ),
        ];
        for (state, expected) in &states {
            let display = format!("{}", state);
            assert_eq!(&display, expected);
        }
    }

    #[test]
    fn test_aimv_uri_too_many_segments() {
        let result = AimvUri::parse("aimv://vault/model/resource/extra");
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("too many path segments"));
    }

    #[test]
    fn test_aimv_uri_invalid_version_number() {
        let result = AimvUri::parse("aimv://vault/model@abc");
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Invalid version number"));
    }

    #[test]
    fn test_vault_event_timestamp_all_variants() {
        let now = Utc::now();
        let events = vec![
            VaultEvent::VaultCreated {
                vault: "v".into(),
                timestamp: now,
            },
            VaultEvent::VaultUnlocked {
                vault: "v".into(),
                timestamp: now,
            },
            VaultEvent::VaultLocked {
                vault: "v".into(),
                timestamp: now,
            },
            VaultEvent::ModelStored {
                vault: "v".into(),
                model: "m".into(),
                version: 1,
                format: "pt".into(),
                size: 100,
                checksum: "abc".into(),
                timestamp: now,
            },
            VaultEvent::ModelRetrieved {
                vault: "v".into(),
                model: "m".into(),
                version: 1,
                timestamp: now,
            },
            VaultEvent::ModelDeleted {
                vault: "v".into(),
                model: "m".into(),
                version: 1,
                timestamp: now,
            },
            VaultEvent::PassphraseChanged {
                vault: "v".into(),
                files_reencrypted: 3,
                timestamp: now,
            },
            VaultEvent::IntegrityFailed {
                vault: "v".into(),
                model: "m".into(),
                version: 1,
                expected: "a".into(),
                actual: "b".into(),
                timestamp: now,
            },
            VaultEvent::ComplianceChecked {
                vault: "v".into(),
                passed: true,
                timestamp: now,
            },
        ];
        for event in &events {
            assert_eq!(event.timestamp(), now);
            assert_eq!(event.vault_name(), "v");
            assert!(!event.event_type().is_empty());
        }
    }

    #[test]
    fn test_vault_event_display() {
        let event = VaultEvent::VaultCreated {
            vault: "test".into(),
            timestamp: Utc::now(),
        };
        let display = format!("{}", event);
        assert!(display.contains("vault_created"));
    }

    #[test]
    fn test_event_bus_error_handling() {
        struct FailingSubscriber;
        impl EventSubscriber for FailingSubscriber {
            fn on_event(&self, _event: &VaultEvent) -> Result<()> {
                Err(crate::error::VaultError::AuditError(
                    "subscriber failed".into(),
                ))
            }
            fn name(&self) -> &str {
                "FailingSubscriber"
            }
        }
        let mut bus = EventBus::new();
        bus.subscribe(Box::new(FailingSubscriber));
        assert_eq!(bus.subscriber_count(), 1);
        bus.emit(&VaultEvent::VaultCreated {
            vault: "test".into(),
            timestamp: Utc::now(),
        });
    }

    #[test]
    fn test_event_bus_selective_accepts() {
        use std::sync::{Arc, Mutex};
        struct SelectiveSubscriber {
            events: Arc<Mutex<Vec<String>>>,
        }
        impl EventSubscriber for SelectiveSubscriber {
            fn accepts(&self, event: &VaultEvent) -> bool {
                event.event_type() == "model_stored"
            }
            fn on_event(&self, event: &VaultEvent) -> Result<()> {
                self.events
                    .lock()
                    .unwrap()
                    .push(event.event_type().to_string());
                Ok(())
            }
            fn name(&self) -> &str {
                "SelectiveSubscriber"
            }
        }
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut bus = EventBus::new();
        bus.subscribe(Box::new(SelectiveSubscriber {
            events: events.clone(),
        }));
        bus.emit(&VaultEvent::VaultCreated {
            vault: "v".into(),
            timestamp: Utc::now(),
        });
        bus.emit(&VaultEvent::ModelStored {
            vault: "v".into(),
            model: "m".into(),
            version: 1,
            format: "pt".into(),
            size: 100,
            checksum: "abc".into(),
            timestamp: Utc::now(),
        });
        let collected = events.lock().unwrap();
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0], "model_stored");
    }

    #[test]
    fn test_subscriber_name_methods() {
        let sink = NullAuditSink;
        let audit_sub = AuditLogSubscriber::new(Box::new(sink));
        assert_eq!(audit_sub.name(), "AuditLogSubscriber");
        let metrics = std::sync::Arc::new(VaultMetrics::new());
        let metrics_sub = MetricsSubscriber::new(metrics);
        assert_eq!(metrics_sub.name(), "MetricsSubscriber");
    }

    #[test]
    fn test_metrics_subscriber_passphrase_changed() {
        let metrics = std::sync::Arc::new(VaultMetrics::new());
        let sub = MetricsSubscriber::new(metrics.clone());
        let mut bus = EventBus::new();
        bus.subscribe(Box::new(sub));
        bus.emit(&VaultEvent::PassphraseChanged {
            vault: "v".into(),
            files_reencrypted: 5,
            timestamp: Utc::now(),
        });
        bus.emit(&VaultEvent::ComplianceChecked {
            vault: "v".into(),
            passed: true,
            timestamp: Utc::now(),
        });
        let snap = metrics.snapshot();
        assert_eq!(snap.models_stored_total, 0);
    }

    #[test]
    fn test_vault_metrics_default() {
        let metrics = VaultMetrics::default();
        let snap = metrics.snapshot();
        assert_eq!(snap.models_stored_total, 0);
        assert!(!snap.vault_unlocked);
    }

    #[test]
    fn test_event_bus_default() {
        let bus = EventBus::default();
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn test_parse_query_empty_key() {
        // Covers L173 — parse_query filters out empty keys
        let uri = crate::traits::AimvUri::parse("aimv://vault/model?=value&good=yes").unwrap();
        // Empty key should be filtered out
        assert!(uri.query.contains_key("good"));
        assert!(!uri.query.contains_key(""));
    }

    #[test]
    fn test_vault_event_display_format() {
        // Covers L622 — Display impl for VaultEvent
        let event = VaultEvent::VaultUnlocked {
            vault: "test_vault".to_string(),
            timestamp: chrono::Utc::now(),
        };
        let display = format!("{}", event);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_async_blob_store_adapter() {
        // Covers L369, L376-414 — AsyncBlobStoreAdapter with LocalBackend
        use crate::storage::local::LocalBackend;
        use tokio::runtime::Runtime;

        let rt = Runtime::new().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let backend = LocalBackend::new(temp_dir.path().to_path_buf()).unwrap();
        let adapter = AsyncBlobStoreAdapter::new(backend);

        rt.block_on(async {
            // put
            let receipt = adapter.put("test_blob", b"hello blob").await.unwrap();
            assert_eq!(receipt.key, "test_blob");
            assert_eq!(receipt.size_bytes, 10);

            // get
            let data = adapter.get("test_blob").await.unwrap();
            assert_eq!(data, b"hello blob");

            // exists
            assert!(adapter.exists("test_blob").await.unwrap());

            // stat
            let info = adapter.stat("test_blob").await.unwrap();
            assert_eq!(info.key, "test_blob");
            assert_eq!(info.size_bytes, 10);

            // list
            let items = adapter.list(None).await.unwrap();
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].key, "test_blob");

            // delete
            let deleted = adapter.delete("test_blob").await.unwrap();
            assert!(deleted);

            // verify deleted
            assert!(!adapter.exists("test_blob").await.unwrap());
        });
    }

    #[test]
    fn test_event_subscriber_default_accepts() {
        // Test the default `accepts()` method which returns `true` for all events.
        struct AllAcceptor;
        impl crate::traits::EventSubscriber for AllAcceptor {
            fn on_event(&self, _event: &VaultEvent) -> crate::error::Result<()> {
                Ok(())
            }
            fn name(&self) -> &str {
                "all_acceptor"
            }
        }
        let sub = AllAcceptor;
        let event = VaultEvent::ModelStored {
            vault: "v".into(),
            model: "m".into(),
            version: 1,
            format: "safetensors".into(),
            size: 100,
            checksum: "abc".into(),
            timestamp: chrono::Utc::now(),
        };
        assert!(sub.accepts(&event));
    }
}
