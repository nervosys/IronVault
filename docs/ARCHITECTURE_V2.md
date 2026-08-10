# IronVault — Architecture v2

_A first-principles redesign for efficiency, capability, and agent observability._

**Date**: 2026-02-22
**Status**: Proposed
**Supersedes**: [ARCHITECTURE.md](ARCHITECTURE.md) (retained as historical reference)

---

## 1. First-Principles Analysis

### 1.1 Core Problem

AI model artifacts are high-value intellectual property that need **secure lifecycle management**: encryption at rest, integrity verification, version lineage, format portability, regulatory compliance, and distributed replication — all while remaining **observable and actionable by autonomous agents**.

### 1.2 Fundamental Invariants

These properties must hold at ALL times, in ALL code paths:

| #   | Invariant                                                            | Enforcement                                                |
| --- | -------------------------------------------------------------------- | ---------------------------------------------------------- |
| I1  | **Confidentiality**: Plaintext model data never persists on disk     | Encrypt-before-write; zeroize all buffers                  |
| I2  | **Integrity**: Every retrieval verifies SHA-256 checksum             | Checksum computed pre-encryption, verified post-decryption |
| I3  | **Auditability**: Every state-changing operation emits an event      | Event bus with guaranteed delivery to audit sink           |
| I4  | **Immutability**: Stored versions are never mutated in-place         | Append-only version log; deletes produce tombstones        |
| I5  | **Addressability**: Every entity has a stable URI                    | `aimv://{vault}/{model}@{version}`                         |
| I6  | **Recoverability**: Vault state is reconstructable from on-disk data | Version manifest + encrypted blobs = complete state        |

### 1.3 Domain Concepts (from primitives)

```
Artifact   := an immutable byte sequence with a content address (SHA-256)
Identity   := (vault_name, model_name, version) → unique reference
Envelope   := Encrypt(Compress(Artifact), key) + nonce + auth_tag
Lineage    := DAG of Identity → parent Identity relationships
Vault      := namespace + encryption key scope + policy set
Policy     := retention rules, encryption params, compliance requirements
Event      := (timestamp, event_type, identity?, payload) — immutable record
Capability := permission to perform an operation on a resource scope
```

---

## 2. Current Architecture Assessment

### 2.1 What Works Well

| Aspect            | Rating | Notes                                                                 |
| ----------------- | ------ | --------------------------------------------------------------------- |
| Crypto primitives | A      | AES-256-GCM, Argon2id, SHA-256 — well-chosen, correctly parameterized |
| Format detection  | A      | 23+ formats with extension-based and magic-byte detection             |
| Compliance checks | B+     | FIPS/CMMC/MITRE framework coverage is comprehensive                   |
| Feature gating    | B+     | Optional features keep binary size manageable                         |
| XDG compliance    | A      | Proper cross-platform directory layout                                |
| CLI design        | B+     | Clean clap-based commands, good ergonomics                            |

### 2.2 Architectural Debt

| Issue                                                                                                          | Severity | Impact                                                           |
| -------------------------------------------------------------------------------------------------------------- | -------- | ---------------------------------------------------------------- |
| **God Object** — `Vault` owns config, storage, version control, audit, crypto, key manager, active key         | Critical | Cannot test, extend, or compose subsystems independently         |
| **Sync/Async Mismatch** — Core vault is sync, cloud backends are async                                         | High     | Cannot unify local and cloud storage behind one trait            |
| **Fake Streaming** — `store_model_streamed` collects all chunks into a `Vec<u8>`, then encrypts monolithically | High     | Multi-GB models require full in-memory buffering                 |
| **Single-file Version DB** — `versions.json` holds ALL metadata for ALL models in one JSON file                | High     | No concurrent access, no indexing, O(n) lookups, corruption risk |
| **No Event System** — Audit logging via inline `if let Some(logger)` checks                                    | Medium   | No hooks, no subscriptions, no agent reactivity                  |
| **No Trait Boundaries** — `Vault`, `Storage`, `VersionControl` are concrete structs                            | Medium   | No mocking, no middleware, no alternative implementations        |
| **Scattered Crypto** — Both `Vault` and `Storage` instantiate `FipsCrypto`                                     | Medium   | Key management responsibility is split and unclear               |
| **No Observability** — No metrics, no structured spans, no agent-queryable state                               | Medium   | Agents cannot introspect operations or performance               |
| **Stringly-Typed Errors** — Most errors are `VaultError::SomeVariant(String)`                                  | Low      | Programmatic error handling is difficult for agents              |

### 2.3 Missing Capabilities

- True streaming encryption/decryption for large models
- Pluggable format converter registry
- Event-driven hooks for agent integration
- Resource-based access control (beyond passphrase unlock)
- Structured operation results (not just `Result<T>`)
- Content-addressable storage (dedup at the storage layer)
- Transaction semantics for multi-step operations
- Health/readiness endpoints for API server

---

## 3. Target Architecture

### 3.1 Layered Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│ Layer 5: INTERFACE                                              │
│ ┌─────────┐ ┌──────────┐ ┌─────────┐ ┌─────┐ ┌──────────────┐ │
│ │   CLI   │ │ REST API │ │ GraphQL │ │ MCP │ │ Python (PyO3) │ │
│ └────┬────┘ └────┬─────┘ └────┬────┘ └──┬──┘ └──────┬───────┘ │
├──────┼───────────┼────────────┼─────────┼──────────┬┼──────────┤
│ Layer 4: APPLICATION (use cases + orchestration)   ││          │
│ ┌────┴───────────┴────────────┴─────────┴──────────┘│          │
│ │ VaultService  ConvertService  RAGService  CloudService       │
│ │   ┌─────────────────────────────────────────────┐            │
│ │   │           EventDispatcher                   │            │
│ │   │  (audit, telemetry, hooks, agent webhooks)  │            │
│ │   └─────────────────────────────────────────────┘            │
│ └──────────────────────────────────────────────────────────────┘│
├────────────────────────────────────────────────────────────────┤
│ Layer 3: DOMAIN (entities, value objects, domain services)      │
│ ┌──────────┐ ┌──────────┐ ┌────────┐ ┌──────────┐ ┌────────┐  │
│ │ Artifact │ │ Identity │ │ Policy │ │ Lineage  │ │ Event  │  │
│ └──────────┘ └──────────┘ └────────┘ └──────────┘ └────────┘  │
│ ┌──────────────────┐ ┌────────────────────┐ ┌───────────────┐  │
│ │ ModelFormat (23+) │ │  ModelCard          │ │ Compliance    │  │
│ └──────────────────┘ └────────────────────┘ └───────────────┘  │
├────────────────────────────────────────────────────────────────┤
│ Layer 2: INFRASTRUCTURE (implementations behind traits)        │
│ ┌────────────────┐ ┌──────────────────┐ ┌────────────────────┐ │
│ │ CryptoProvider │ │ StorageProvider  │ │ VersionRepository  │ │
│ │  ├ Aes256Gcm   │ │  ├ LocalFS       │ │  ├ JsonFile        │ │
│ │  └ (pluggable) │ │  ├ S3            │ │  ├ SQLite          │ │
│ │                │ │  ├ Azure         │ │  └ (pluggable)     │ │
│ │                │ │  └ GCS           │ │                    │ │
│ └────────────────┘ └──────────────────┘ └────────────────────┘ │
│ ┌────────────────┐ ┌──────────────────┐ ┌────────────────────┐ │
│ │ AuditSink      │ │ DocumentStore    │ │ VectorStore        │ │
│ │  ├ FileAppend   │ │  ├ InMemory      │ │  ├ SimpleVec       │ │
│ │  ├ Blockchain   │ │  ├ SQLite        │ │  ├ Qdrant          │ │
│ │  └ (pluggable) │ │  └ Sled          │ │  └ LanceDB         │ │
│ └────────────────┘ └──────────────────┘ └────────────────────┘ │
├────────────────────────────────────────────────────────────────┤
│ Layer 1: PLATFORM (OS abstractions)                            │
│ ┌──────────┐ ┌────────────┐ ┌──────────────┐ ┌──────────────┐ │
│ │ XDG Dirs │ │ File Perms │ │ Secure Alloc │ │ OS Keyring   │ │
│ └──────────┘ └────────────┘ └──────────────┘ └──────────────┘ │
└────────────────────────────────────────────────────────────────┘
```

### 3.2 Core Trait Hierarchy

Every major subsystem is defined by a trait. Implementations are injected, never hard-coded.

```rust
// ── Layer 3: Domain Traits ───────────────────────────────────

/// Core vault operations. All implementations must uphold invariants I1–I6.
#[async_trait]
pub trait VaultOps: Send + Sync {
    async fn store(&self, id: &ModelId, data: DataStream, meta: ModelMetadata) -> Result<ModelVersion>;
    async fn get(&self, id: &ModelId, version: Option<u32>) -> Result<DataStream>;
    async fn delete(&self, id: &ModelId, version: u32) -> Result<()>;
    async fn list(&self) -> Result<Vec<ModelSummary>>;
    async fn versions(&self, model: &str) -> Result<Vec<ModelVersion>>;
    async fn lineage(&self, id: &ModelId, version: u32) -> Result<LineageGraph>;
    fn state(&self) -> VaultState;  // Observable state for agents
}

/// Typed data stream for true streaming I/O.
pub struct DataStream {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>,
    total_size: Option<u64>,
}

// ── Layer 2: Infrastructure Traits ───────────────────────────

/// Crypto operations — encrypt, decrypt, hash, derive keys.
pub trait CryptoProvider: Send + Sync {
    fn derive_key(&self, passphrase: &[u8], salt: Option<&[u8]>) -> Result<(SecureKey, Vec<u8>)>;
    fn encrypt_stream(&self, input: DataStream, key: &SecureKey) -> Result<DataStream>;
    fn decrypt_stream(&self, input: DataStream, key: &SecureKey) -> Result<DataStream>;
    fn hash(&self, data: &[u8]) -> Hash;
}

/// Blob storage — content-addressable, backend-agnostic.
#[async_trait]
pub trait BlobStore: Send + Sync {
    async fn put(&self, key: &str, data: DataStream) -> Result<BlobReceipt>;
    async fn get(&self, key: &str) -> Result<DataStream>;
    async fn delete(&self, key: &str) -> Result<bool>;
    async fn exists(&self, key: &str) -> Result<bool>;
    async fn list(&self, prefix: Option<&str>) -> Result<Vec<BlobInfo>>;
    async fn stat(&self, key: &str) -> Result<BlobInfo>;
}

/// Version metadata repository.
#[async_trait]
pub trait VersionRepo: Send + Sync {
    async fn add(&self, model: &str, version: ModelVersion) -> Result<()>;
    async fn get(&self, model: &str, version: Option<u32>) -> Result<Option<ModelVersion>>;
    async fn list(&self, model: &str) -> Result<Vec<ModelVersion>>;
    async fn delete(&self, model: &str, version: u32) -> Result<bool>;
    async fn search(&self, query: &VersionQuery) -> Result<Vec<ModelVersion>>;
}

/// Audit event sink.
#[async_trait]
pub trait AuditSink: Send + Sync {
    async fn emit(&self, event: AuditEvent) -> Result<()>;
    async fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditEvent>>;
}
```

### 3.3 Event-Driven Core

Every state-changing operation emits a typed event. No more `if let Some(logger)` checks.

```rust
/// Domain events — the canonical record of "what happened."
#[derive(Debug, Clone, Serialize)]
pub enum VaultEvent {
    VaultCreated   { vault: String, timestamp: DateTime<Utc> },
    VaultUnlocked  { vault: String, timestamp: DateTime<Utc> },
    VaultLocked    { vault: String, timestamp: DateTime<Utc> },
    ModelStored    { vault: String, model: String, version: u32, format: ModelFormat, size: u64, checksum: Hash, timestamp: DateTime<Utc> },
    ModelRetrieved { vault: String, model: String, version: u32, timestamp: DateTime<Utc> },
    ModelDeleted   { vault: String, model: String, version: u32, timestamp: DateTime<Utc> },
    ModelConverted { vault: String, model: String, from: ModelFormat, to: ModelFormat, timestamp: DateTime<Utc> },
    ComplianceChecked { vault: String, result: ComplianceStatus, timestamp: DateTime<Utc> },
    IntegrityFailed   { vault: String, model: String, version: u32, expected: Hash, actual: Hash, timestamp: DateTime<Utc> },
    CloudSynced    { vault: String, model: String, provider: String, direction: SyncDirection, timestamp: DateTime<Utc> },
}

/// Event dispatcher with pluggable subscribers.
pub struct EventBus {
    subscribers: Vec<Box<dyn EventSubscriber>>,
}

#[async_trait]
pub trait EventSubscriber: Send + Sync {
    /// Filter: return true to receive this event type.
    fn accepts(&self, event: &VaultEvent) -> bool { true }
    /// Handle the event. Errors are logged but don't block the operation.
    async fn on_event(&self, event: &VaultEvent) -> Result<()>;
}

// Built-in subscribers:
// - AuditLogSubscriber   → writes to append-only audit log
// - BlockchainSubscriber → appends to Merkle-chained block
// - TelemetrySubscriber  → emits OpenTelemetry spans/metrics
// - WebhookSubscriber    → POSTs events to external URLs (for agents)
// - MetricsSubscriber    → updates in-memory counters (for /metrics endpoint)
```

### 3.4 Vault State Machine

The vault transitions through typed states. Agents can query the current state.

```
                    ┌──────────────┐
         init()     │              │
        ────────►   │  Initialized │
                    │  (no key)    │
                    └──────┬───────┘
                           │ unlock(passphrase)
                           ▼
                    ┌──────────────┐
                    │              │  store() / get() / delete()
                    │   Unlocked   │◄──────────────────────────┐
                    │  (key active)│────────────────────────────┘
                    └──────┬───────┘
                           │ lock() / timeout
                           ▼
                    ┌──────────────┐
                    │              │
                    │    Locked    │
                    │  (key zeroed)│
                    └──────┬───────┘
                           │ unlock(passphrase)
                           ▼
                    ┌──────────────┐
                    │   Unlocked   │
                    └──────────────┘
```

```rust
/// Observable vault state — agents can query this at any time.
#[derive(Debug, Clone, Serialize)]
pub enum VaultState {
    Uninitialized,
    Initialized { vault_name: String, created_at: DateTime<Utc> },
    Locked { vault_name: String, model_count: usize },
    Unlocked {
        vault_name: String,
        model_count: usize,
        unlocked_at: DateTime<Utc>,
        session_timeout: Duration,
        operations_count: u64,
    },
    Error { message: String },
}
```

### 3.5 Resource Addressing

Every entity has a URI. Agents use URIs to refer to any object.

```
aimv://                                  — root (list vaults)
aimv://default/                          — vault "default"
aimv://default/llama-3                   — model "llama-3" (latest version)
aimv://default/llama-3@3                 — model "llama-3" version 3
aimv://default/llama-3@3/card            — model card for version 3
aimv://default/llama-3@3/lineage         — version lineage graph
aimv://default/_compliance               — compliance report
aimv://default/_stats                    — vault statistics
aimv://default/_events?since=2026-01-01  — event log (filtered)
aimv://rag/{kb_name}/documents           — RAG document list
aimv://rag/{kb_name}/search?q=attention  — RAG search
```

```rust
/// Parsed AIMV URI with typed components.
#[derive(Debug, Clone, PartialEq)]
pub struct AimvUri {
    pub vault: Option<String>,
    pub model: Option<String>,
    pub version: Option<u32>,
    pub resource: Option<String>,  // "card", "lineage", "_stats", etc.
    pub query: HashMap<String, String>,
}

impl AimvUri {
    pub fn parse(uri: &str) -> Result<Self> { /* ... */ }
}
```

---

## 4. Subsystem Redesigns

### 4.1 Streaming Crypto Pipeline

**Problem**: Current implementation buffers entire models in memory.

**Solution**: Chunk-based streaming with authenticated encryption.

```rust
/// Streaming encryption: process data in fixed-size chunks.
/// Each chunk gets its own nonce (derived from base_nonce + chunk_index).
/// Final chunk includes a stream-authentication tag over all chunk MACs.
///
/// Wire format:
///   [header: 32 bytes]
///   [chunk_0: nonce(12) | ciphertext(chunk_size) | tag(16)]
///   [chunk_1: nonce(12) | ciphertext(chunk_size) | tag(16)]
///   ...
///   [chunk_n: nonce(12) | ciphertext(remaining) | tag(16)]
///   [stream_mac: 32 bytes]  ← SHA-256 over all chunk tags + chunk count
///
/// Chunk size default: 4 MiB (tuned for SSD page alignment)
const DEFAULT_CHUNK_SIZE: usize = 4 * 1024 * 1024;

impl FipsCrypto {
    pub fn encrypt_stream(
        &self,
        reader: impl AsyncRead + Unpin,
        key: &SecureKey,
        chunk_size: usize,
    ) -> impl Stream<Item = Result<Bytes>> {
        // Yields encrypted chunks as a stream — never holds full model in memory
    }

    pub fn decrypt_stream(
        &self,
        reader: impl AsyncRead + Unpin,
        key: &SecureKey,
    ) -> impl Stream<Item = Result<Bytes>> {
        // Reads and decrypts chunk-by-chunk, verifies stream MAC at end
    }
}
```

**Memory budget**: For a 70B parameter model (~140 GB in FP16), the old architecture required ~140 GB of RAM. The new architecture requires only `chunk_size` (4 MiB) + compression buffer (4 MiB) = **8 MiB regardless of model size**.

### 4.2 Unified Storage Abstraction

**Problem**: `Storage` (sync, local-only) and `StorageBackend` (async, cloud) are incompatible.

**Solution**: Single `BlobStore` trait, all async, with a sync wrapper for CLI.

```rust
/// Unified blob store — every backend implements this single trait.
#[async_trait]
pub trait BlobStore: Send + Sync {
    async fn put(&self, key: &str, data: DataStream) -> Result<BlobReceipt>;
    async fn get(&self, key: &str) -> Result<DataStream>;
    async fn delete(&self, key: &str) -> Result<bool>;
    async fn exists(&self, key: &str) -> Result<bool>;
    async fn list(&self, prefix: Option<&str>) -> Result<Vec<BlobInfo>>;
    async fn stat(&self, key: &str) -> Result<BlobInfo>;
}

/// Receipt returned after a successful put — includes content hash for dedup.
#[derive(Debug, Clone)]
pub struct BlobReceipt {
    pub key: String,
    pub content_hash: Hash,
    pub size_bytes: u64,
    pub stored_at: DateTime<Utc>,
}

// Implementations:
// - LocalBlobStore  (wraps tokio::fs for async file I/O)
// - S3BlobStore     (wraps aws-sdk-s3)
// - AzureBlobStore  (wraps azure_storage_blobs)
// - GcsBlobStore    (wraps google-cloud-storage)
// - MemoryBlobStore (for testing — stores in HashMap)
//
// Middleware (decorator pattern):
// - CachingBlobStore     — LRU cache in front of any backend
// - MeteredBlobStore     — emits metrics (bytes transferred, latency)
// - RetryBlobStore       — retry with exponential backoff
// - EncryptingBlobStore  — transparent encryption layer
```

### 4.3 Version Repository

**Problem**: `versions.json` is a single file loaded entirely into memory.

**Solution**: Trait-based repository with pluggable backends.

```rust
#[async_trait]
pub trait VersionRepo: Send + Sync {
    async fn add_version(&self, model: &str, version: ModelVersion) -> Result<()>;
    async fn get_version(&self, model: &str, version: Option<u32>) -> Result<Option<ModelVersion>>;
    async fn list_versions(&self, model: &str) -> Result<Vec<ModelVersion>>;
    async fn list_models(&self) -> Result<Vec<ModelSummary>>;
    async fn delete_version(&self, model: &str, version: u32) -> Result<bool>;
    async fn get_lineage(&self, model: &str, version: u32) -> Result<Vec<ModelVersion>>;
    async fn search(&self, query: &VersionQuery) -> Result<Vec<ModelVersion>>;
    async fn verify_checksum(&self, model: &str, version: u32, hash: &Hash) -> Result<bool>;
}

// Implementations:
// - JsonFileRepo     (backward-compatible migration from versions.json)
// - SqliteRepo       (indexed, concurrent, ACID — recommended default)
// - MemoryRepo       (for testing)
```

### 4.4 Observability Layer

```rust
/// Metrics that agents can query via REST API, MCP, or CLI.
#[derive(Debug, Clone, Serialize)]
pub struct VaultMetrics {
    // Counters
    pub models_stored_total: u64,
    pub models_retrieved_total: u64,
    pub bytes_encrypted_total: u64,
    pub bytes_decrypted_total: u64,
    pub conversions_total: u64,
    pub errors_total: u64,

    // Gauges
    pub models_count: usize,
    pub versions_count: usize,
    pub storage_bytes: u64,
    pub vault_state: VaultState,
    pub uptime_seconds: u64,

    // Histograms (recent operations)
    pub store_latency_p50_ms: f64,
    pub store_latency_p99_ms: f64,
    pub retrieve_latency_p50_ms: f64,
    pub retrieve_latency_p99_ms: f64,

    // Health
    pub healthy: bool,
    pub last_error: Option<String>,
    pub compliance_status: Option<ComplianceStatus>,
}
```

### 4.5 Agent Integration Architecture

Agents interact with AIMV through four surfaces:

```
┌──────────────────────────────────────────────────────────┐
│                    Agent Surfaces                         │
│                                                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐  │
│  │  MCP Tools  │  │  REST API   │  │  Event Webhooks │  │
│  │  (13 tools) │  │  (OpenAPI)  │  │  (push)         │  │
│  └──────┬──────┘  └──────┬──────┘  └────────┬────────┘  │
│         │                │                   │           │
│  ┌──────┴────────────────┴───────────────────┴────────┐  │
│  │              Observability Layer                    │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────────┐   │  │
│  │  │ Metrics  │ │ Events   │ │ State Machine    │   │  │
│  │  │ (query)  │ │ (stream) │ │ (introspect)     │   │  │
│  │  └──────────┘ └──────────┘ └──────────────────┘   │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │              Ontology (this document)               │  │
│  │  Classes · States · Events · Affordances · URIs    │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

**Key principle**: An agent can fully understand AIMV without reading source code — the ontology describes what exists, what state it's in, what actions are possible, and what effects those actions have.

---

## 5. Module Restructure

### 5.1 Proposed Source Layout

```
src/
├── lib.rs                    # Public API surface + re-exports
├── main.rs                   # CLI entry point
│
├── domain/                   # Layer 3: Pure domain logic (no I/O)
│   ├── mod.rs
│   ├── artifact.rs           # Artifact, Hash, content addressing
│   ├── identity.rs           # ModelId, AimvUri, resource addressing
│   ├── version.rs            # ModelVersion, LineageGraph (value objects)
│   ├── format.rs             # ModelFormat enum + detection
│   ├── metadata.rs           # ModelMetadata, ModelCard
│   ├── policy.rs             # RetentionPolicy, EncryptionPolicy
│   ├── event.rs              # VaultEvent enum + EventBus trait
│   ├── state.rs              # VaultState state machine
│   └── error.rs              # Structured error types
│
├── service/                  # Layer 4: Application services (orchestration)
│   ├── mod.rs
│   ├── vault_service.rs      # Store/get/delete orchestration + events
│   ├── convert_service.rs    # Format conversion pipeline
│   ├── cloud_service.rs      # Cloud push/pull orchestration
│   ├── rag_service.rs        # RAG operations
│   ├── compliance_service.rs # Compliance checking
│   └── federation_service.rs # Federated sync
│
├── infra/                    # Layer 2: Infrastructure implementations
│   ├── mod.rs
│   ├── crypto/
│   │   ├── mod.rs            # CryptoProvider trait
│   │   ├── aes_gcm.rs        # AES-256-GCM implementation
│   │   ├── streaming.rs      # Chunk-based streaming encrypt/decrypt
│   │   ├── key_management.rs # Key derivation, rotation, zeroize
│   │   └── compression.rs    # Gzip, LZMA, Zstd
│   ├── storage/
│   │   ├── mod.rs            # BlobStore trait
│   │   ├── local.rs          # Async local filesystem
│   │   ├── s3.rs             # AWS S3
│   │   ├── azure.rs          # Azure Blob
│   │   ├── gcs.rs            # Google Cloud Storage
│   │   ├── memory.rs         # In-memory (testing)
│   │   └── middleware.rs     # Caching, metering, retry decorators
│   ├── version_repo/
│   │   ├── mod.rs            # VersionRepo trait
│   │   ├── json_file.rs      # Legacy JSON file backend
│   │   └── sqlite.rs         # SQLite backend (recommended)
│   ├── audit/
│   │   ├── mod.rs            # AuditSink trait
│   │   ├── file_sink.rs      # Append-only file
│   │   └── blockchain.rs     # Merkle-chained blocks
│   ├── rag/
│   │   ├── mod.rs
│   │   ├── document_store.rs
│   │   ├── knowledge_base.rs
│   │   ├── rule_engine.rs
│   │   ├── vector_store.rs
│   │   └── mcp_server.rs
│   └── observability/
│       ├── mod.rs
│       ├── metrics.rs        # In-memory metrics collector
│       ├── telemetry.rs      # OpenTelemetry integration
│       └── event_bus.rs      # EventBus implementation + subscribers
│
├── interface/                # Layer 5: External interfaces
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── args.rs           # Clap definitions
│   │   └── handlers/         # Command handlers
│   ├── api/
│   │   ├── mod.rs
│   │   ├── server.rs         # Axum server setup
│   │   ├── routes.rs         # REST endpoints
│   │   ├── graphql.rs        # GraphQL schema
│   │   ├── auth.rs           # JWT middleware
│   │   └── openapi.rs        # OpenAPI spec generation
│   ├── mcp/
│   │   ├── mod.rs
│   │   └── tools.rs          # MCP tool implementations
│   └── python/
│       └── mod.rs            # PyO3 bindings
│
└── platform/                 # Layer 1: OS abstractions
    ├── mod.rs
    ├── dirs.rs               # XDG directory resolution
    ├── permissions.rs        # File permission management
    └── keyring.rs            # OS keyring integration
```

### 5.2 Dependency Rules

```
interface → service → domain ← infra
                         ↑
                      platform
```

- **domain** has ZERO external dependencies (only std)
- **interface** never imports from **infra** directly
- **service** depends on traits defined in **domain**, implemented in **infra**
- **infra** depends on **domain** types and external crates
- **platform** depends on OS-specific crates only

---

## 6. Migration Path

### Phase 1: Trait Extraction (non-breaking)

1. Extract `CryptoProvider` trait from `FipsCrypto`
2. Extract `BlobStore` trait from `Storage`
3. Extract `VersionRepo` trait from `VersionControl`
4. Extract `AuditSink` trait from `AuditLogger`
5. `Vault` takes `dyn Trait` references instead of concrete types
6. All existing tests continue to pass

### Phase 2: Event System

1. Define `VaultEvent` enum
2. Implement `EventBus` with subscriber registration
3. Replace inline audit logging with event emission
4. Add `AuditLogSubscriber` as first subscriber
5. Add `MetricsSubscriber` for observability

### Phase 3: Async Unification

1. Make all `BlobStore` methods async
2. Wrap local filesystem with `tokio::fs`
3. Unify cloud backends under the same `BlobStore` trait
4. Add sync wrappers for CLI (`tokio::runtime::Runtime::block_on`)

### Phase 4: Streaming Encryption

1. Implement chunked `encrypt_stream` / `decrypt_stream`
2. Define wire format for chunked encrypted files
3. Add backward-compatibility reader for monolithic `.vault` files
4. Migrate `store_model` to use streaming pipeline

### Phase 5: Repository & Observability

1. Implement `SqliteVersionRepo`
2. Auto-migrate from `versions.json` on first access
3. Add `/metrics`, `/health`, `/events` API endpoints
4. Implement `AimvUri` parser
5. Publish comprehensive ontology

---

## 7. Performance Targets

| Operation                | Current          | Target          | Method                            |
| ------------------------ | ---------------- | --------------- | --------------------------------- |
| Store 1 GB model         | ~8s (buffer all) | ~3s (streaming) | Chunk-based encrypt + async write |
| Retrieve 1 GB model      | ~6s (buffer all) | ~2s (streaming) | Async read + chunk decrypt        |
| Store 70B model (140 GB) | OOM              | ~5 min          | Streaming — 8 MiB memory budget   |
| List 10,000 models       | ~2s (JSON parse) | ~5ms            | SQLite indexed query              |
| Version lookup           | O(n) JSON scan   | O(1)            | SQLite primary key                |
| Concurrent API requests  | 1 (mutex)        | 1000+           | Async + connection pool           |

---

## 8. Security Considerations

### 8.1 Streaming Encryption Security

The chunked format maintains the same security guarantees as monolithic encryption:
- Each chunk has a unique nonce (base_nonce ⊕ chunk_index)
- Each chunk has its own AES-GCM authentication tag
- A stream MAC prevents chunk reordering, truncation, or extension
- The header includes format version for forward compatibility

### 8.2 Key Management Evolution

Current: Passphrase → Argon2id → single key for all operations.

Future (non-breaking extension):
```
Passphrase → Argon2id → Master Key
                           ├─ HKDF("encrypt") → Encryption Key
                           ├─ HKDF("signing") → Signing Key
                           └─ HKDF("audit")   → Audit Key
```

This separation allows:
- Encryption key rotation without re-encrypting everything
- Audit log signing with a separate key
- Future multi-user access with per-user derived keys

---

## Appendix A: Decision Records

| Decision       | Choice                   | Rationale                                                             |
| -------------- | ------------------------ | --------------------------------------------------------------------- |
| Async runtime  | Tokio                    | Already used; largest ecosystem; required by cloud SDKs               |
| Version DB     | SQLite                   | ACID, indexed, battle-tested, already a dependency (`rusqlite`)       |
| Chunk size     | 4 MiB                    | Aligns with SSD page sizes, good compression ratio, reasonable memory |
| Event delivery | Best-effort (log errors) | Audit subscribers must not block crypto operations                    |
| URI scheme     | `aimv://`                | Short, unique, follows RFC 3986                                       |
| State machine  | Enum-based               | Compile-time safety, exhaustive matching, serializable                |

## Appendix B: Ontology Reference

See [`.well-known/ontology.jsonld`](https://github.com/nervosys/IronVault/blob/master/.well-known/ontology.jsonld) for the full machine-readable ontology that makes this architecture observable by agents.
