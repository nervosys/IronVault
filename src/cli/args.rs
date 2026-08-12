//! CLI argument definitions and command structures.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "iv")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Universal secure vault for AI model formats (Neural & Neurosymbolic)", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Vault name (uses default if not specified)
    #[arg(short, long)]
    pub vault: Option<String>,

    /// Config file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Use SQLite for version storage (ACID, indexed, concurrent)
    #[cfg(feature = "sqlite")]
    #[arg(long, env = "IRONVAULT_SQLITE_VERSIONS")]
    pub sqlite_versions: bool,

    /// Disable telemetry for this session
    #[arg(long, env = "IRONVAULT_TELEMETRY_DISABLED")]
    pub no_telemetry: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new vault
    Init {
        /// Vault name
        #[arg(short, long, default_value = "default")]
        name: String,
    },

    /// Store a model in the vault
    Store {
        /// Model name
        name: String,

        /// Path to model file
        path: PathBuf,

        /// Model format (safetensors, gguf, pytorch, onnx, tflite, coreml, tensorrt, mlx, etc.)
        /// Auto-detected from extension if not specified
        #[arg(short, long)]
        format: Option<String>,

        /// Description
        #[arg(short, long)]
        description: Option<String>,

        /// Framework (e.g., pytorch, tensorflow)
        #[arg(long)]
        framework: Option<String>,

        /// Task (e.g., text-generation, image-classification)
        #[arg(long)]
        task: Option<String>,
    },

    /// Retrieve a model from the vault
    Get {
        /// Model name
        name: String,

        /// Output path
        output: PathBuf,

        /// Version number (latest if not specified)
        #[arg(short, long)]
        version: Option<u32>,
    },

    /// List all models in the vault
    List {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Show versions of a model
    Versions {
        /// Model name
        name: String,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Show lineage/history of a model version
    Lineage {
        /// Model name
        name: String,

        /// Version number
        version: u32,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Delete a model version
    Delete {
        /// Model name
        name: String,

        /// Version number
        version: u32,

        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Show vault statistics
    Stats {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Run compliance checks
    Compliance {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Change vault passphrase
    ChangePassphrase,

    /// Archive models to TAR or ZIP
    Archive {
        /// Model names to archive
        #[arg(required = true)]
        models: Vec<String>,

        /// Output archive path
        output: PathBuf,

        /// Archive format (tar or zip)
        #[arg(short, long, default_value = "tar")]
        format: String,

        /// Version numbers (if not specified, uses latest for each model)
        #[arg(short, long)]
        versions: Option<Vec<u32>>,
    },

    /// Extract models from archive
    Extract {
        /// Archive path
        archive: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },

    /// Analyze compression efficiency
    Analyze {
        /// Model name
        name: String,

        /// Version number (latest if not specified)
        #[arg(short, long)]
        version: Option<u32>,
    },

    /// Find duplicate models in vault
    Deduplicate {
        /// Show detailed similarity scores
        #[arg(short, long)]
        detailed: bool,
    },

    /// Export model with metadata
    Export {
        /// Model name
        name: String,

        /// Output directory
        output: PathBuf,

        /// Version number (latest if not specified)
        #[arg(short, long)]
        version: Option<u32>,
    },

    /// Show cache statistics (if caching is enabled)
    Cache,

    /// Convert model between formats
    Convert {
        /// Model name in vault
        name: String,

        /// Target format (safetensors, onnx, gguf, tflite, coreml, etc.)
        #[arg(short = 't', long)]
        to_format: String,

        /// Output file path (optional, defaults to model_name.{extension})
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Version number (latest if not specified)
        #[arg(short, long)]
        version: Option<u32>,

        /// Quantization level for GGUF conversion (q4_0, q4_k_m, q8_0, etc.)
        #[arg(short, long)]
        quantization: Option<String>,

        /// ONNX opset version (default 17)
        #[arg(long)]
        opset: Option<u32>,

        /// Validate conversion output
        #[arg(long)]
        validate: bool,

        /// Only show conversion plan (don't execute)
        #[arg(long)]
        plan_only: bool,
    },

    /// List supported format conversions
    ListConversions,

    /// Start REST API server
    #[cfg(feature = "api")]
    Serve {
        /// Host address to bind to
        #[arg(long, default_value = "127.0.0.1", env = "IRONVAULT_HOST")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value_t = 8080, env = "IRONVAULT_PORT")]
        port: u16,

        /// JWT secret for token signing
        #[arg(long, env = "IRONVAULT_JWT_SECRET")]
        jwt_secret: String,

        /// Token expiry in seconds
        #[arg(long, default_value_t = 3600)]
        token_expiry: u64,

        /// Allow CORS from any origin
        #[arg(long)]
        cors_permissive: bool,

        /// Disable the embedded web dashboard
        #[arg(long)]
        no_dashboard: bool,

        /// File to persist the JWT revocation list to
        ///
        /// Without it, revoked tokens are re-admitted when the server
        /// restarts. Use durable storage that outlives the process.
        #[arg(long, env = "IRONVAULT_REVOCATION_STORE")]
        revocation_store: Option<std::path::PathBuf>,
    },

    /// Cloud storage operations
    Cloud {
        #[command(subcommand)]
        command: CloudCommands,
    },

    /// Blockchain audit trail (requires security.blockchain_audit = true)
    Chain {
        #[command(subcommand)]
        command: ChainCommands,
    },

    /// Federation — sync models with peer nodes
    Federation {
        #[command(subcommand)]
        command: FederationCommands,
    },

    /// Model card operations
    Card {
        #[command(subcommand)]
        command: CardCommands,
    },

    /// Database operations for RAG knowledge base
    Database {
        #[command(subcommand)]
        command: DatabaseCommands,
    },

    /// Telemetry and analytics settings
    Telemetry {
        #[command(subcommand)]
        command: TelemetryCommands,
    },

    /// Output complete CLI schema as machine-readable JSON for agent discovery
    Introspect {
        /// Output format (json, yaml, jsonld)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Minimize output (no descriptions or examples)
        #[arg(long)]
        compact: bool,
    },

    /// Pull/download a model from HuggingFace, Ollama, or HTTPS URL
    Pull {
        /// Model source URI (hf://org/model, ollama://model:tag, or https://...)
        source: String,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Expected SHA-256 hash for verification
        #[arg(long)]
        sha256: Option<String>,

        /// HuggingFace auth token (for private repos)
        #[arg(long, env = "HF_TOKEN")]
        token: Option<String>,

        /// Store directly into the vault after download
        #[arg(long)]
        store: bool,

        /// Model name when using --store
        #[arg(long)]
        name: Option<String>,
    },

    /// Sign a model file for provenance verification
    Sign {
        /// Model name in vault (or path to file with --file)
        name: String,

        /// Version number (latest if not specified)
        #[arg(short, long)]
        version: Option<u32>,

        /// Signing key pair: a file path (generated if not found), or a KMS URI
        /// such as `azure-kv://my-vault/hmac-key` (see docs/KMS.md)
        #[arg(short, long)]
        key: Option<String>,

        /// Signer identity (e.g. email or name)
        #[arg(short, long)]
        identity: Option<String>,

        /// Sign a file on disk instead of a vault model
        #[arg(long)]
        file: Option<PathBuf>,
    },

    /// Verify a model file's signature
    Verify {
        /// Model name in vault (or path to file with --file)
        name: String,

        /// Version number (latest if not specified)
        #[arg(short, long)]
        version: Option<u32>,

        /// Path to signature file
        #[arg(short, long)]
        signature: PathBuf,

        /// Verification key: a file path or a KMS URI (see docs/KMS.md).
        /// Omit to check the file hash only.
        #[arg(short, long)]
        key: Option<String>,

        /// Verify a file on disk instead of a vault model
        #[arg(long)]
        file: Option<PathBuf>,
    },

    /// Scan a model file for pickle deserialization vulnerabilities
    Scan {
        /// Model name in vault
        name: Option<String>,

        /// Scan a file on disk instead of a vault model
        #[arg(long)]
        file: Option<PathBuf>,

        /// Version number (latest if not specified)
        #[arg(short, long)]
        version: Option<u32>,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Compare two model versions or files
    Diff {
        /// Left model (name@version or file path)
        left: String,

        /// Right model (name@version or file path)
        right: String,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Register a vault model with an inference engine (Ollama, LM Studio)
    Register {
        /// Model name in vault
        name: String,

        /// Inference engine (ollama, lm-studio)
        #[arg(short, long)]
        engine: String,

        /// Version number (latest if not specified)
        #[arg(short, long)]
        version: Option<u32>,

        /// Name to register the model as (defaults to vault model name)
        #[arg(long)]
        alias: Option<String>,

        /// System prompt (Ollama only)
        #[arg(long)]
        system_prompt: Option<String>,
    },

    /// Record or view benchmark results for a model
    Benchmark {
        #[command(subcommand)]
        command: BenchmarkCommands,
    },

    /// Scan model files or directories for license information
    LicenseScan {
        /// Path to scan (file or directory)
        path: PathBuf,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Manage model tags
    Tag {
        #[command(subcommand)]
        command: TagCommands,
    },

    /// Search models by name or tags
    Search {
        /// Search query text
        #[arg(default_value = "")]
        query: String,

        /// Filter by tags
        #[arg(short, long)]
        tag: Vec<String>,

        /// Output format (text, json)
        #[arg(short, long)]
        format: Option<String>,
    },

    /// Export vault as portable archive
    VaultExport {
        /// Output archive path (.tar.gz)
        output: PathBuf,
    },

    /// Import vault from portable archive
    VaultImport {
        /// Archive path
        archive: PathBuf,

        /// Target directory (defaults to current vault)
        #[arg(short, long)]
        target: Option<PathBuf>,
    },

    /// Garbage-collect orphaned blobs and temp files
    Gc {
        /// Show what would be cleaned without deleting
        #[arg(long)]
        dry_run: bool,
    },

    /// Browse vault models in a formatted table
    Browse,

    /// Manage webhook targets
    Webhook {
        #[command(subcommand)]
        command: WebhookCommands,
    },

    /// Manage access control lists
    Acl {
        #[command(subcommand)]
        command: AclCommands,
    },

    /// Validate a stored model's integrity
    Validate {
        /// Model name
        name: String,

        /// Version (latest if not specified)
        #[arg(short, long)]
        version: Option<u32>,
    },

    /// Manage retention policies
    Policy {
        #[command(subcommand)]
        command: PolicyCommands,
    },

    /// Cross-model lineage graph
    LineageGraph {
        #[command(subcommand)]
        command: LineageGraphCommands,
    },

    /// Manage plugins
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },

    /// Manage configuration profiles
    Profile {
        #[command(subcommand)]
        command: ProfileCommands,
    },

    /// Manage quantization profiles and estimate sizes
    Quantize {
        #[command(subcommand)]
        command: QuantizeCommands,
    },

    /// Record and compare model evaluations
    Eval {
        #[command(subcommand)]
        command: EvalCommands,
    },

    /// Manage vault backup schedules
    Backup {
        #[command(subcommand)]
        command: BackupCommands,
    },

    /// Manage multiple vault registrations
    Vaults {
        #[command(subcommand)]
        command: VaultsCommands,
    },
}

#[derive(Subcommand)]
pub enum TelemetryCommands {
    /// Show current telemetry status
    Status,

    /// Enable telemetry
    Enable,

    /// Disable telemetry (opt out)
    Disable,

    /// Reset device ID (generates new anonymous ID)
    Reset,
}

#[derive(Subcommand)]
pub enum DatabaseCommands {
    /// Initialize a new database
    Init {
        /// Database path
        #[arg(short, long)]
        path: PathBuf,

        /// Database type (sqlite, sled)
        #[arg(short = 't', long, default_value = "sqlite")]
        db_type: String,
    },

    /// Store a document in the database
    Store {
        /// Database path
        #[arg(short, long)]
        path: PathBuf,

        /// Document content (from file or stdin)
        #[arg(short, long)]
        input: PathBuf,

        /// Document ID (optional, generated if not provided)
        #[arg(short = 'I', long)]
        id: Option<String>,

        /// Metadata key=value pairs
        #[arg(short, long)]
        metadata: Vec<String>,
    },

    /// Retrieve a document by ID
    Get {
        /// Database path
        #[arg(short, long)]
        path: PathBuf,

        /// Document ID
        id: String,
    },

    /// Search documents
    Search {
        /// Database path
        #[arg(short, long)]
        path: PathBuf,

        /// Search query
        query: String,

        /// Maximum number of results
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },

    /// List all documents
    List {
        /// Database path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// Delete a document
    Delete {
        /// Database path
        #[arg(short, long)]
        path: PathBuf,

        /// Document ID
        id: String,
    },

    /// Export database to JSON
    Export {
        /// Database path
        #[arg(short, long)]
        path: PathBuf,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Import documents from JSON
    Import {
        /// Database path
        #[arg(short, long)]
        path: PathBuf,

        /// Input JSON file
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Show database statistics
    Stats {
        /// Database path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// Build vector index for similarity search
    BuildIndex {
        /// Database path
        #[arg(short, long)]
        path: PathBuf,

        /// Output index path
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Search by vector similarity
    VectorSearch {
        /// Index path
        #[arg(short, long)]
        index: PathBuf,

        /// Query text or embedding file
        #[arg(short, long)]
        query: PathBuf,

        /// Number of results
        #[arg(short = 'n', long, default_value = "5")]
        limit: usize,
    },
}

#[derive(Subcommand)]
pub enum CardCommands {
    /// Create a new model card
    Create {
        /// Model name
        name: String,

        /// Model version
        #[arg(short, long)]
        version: String,

        /// Description
        #[arg(short, long)]
        description: String,

        /// Model type (e.g., "Large Language Model", "Image Classifier")
        #[arg(short = 't', long)]
        model_type: String,

        /// Architecture (e.g., "Transformer", "ResNet-50")
        #[arg(short, long)]
        architecture: String,

        /// Output file (JSON, YAML, or Markdown based on extension)
        #[arg(short, long)]
        output: PathBuf,

        /// Open interactive wizard for additional fields
        #[arg(short, long)]
        interactive: bool,
    },

    /// Show a model card
    Show {
        /// Path to model card file (JSON or YAML)
        path: PathBuf,

        /// Output format (json, yaml, markdown)
        #[arg(short, long, default_value = "markdown")]
        format: String,
    },

    /// Validate a model card
    Validate {
        /// Path to model card file
        path: PathBuf,

        /// Check for required fields
        #[arg(short, long)]
        strict: bool,
    },

    /// Convert model card between formats
    Convert {
        /// Input model card file
        input: PathBuf,

        /// Output file (format determined by extension)
        output: PathBuf,
    },

    /// Generate a template model card
    Template {
        /// Template type (llm, classifier, medical, hiring)
        #[arg(short, long, default_value = "basic")]
        template_type: String,

        /// Output file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Attach a model card to a vault model
    Attach {
        /// Model name in vault
        model: String,

        /// Version number (latest if not specified)
        #[arg(short, long)]
        version: Option<u32>,

        /// Path to model card file
        card: PathBuf,
    },

    /// Extract model card from a vault model
    Extract {
        /// Model name in vault
        model: String,

        /// Version number (latest if not specified)
        #[arg(short, long)]
        version: Option<u32>,

        /// Output file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Generate model card from model metadata
    Generate {
        /// Model name in vault
        model: String,

        /// Version number (latest if not specified)
        #[arg(short, long)]
        version: Option<u32>,

        /// Output file
        #[arg(short, long)]
        output: PathBuf,

        /// Include training data section
        #[arg(long)]
        include_training: bool,

        /// Include evaluation section
        #[arg(long)]
        include_evaluation: bool,
    },
}

#[derive(Subcommand)]
pub enum FederationCommands {
    /// Show this node's identity, peers, and sync history
    Status,

    /// Show what a sync with a peer would transfer, without transferring it
    Plan {
        /// Peer node ID (see `iv federation status`)
        peer: String,
    },

    /// Sync with a peer: download what is missing here, upload what is missing there
    Sync {
        /// Peer node ID
        peer: String,

        /// Report what would transfer, then stop
        #[arg(long)]
        dry_run: bool,
    },

    /// Print this node's sync manifest (what peers would see)
    Manifest,
}

#[derive(Subcommand)]
pub enum ChainCommands {
    /// Show chain height, latest block hash, and pending entry count
    Status,

    /// Verify the whole chain: hash links, Merkle roots, and block hashes
    Verify,

    /// Emit a Merkle inclusion proof for one entry
    Proof {
        /// Block index (see `iv chain status` for the height)
        #[arg(short, long)]
        block: u64,

        /// Entry index within that block
        #[arg(short, long)]
        entry: usize,

        /// Write the proof JSON here instead of stdout
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },

    /// Verify a proof produced by `iv chain proof`
    ///
    /// Checks the proof internally -- that the entry hashes to a leaf which
    /// reaches the stated Merkle root. It does not confirm that root belongs
    /// to this vault's chain; run `iv chain verify` for that.
    VerifyProof {
        /// Path to the proof JSON
        proof: std::path::PathBuf,
    },

    /// Search recorded entries
    Search {
        /// Filter by model name
        #[arg(short, long)]
        model: Option<String>,

        /// Filter by event type (e.g. MODEL_STORED, MODEL_DELETED)
        #[arg(short, long)]
        event: Option<String>,

        /// Maximum results
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
}

#[derive(Subcommand)]
pub enum CloudCommands {
    /// Push model to cloud storage
    Push {
        /// Model name
        model: String,

        /// Version number (latest if not specified)
        #[arg(short, long)]
        version: Option<u32>,

        /// Cloud provider (s3, azure, gcs)
        #[arg(short, long)]
        provider: String,

        /// Cloud bucket/container name
        #[arg(short, long)]
        bucket: String,
    },

    /// Pull model from cloud storage
    Pull {
        /// Model name
        model: String,

        /// Cloud provider (s3, azure, gcs)
        #[arg(short, long)]
        provider: String,

        /// Cloud bucket/container name
        #[arg(short, long)]
        bucket: String,

        /// Remote path/key
        #[arg(short = 'k', long)]
        remote_path: String,
    },

    /// List models in cloud storage
    List {
        /// Cloud provider (s3, azure, gcs)
        #[arg(short, long)]
        provider: String,

        /// Cloud bucket/container name
        #[arg(short, long)]
        bucket: String,

        /// Prefix/folder path (optional)
        #[arg(short = 'x', long)]
        prefix: Option<String>,
    },

    /// Configure cloud credentials
    Config {
        /// Cloud provider (s3, azure, gcs)
        #[arg(short, long)]
        provider: String,

        /// Show current configuration
        #[arg(short, long)]
        show: bool,
    },
}

#[derive(Subcommand)]
pub enum BenchmarkCommands {
    /// Record a benchmark result for a model version
    Add {
        /// Model name
        name: String,

        /// Version number
        #[arg(short, long)]
        version: u32,

        /// Benchmark name (e.g. MMLU, HumanEval, perplexity)
        #[arg(short, long)]
        benchmark: String,

        /// Score value
        #[arg(short, long)]
        score: f64,

        /// Unit (e.g. accuracy, ppl, pass@1, ms, tokens/s)
        #[arg(short, long, default_value = "score")]
        unit: String,

        /// Higher is better
        #[arg(long)]
        higher_is_better: bool,

        /// Hardware description
        #[arg(long)]
        hardware: Option<String>,

        /// Dataset or split
        #[arg(long)]
        dataset: Option<String>,
    },

    /// Show benchmark results for a model
    Show {
        /// Model name
        name: String,

        /// Version number (shows all versions if not specified)
        #[arg(short, long)]
        version: Option<u32>,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
}

#[derive(Subcommand)]
pub enum TagCommands {
    /// Add tags to a model
    Add {
        /// Model name
        model: String,

        /// Tags to add
        #[arg(required = true)]
        tags: Vec<String>,
    },

    /// Remove tags from a model
    Remove {
        /// Model name
        model: String,

        /// Tags to remove
        #[arg(required = true)]
        tags: Vec<String>,
    },

    /// List tags for a model
    List {
        /// Model name
        model: String,
    },
}

#[derive(Subcommand)]
pub enum WebhookCommands {
    /// Register a webhook target
    Add {
        /// Webhook ID
        id: String,

        /// Target URL
        url: String,

        /// Shared secret for HMAC signing
        #[arg(short, long)]
        secret: Option<String>,

        /// Events to subscribe to (empty = all)
        #[arg(short, long)]
        events: Vec<String>,
    },

    /// Remove a webhook
    Remove {
        /// Webhook ID
        id: String,
    },

    /// List all webhooks
    List,
}

#[derive(Subcommand)]
pub enum AclCommands {
    /// Grant a role to an identity
    Grant {
        /// Identity (e.g. username or email)
        identity: String,

        /// Role (reader, writer, admin)
        role: String,
    },

    /// Revoke access for an identity
    Revoke {
        /// Identity
        identity: String,
    },

    /// List all ACL entries
    List,
}

#[derive(Subcommand)]
pub enum PolicyCommands {
    /// Set a retention policy for a model
    Set {
        /// Model name
        model: String,

        /// Maximum number of versions to keep
        #[arg(long)]
        max_versions: Option<usize>,

        /// Maximum age in days
        #[arg(long)]
        max_age_days: Option<u64>,

        /// Minimum versions to always keep
        #[arg(long)]
        keep_minimum: Option<usize>,
    },

    /// Remove a retention policy
    Remove {
        /// Model name
        model: String,
    },

    /// Show the policy for a model
    Show {
        /// Model name
        model: String,
    },

    /// Apply retention policies (delete old versions)
    Apply {
        /// Apply only for a specific model (applies all if omitted)
        #[arg(short, long)]
        model: Option<String>,

        /// Show what would be deleted without deleting
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
pub enum LineageGraphCommands {
    /// Add a derivation edge
    Add {
        /// Parent model (name or name@version)
        parent: String,

        /// Child model (name or name@version)
        child: String,

        /// Derivation kind (finetune, quantize, merge, distill, convert)
        #[arg(short, long, default_value = "finetune")]
        kind: String,
    },

    /// Show lineage for a model
    Show {
        /// Model name
        model: String,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
}

#[derive(Subcommand)]
pub enum PluginCommands {
    /// List installed plugins
    List,

    /// Install a plugin from a directory
    Install {
        /// Plugin directory path
        path: PathBuf,
    },

    /// Uninstall a plugin by ID
    Uninstall {
        /// Plugin ID
        id: String,
    },

    /// Discover plugins in the plugin directory
    Discover,
}

#[derive(Subcommand)]
pub enum ProfileCommands {
    /// Create a new profile
    Create {
        /// Profile name
        name: String,

        /// Key=value overrides
        #[arg(short, long)]
        set: Vec<String>,
    },

    /// Activate a profile
    Activate {
        /// Profile name
        name: String,
    },

    /// Deactivate the current profile
    Deactivate,

    /// List all profiles
    List,

    /// Show a profile's details
    Show {
        /// Profile name
        name: String,
    },

    /// Delete a profile
    Delete {
        /// Profile name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum QuantizeCommands {
    /// Create or update a quantization profile
    Set {
        /// Profile name
        name: String,

        /// Quantization method (q4_0, q4_k_m, q5_k_m, q8_0, f16, f32)
        #[arg(short, long)]
        method: String,

        /// Description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Remove a quantization profile
    Remove {
        /// Profile name
        name: String,
    },

    /// List quantization profiles
    List,

    /// Estimate output size for a quantization method
    Estimate {
        /// Original file size in bytes
        #[arg(short, long)]
        size: u64,

        /// Source precision (default: f32)
        #[arg(long, default_value = "f32")]
        from: String,

        /// Target quantization method
        #[arg(short, long)]
        to: String,
    },
}

#[derive(Subcommand)]
pub enum EvalCommands {
    /// Record an evaluation run
    Record {
        /// Model name
        name: String,

        /// Model version
        #[arg(short, long)]
        version: u64,

        /// Evaluation suite name (e.g. mmlu, hellaswag)
        #[arg(short, long)]
        suite: String,

        /// Metric in name=value format (repeatable)
        #[arg(short, long, required = true)]
        metric: Vec<String>,

        /// Unit for all metrics (default: score)
        #[arg(short, long, default_value = "score")]
        unit: String,

        /// Higher is better (default: true)
        #[arg(long, default_value_t = true)]
        higher_is_better: bool,
    },

    /// List evaluation runs for a model
    List {
        /// Model name
        name: String,

        /// Filter by version
        #[arg(short, long)]
        version: Option<u64>,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Compare two model versions on a suite
    Compare {
        /// First model (name@version)
        a: String,

        /// Second model (name@version)
        b: String,

        /// Evaluation suite
        #[arg(short, long)]
        suite: String,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// List all known evaluation suites
    Suites,
}

#[derive(Subcommand)]
pub enum BackupCommands {
    /// Create or update a backup schedule
    Set {
        /// Schedule name
        name: String,

        /// Frequency (hourly, daily, weekly, monthly)
        #[arg(short, long)]
        frequency: String,

        /// Maximum backups to retain
        #[arg(short, long, default_value_t = 7)]
        max_backups: usize,

        /// Output directory for backup archives
        #[arg(short, long)]
        output_dir: std::path::PathBuf,
    },

    /// Remove a backup schedule
    Remove {
        /// Schedule name
        name: String,
    },

    /// List backup schedules
    List,

    /// Show backup history
    History {
        /// Filter by schedule name
        #[arg(short, long)]
        schedule: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum VaultsCommands {
    /// Register a vault
    Register {
        /// Vault name/alias
        name: String,

        /// Path to vault directory
        path: std::path::PathBuf,

        /// Description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Unregister a vault
    Unregister {
        /// Vault name
        name: String,
    },

    /// Set the active vault
    Activate {
        /// Vault name
        name: String,
    },

    /// Clear the active vault
    Deactivate,

    /// List registered vaults
    List,
}
