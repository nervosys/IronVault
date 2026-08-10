//! IronVault - Universal secure vault for AI model formats
//!
//! A cross-platform, XDG-compliant secure storage system for Neural and Neurosymbolic AI models with:
//! - FIPS 140-3 compliant encryption
//! - CVE scanning and compliance
//! - MITRE ATT&CK framework alignment
//! - CMMC 2.0 compliance
//! - Version control with complete checkpoint history
//! - Format conversion capabilities

pub mod access_control;
#[cfg(feature = "api")]
pub mod api;
pub mod audit;
pub mod benchmark;
pub mod blockchain;
pub mod cloud_envelope;
pub mod compliance;
pub mod config;
pub mod conversion;
pub mod crypto;
pub mod diff;
pub mod download;
pub mod env;
pub mod error;
pub mod evaluation;
pub mod federation;
pub mod federation_transport;
pub mod formats;
pub mod gc;
pub mod gguf;
pub mod interop;
pub mod kms;
pub mod license_scan;
pub mod lineage_graph;
pub mod model_card;
pub mod multi_vault;
pub mod permissions;
pub mod plugins;
pub mod policies;
pub mod profiles;
pub mod quantization;
pub mod rag;
pub mod scanning;
pub mod scheduler;
pub mod signing;
pub mod storage;
pub mod tags;
pub mod telemetry;
/// OTLP export transport for telemetry events.
#[cfg(feature = "otel")]
pub mod telemetry_otlp;
pub mod traits;
pub mod tui;
pub mod utils;
pub mod validation;
pub mod vault;
pub mod vault_bundle;
pub mod version;
#[cfg(feature = "sqlite")]
pub mod version_sqlite;
pub mod webhooks;

#[cfg(feature = "python")]
mod python;

pub use config::VaultConfig;
pub use conversion::{
    ConversionOptions, ConversionPipeline, ConversionProgress, ConversionResult, Converter,
    ValidationCheck, ValidationReport,
};
pub use crypto::streaming::{
    decrypt_chunked, encrypt_chunked, is_chunked_format, StreamHeader, DEFAULT_CHUNK_SIZE,
    HEADER_SIZE, STREAM_MAGIC, STREAM_VERSION,
};
pub use error::{ConversionError, CryptoError, Result, StorageError, VaultError};
pub use error::{
    EXIT_AUTH, EXIT_COMPLIANCE, EXIT_CONFIG, EXIT_GENERAL, EXIT_INTEGRITY, EXIT_INVALID_INPUT,
    EXIT_NOT_FOUND, EXIT_PERMISSION, EXIT_SUCCESS,
};
pub use model_card::{
    CaveatsAndRecommendations, EnvironmentalImpact, EthicalConsiderations, Evaluation, IntendedUse,
    Metric, ModelCard, ModelDetails, TrainingData,
};
pub use rag::{
    Database, Document, DocumentStore, InMemoryDatabase, KnowledgeBase, KnowledgeBaseConfig,
    MCPServer, MCPTool, RetrievalCache, Rule, RuleAction, RuleCondition, RuleEngine, ToolContext,
    ToolExecutor, ToolResult,
};
pub use traits::{
    AsyncBlobStore, AsyncBlobStoreAdapter, AuditLogSubscriber, AuditSink, BlobInfo, BlobReceipt,
    BlobStore, BlobStoreStats, CryptoProvider, EventBus, EventSubscriber, IvUri, MetricsSnapshot,
    MetricsSubscriber, NullAuditSink, VaultEvent, VaultMetrics, VaultState, VersionRepo,
};
pub use utils::{
    CompressionAnalyzer, CompressionReport, ModelAnalysis, ModelAnalyzer, ModelArchive,
    ModelDeduplicator, ModelExporter, PruningInfo, PruningMethod, QuantizationInfo,
    QuantizationSavings, RetrievalOptimizer,
};
pub use vault::{Vault, VaultBuilder, VersionBackend};
pub use version::{ModelVersion, VersionControl};
#[cfg(feature = "sqlite")]
pub use version_sqlite::SqliteVersionRepo;

// Benchmark exports
pub use benchmark::{BenchmarkRecord, BenchmarkResult, BenchmarkStore};

// Diff exports
pub use diff::{DiffSummary, ModelDiff, ModelDiffer, TensorChange, TensorInfo};

// Download exports
pub use download::{ModelDownloader, ModelSource};

// Interop exports
pub use interop::{
    register_lm_studio, register_ollama, InferenceEngine, LmStudioOptions, OllamaOptions,
    RegistrationResult,
};

// License scanning exports
pub use license_scan::{
    DetectedLicense, LicenseClass, LicenseScanReport, LicenseScanner, LicenseSource,
};

// Pickle scanning exports
pub use scanning::{PickleScanner, ScanFinding, ScanReport, Severity};

// Signing exports
pub use signing::{ModelSignature, ModelSigner, SignatureVerification, SigningKeyPair};

// Blockchain audit exports
pub use blockchain::{
    AuditBlock, AuditProof, BlockchainAudit, ChainVerification, MerkleProof, MerkleTree,
    ProofVerification,
};

// Federation exports
pub use federation::{
    ConflictResolution, FederationConfig, FederationManager, FederationStatus, PeerConfig,
    SyncConflict, SyncManifest, SyncResult, VectorClock,
};

// Access control exports
pub use access_control::{AclEntry, AclGuard, Role};

// GC exports
pub use gc::GcReport;

// KMS exports
pub use kms::{is_kms_uri, KmsBackend, KmsRequest, KmsUri};

// Lineage graph exports
pub use lineage_graph::{DerivationKind, LineageEdge, LineageGraph};

// Plugin exports
pub use plugins::{PluginInfo, PluginManifest, PluginRegistry};

// Policy exports
pub use policies::{PolicyReport, PolicyStore, RetentionPolicy};

// Profile exports
pub use profiles::{Profile, ProfileStore};

// Tag exports
pub use tags::{SearchQuery, SearchResult, TagStore};

// Validation exports
pub use validation::{ValidationProbe, ValidationResult, ValidationStore};

// Vault bundle exports
pub use vault_bundle::{BundleManifest, ExportReport, ImportReport};

// Webhook exports
pub use webhooks::{WebhookPayload, WebhookStore, WebhookTarget};

// Quantization exports
pub use quantization::{
    BatchQuantReport, QuantMethod, QuantProfile, QuantProfileStore, QuantResult,
};

// Evaluation exports
pub use evaluation::{EvalComparison, EvalRun, EvalStore, MetricDelta, MetricResult};

// Scheduler exports
pub use scheduler::{BackupFrequency, BackupManager, BackupRecord, BackupReport, BackupSchedule};

// Multi-vault exports
pub use multi_vault::{VaultEntry, VaultRegistry, VaultSummary};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
