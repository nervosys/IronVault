# IronVault - Project Structure

This document describes the organization of the IronVault project.

## Root Directory

```
IronVault/
├── src/                    # Source code (Rust)
├── tests/                  # Integration and unit tests
├── examples/               # Example programs (10)
├── benches/                # Performance benchmarks
├── docs/                   # Documentation
├── reports/                # Development reports and test outputs
├── deploy/                 # Deployment (systemd install script)
├── website/                # Project website
├── videos/                 # Demo videos
├── .github/                # GitHub Actions workflows
├── .vscode/                # VS Code configuration
├── .well-known/            # Agent/API discovery manifests
├── target/                 # Build artifacts (gitignored)
│
├── AGENTS.md               # AI agent discovery guide
├── README.md               # Main project documentation
├── CHANGELOG.md            # Version history and changes
├── LICENSE                 # Dual license (AGPL-3.0 + Commercial)
├── COMMERCIAL_LICENSE.md   # Commercial license terms
├── CLA.md                  # Contributor License Agreement
├── SECURITY.md             # Security policy
├── CONTRIBUTING.md         # Contribution guidelines
├── DEVELOPMENT.md          # Developer guide
├── ROADMAP.md              # Project roadmap
├── FORMATS.md              # Supported model formats
│
├── Cargo.toml              # Rust dependencies and configuration
├── Cargo.lock              # Locked dependency versions
├── pyproject.toml          # Python bindings configuration
├── deny.toml               # Dependency security policy (cargo-deny)
├── Makefile                # Build automation (Unix)
│
├── build.ps1               # Windows build script
├── build.sh                # Unix build script
├── validate.ps1            # Windows validation
└── validate.sh             # Unix validation
```

## Source Code (`src/`)

```
src/
├── lib.rs                  # Library entry point (pub modules)
├── main.rs                 # CLI application entry
├── error.rs                # Error types
├── config.rs               # XDG-compliant configuration
├── vault.rs                # Core vault logic + VaultBuilder
├── traits.rs               # Core traits, event system, URI parser, metrics
├── storage.rs              # Storage abstraction
├── version.rs              # Version control (JSON backend)
├── version_sqlite.rs       # Version control (SQLite backend)
├── formats.rs              # 23+ format detection
├── conversion.rs           # Format conversion pipeline (10 converters)
├── model_card.rs           # Model Cards (Google/HuggingFace standard)
├── utils.rs                # Model utilities
├── audit.rs                # Security audit logging
├── compliance.rs           # FIPS/CMMC/MITRE compliance checks
├── blockchain.rs           # Blockchain audit trail (Merkle proofs)
├── federation.rs           # Federated vault sync (vector clocks)
├── telemetry.rs            # Anonymous telemetry (opt-in)
├── python.rs               # Python bindings (PyO3)
│
├── crypto/                 # Cryptography module
│   ├── mod.rs              # AES-256-GCM + Argon2id (FIPS 140-3)
│   ├── compression.rs      # Compression algorithms
│   └── streaming.rs        # Streaming encryption
│
├── cli/                    # CLI subcommand handlers
│   ├── mod.rs              # CLI module root
│   ├── args.rs             # Argument parsing (clap)
│   ├── helpers.rs          # CLI helper functions
│   └── handlers/           # Subcommand implementations
│       ├── mod.rs
│       ├── analyze.rs      # Model analysis commands
│       ├── archive.rs      # Archive/extract commands
│       ├── card.rs         # Model card commands
│       ├── cloud.rs        # Cloud push/pull commands
│       ├── convert.rs      # Format conversion commands
│       ├── database.rs     # RAG database commands
│       ├── telemetry.rs    # Telemetry commands
│       └── vault.rs        # Core vault commands
│
├── api/                    # REST/GraphQL API (feature-gated)
│   ├── mod.rs              # API module root
│   ├── server.rs           # Axum server setup
│   ├── routes.rs           # REST route handlers
│   ├── graphql.rs          # GraphQL schema (async-graphql)
│   ├── openapi.rs          # OpenAPI specification
│   ├── auth.rs             # JWT authentication
│   ├── dashboard.rs        # Dashboard endpoints
│   └── error.rs            # API error types
│
├── rag/                    # RAG system (7 submodules)
│   ├── mod.rs              # RAG module root
│   ├── database.rs         # Database backends
│   ├── documents.rs        # Document processing
│   ├── knowledge.rs        # Knowledge base
│   ├── rules.rs            # Rule engine
│   ├── mcp.rs              # MCP tool integration
│   ├── vector.rs           # Vector similarity search
│   └── cache.rs            # Query caching
│
├── storage/                # Storage backends
│   ├── local.rs            # Local filesystem
│   ├── s3.rs               # AWS S3 (optional)
│   ├── azure.rs            # Azure Blob (optional)
│   └── gcs.rs              # Google Cloud (optional, disabled)
│
└── ironvault/           # Python package (PyO3 bindings)
    ├── __init__.py
    ├── core/
    ├── crypto/
    ├── formats/
    └── version/
```

## Tests (`tests/`)

```
tests/
├── integration_tests.rs            # End-to-end workflow tests
├── crypto_tests.rs                 # Cryptography tests
├── format_tests.rs                 # Format detection tests
├── conversion_tests.rs             # Format conversion tests
├── config_error_tests.rs           # Configuration tests
├── utils_tests.rs                  # Utilities tests
├── rag_tests.rs                    # RAG system tests
├── cli_tests.rs                    # CLI handler tests
├── api_tests.rs                    # REST/GraphQL API tests
├── vault_builder_tests.rs          # VaultBuilder tests
├── model_card_tests.rs             # Model card unit tests
├── model_card_integration_tests.rs # Model card integration tests
├── coverage_tests.rs               # Comprehensive coverage tests
└── test_ironvault.py            # Python bindings tests
```

## Examples (`examples/`)

```
examples/
├── basic_usage.rs          # Core vault operations
├── security_demo.rs        # Security features
├── utilities_demo.rs       # Model utilities showcase
├── rag_demo.rs             # RAG pipeline demo
├── mcp_tools_demo.rs       # MCP tools demo
├── model_card_demo.rs      # Model card generation
├── version_control_demo.rs # Version control workflow
├── providers_formats_demo.rs # Format providers demo
├── huggingface_demo.rs     # HuggingFace integration
└── xdg_demo.rs             # XDG configuration demo
```

## Documentation (`docs/`)

### User Guides
- **QUICKSTART.md** - 5-minute tutorial
- **CLI.md** - Command-line reference
- **CLOUD_CLI.md** - Cloud CLI reference
- **UTILITIES.md** - Model utilities guide
- **RAG.md** - RAG systems guide
- **MCP_TOOLS.md** - MCP tools documentation
- **MCP_QUICKREF.md** - MCP quick reference
- **CLOUD_STORAGE.md** - Cloud storage guide
- **HDF5_SUPPORT.md** - How .h5/.hdf5 files are handled
- **MODEL_CARDS.md** - Model card guide
- **MODEL_CARDS_QUICKREF.md** - Model card quick reference
- **VERSION_CONTROL.md** - Version control guide
- **VERSION_CONTROL_QUICKREF.md** - Version control quick reference
- **DEMO_GUIDE.md** - Demo script documentation
- **EXAMPLES_GUIDE.md** - Examples walkthrough

### Technical Docs
- **ARCHITECTURE.md** - System architecture (v1)
- **ARCHITECTURE_V2.md** - System architecture (v2)
- **FEATURE_FLAGS.md** - Cargo features reference
- **PERFORMANCE.md** - Performance benchmarks
- **SECURITY_AUDIT.md** - Security audit report
- **SECURITY_HARDENING.md** - Security hardening guide
- **VULNERABILITY_FIXES.md** - Vulnerability resolution report
- **XDG_COMPLIANCE.md** - XDG compliance details
- **XDG_QUICKREF.md** - XDG quick reference
- **MIGRATION.md** - Migration guide
- **UV_INTEGRATION.md** - UV Python integration

### Quick References
- **UTILITIES_QUICKREF.md** - Utilities quick reference
- **UTILITIES_SUMMARY.md** - Utilities overview
- **RAG_QUICKREF.md** - RAG quick reference
- **PROVIDERS_FORMATS.md** - Format providers guide
- **PROVIDERS_FORMATS_QUICKREF.md** - Format providers quick reference
- **DATABASE.md** - Database configuration

### Project Info
- **PROJECT_STRUCTURE.md** - This file
- **PROJECT_SUMMARY.md** - Project overview
- **IMPLEMENTATION.md** - Implementation details
- **TOP_10_FEATURES.md** - Feature highlights
- **EXECUTIVE_SUMMARY.md** - Project metrics
- **PYTORCH_UV_GUIDE.md** - PyTorch + UV setup
- **archived/** - Archived launch docs

## Reports (`reports/`)

### Completion Reports
- **PROJECT_COMPLETE.md** - Overall completion status
- **PRODUCTION_READY.md** - Production readiness status
- **FINAL_STATUS.md** - Final project status
- **FEATURE_COMPLETION_STATUS.md** - Feature completion matrix
- **CLI_UTILITIES_COMPLETE.md** - CLI integration
- **CLOUD_CLI_COMPLETE.md** - Cloud CLI integration
- **CLOUD_STORAGE_COMPLETE.md** - Cloud storage
- **FORMAT_CONVERSION_COMPLETE.md** - Format conversion
- **MCP_IMPLEMENTATION_COMPLETE.md** - MCP tools
- **RAG_IMPLEMENTATION_COMPLETE.md** - RAG system
- **UTILITIES_IMPLEMENTATION_COMPLETE.md** - Utilities
- **VERSION_CONTROL_COMPLETE.md** - Version control
- **MODEL_CARDS_COMPLETE.md** - Model cards
- **TESTING_COMPLETE.md** - Testing validation
- **TEST_COVERAGE.md** - Test coverage analysis
- **COMPREHENSIVE_TEST_REPORT.md** - Detailed test results
- **SECURITY_STATUS.md** - Security posture

### Test Outputs
- **basic_usage_output.txt** - Basic example output
- **security_demo_output.txt** - Security demo output
- **utilities_demo_output.txt** - Utilities demo output
- **test_results.txt** - Full test suite results

## Build Artifacts (`target/`)

Generated by Cargo during compilation (not committed to git):
- `debug/` - Development builds
- `release/` - Optimized release builds
- `doc/` - Generated API documentation

## GitHub Actions (`.github/`)

```
.github/workflows/
├── ci.yml              # Continuous integration
├── security.yml        # Security scanning
└── release.yml         # Release automation
```

## VS Code Configuration (`.vscode/`)

```
.vscode/
└── instructions.md     # AI assistant instructions
```

## Key Files

| File                | Purpose                                     |
| ------------------- | ------------------------------------------- |
| **README.md**       | Main project documentation                  |
| **AGENTS.md**       | AI agent discovery guide                    |
| **CHANGELOG.md**    | Version history                             |
| **ROADMAP.md**      | Project roadmap                             |
| **Cargo.toml**      | Rust dependencies and project configuration |
| **LICENSE**         | Dual license (AGPL-3.0 + Commercial)        |
| **SECURITY.md**     | Security policy and reporting               |
| **CONTRIBUTING.md** | Contribution guidelines                     |
| **DEVELOPMENT.md**  | Developer setup and workflow                |
| **FORMATS.md**      | Supported AI model formats (23+)            |
| **deny.toml**       | Dependency security policy (cargo-deny)     |

## Build Commands

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Generate documentation
cargo doc --open

# Run example
cargo run --example basic_usage

# Install CLI tool
cargo install --path .
```

## Feature Flags

```bash
# Default features (safetensors, ndarray, sqlite)
cargo build

# Full features (all non-system features)
cargo build --features full

# API server
cargo build --features api,graphql

# Cloud storage
cargo build --features cloud
cargo build --features s3
cargo build --features azure

# Python bindings
cargo build --features python

# With HDF5 support (requires system library)

```

See [FEATURE_FLAGS.md](FEATURE_FLAGS.md) for a complete reference.

## Getting Started

1. **Read** `README.md` for overview
2. **Follow** `docs/QUICKSTART.md` for 5-minute setup
3. **Explore** `examples/` for working code
4. **Reference** `docs/CLI.md` for commands
5. **Contribute** via `CONTRIBUTING.md` guidelines

## Documentation Navigation

- **New Users** → README.md → docs/QUICKSTART.md
- **CLI Users** → docs/CLI.md → docs/UTILITIES.md
- **Developers** → DEVELOPMENT.md → docs/ARCHITECTURE_V2.md → src/
- **Security** → SECURITY.md → docs/SECURITY_AUDIT.md → docs/SECURITY_HARDENING.md
- **RAG/AI** → docs/RAG.md → docs/MCP_TOOLS.md → docs/DATABASE.md
- **Cloud** → docs/CLOUD_STORAGE.md → docs/CLOUD_CLI.md
- **Features** → docs/FEATURE_FLAGS.md → docs/PERFORMANCE.md
- **Model Cards** → docs/MODEL_CARDS.md → docs/MODEL_CARDS_QUICKREF.md
- **Version Control** → docs/VERSION_CONTROL.md → docs/VERSION_CONTROL_QUICKREF.md

---

**Last Updated**: November 4, 2025  
**Project Structure Version**: 1.0
