# IronVault — Roadmap

> Last updated: 2026-07-29
> Current version: **3.0.0** (one honest exit-code contract, enforced)
> Status: Production release — 2,160+ Rust tests + 84 Python tests, 11 proptest strategies, 8 fuzz targets, 5 benchmark suites, clippy clean across every feature combination the crate declares, `cargo audit` / `cargo deny` clean with zero suppressions

---

## v3.0.0 — Exit-Code Contract (complete)

The project published four mutually contradictory exit-code tables and
implemented none of them, while telling agents to branch on them.

- [x] **One contract, implemented** — `VaultError::exit_code` maps every variant to `0`–`8`; `README.md`, `AGENTS.md`, `docs/CLI.md`, `.well-known/agents.json` and `.well-known/ontology.jsonld` were all rewritten to match, and a test fails if they drift again
- [x] **Twelve commands no longer exit 0 for work that did not happen** — including `iv validate` (an integrity gate that printed "Some checks failed." and succeeded) and `iv eval compare` (a regression gate that compared nothing and succeeded)
- [x] **A mistyped subcommand no longer exits 2**, which the table defines as "authentication failed"
- [x] **`VaultError` is `#[non_exhaustive]`** — future variants are non-breaking downstream, but still force an exit-code decision inside the crate

## v2.0.0 — Security (complete)

Five defects sharing one shape: code that emitted a confident, plausible answer
where it should have said it could not tell.

- [x] **`iv verify` refuses to verify without a key** — it previously reported forged models as valid, comparing the file hash against a value the attacker-supplied `.sig` file itself contains, and then exited 0 when it printed FAILED
- [x] **Signatures are real HMAC-SHA256** (RFC 2104, tested against RFC 4231), compared in constant time; the old `SHA-256(seed ‖ hash)` construction still verifies as version 1
- [x] **The pickle scanner reads compressed ZIP members** — a DEFLATE-compressed malicious checkpoint previously scanned as `safe: true`
- [x] **`iv vault-import` validates blob paths and verifies the bundle checksum** — the manifest's `file_path` was attacker-controlled and passed straight to `Path::join` (CWE-22), and the checksum was written but never read
- [x] **GGUF metadata is parsed, not guessed at** — one shared bounds-checked reader fixes both `iv diff` (which saw F32 and Q4_K as identical) and license detection (which reported non-commercial models as MIT)
- [x] **Every feature flag the crate declares is built by CI** — the two that no job compiled, `gpu` and `hdf5-support`, were both broken or inert and have been removed

## v1.7.0 — Unattended Operation, KMS & Security (complete)

Closes the gap between what the docs promised and what the code did, and clears
the security backlog. Four features were documented but never implemented, two
cargo features did not compile at all, and nine RUSTSEC advisories had accrued
since the last release.

### Unattended operation

- [x] **Non-interactive passphrase** — `$IRONVAULT_PASSPHRASE` (literal or KMS URI) → piped stdin → interactive prompt. `AGENTS.md` documented the env var; no code read it, so every passphrase-gated command required a TTY.
- [x] **CLI integration coverage** — 7 tests covering store → list → get round-trip, wrong-passphrase rejection, KMS URIs, and stdin. Previously no CLI test could unlock a vault.

### KMS

- [x] **URI scheme** (`KmsUri`) — `env://`, `file://`, `aws-sm://`, `azure-kv://`, `vault://`, matching the table in `docs/KMS.md`, which had no parser behind it
- [x] **`file://` backend** — permission-checked (rejects group/world-readable secrets on Unix)
- [x] **HashiCorp Vault backend** — KV v2 with v1 fallback over `$VAULT_ADDR`
- [x] **Azure Key Vault backend** — REST with a bearer token from `AZURE_KEYVAULT_TOKEN`
- [x] **AWS Secrets Manager backend** — `aws-sdk-secretsmanager` under the `s3` feature (the previous stub errored even with `s3` enabled)

### Build health

- [x] **`s3` feature compiles** — missing `ModelFormat` / `ModelMetadata` imports in `cli/handlers/cloud.rs`
- [x] **`azure` feature compiles** — three API-drift errors in `storage/azure.rs`; `futures-util` added under the feature
- [x] **Clippy clean on current stable** — 60+ findings across lib, tests, examples, and benches
- [x] **CI feature matrix** — clippy over `default`, `s3`, `azure`, `cloud`, `api`, `database`; CI previously built only `full,graphql`, which is why the cloud features rotted unnoticed
- [x] **CI clippy runs `--all-targets`** — examples and benches were never linted

### Security

- [x] **9 RUSTSEC advisories resolved** — `cargo audit` and `cargo deny check` pass again after going red on advisories published since v1.6.0
- [x] **AWS SDKs off the legacy TLS stack** — their default `rustls` feature selects hyper-0.14/rustls-0.21, pulling vulnerable rustls-webpki 0.101 (RUSTSEC-2026-0098/0099/0104)
- [x] **pyo3 0.24 → 0.29** (RUSTSEC-2026-0176/0177) — API migration in `src/python.rs`
- [x] **Stale ignores pruned from `deny.toml`** — three entries were suppressing advisories that are now genuinely fixed
- [x] **`.cargo/audit.toml` added** — `cargo audit` and `cargo deny` read separate config; only the latter had documented exceptions
- [x] **quick-xml 0.31 (RUSTSEC-2026-0194/0195) cleared** — migrated `src/storage/azure.rs` to the Azure SDK for Rust v1 (`azure_storage_blob`), which uses quick-xml 0.41 and drops `http-types`. Breaking: shared-key auth (`AZURE_STORAGE_KEY`) has no equivalent in the v1 SDK; SAS and Entra ID are supported instead.
- [x] **Both advisory ignore lists are empty** — `deny.toml` and `.cargo/audit.toml` carry no suppressions; six further ignores no longer matched the dependency graph and were removed

### Conversion correctness

- [x] **`iv convert` works on vaulted models at all** — it parsed the stored format name (`"PyTorch"`) with `from_extension`, so every conversion failed with "No conversion path"; `iv diff` silently degraded to a byte diff for the same reason
- [x] **`ModelFormat::from_name` / `from_stored`** with a round-trip test across all 22 variants

### Conversion honesty

- [x] **REST `/convert` no longer returns plan JSON as target-format bytes** — it set `target_format: "onnx"` on a base64 payload that was actually a JSON plan; clients decoding it produced corrupt files
- [x] **`ConversionResult::plan` / `is_plan()` / `Converter::produces_plan()`** — a typed signal, replacing the CLI's habit of sniffing its own output for a `"converter"` key
- [x] **Multi-step planning halts at the first external-tooling step** — it used to feed a plan into the next converter and emit a plan-of-a-plan
- [x] **`.well-known/openapi.yaml` `ConversionResult` corrected** — the published schema described fields the endpoint never returned
- [x] **README / AGENTS.md conversion claims narrowed** to what actually converts natively

### Release consistency

- [x] **Python package version synced to the crate** — package was 1.3.0, its test asserted 1.2.1, crate was 1.6.0; all now 1.7.0 and the test compares against `Cargo.toml`
- [x] **`--key` accepts KMS URIs** for `iv sign` / `iv verify` — the last documented-but-missing claim
- [x] **15 orphaned docs added to mkdocs nav**
- [x] **`mkdocs build --strict` actually passes** — it failed with 51 warnings; 52 out-of-`docs/` links now use absolute GitHub URLs, and CI installs the `minify` plugin the config requires

## v1.6.0 — Test Coverage, Benchmarks & Documentation (complete)

Strengthen the project's quality foundation: expand integration test coverage for v1.3–v1.5 modules, add feature-level performance benchmarks, and complete documentation site navigation.

### Test Coverage Expansion

- [x] **Module integration tests** (`tests/module_integration_tests.rs`, 51 tests) — Cross-module integration tests covering tags/search, access control, lineage DAG, plugins, profiles, policies, validation, webhooks, quantization, evaluation, scheduler, multi-vault, signing, scanning, diff, license scanning, benchmarks, GC, and cross-module workflows
- [x] **Property-based tests** (`tests/proptest_tests.rs`, 11 tests) — Proptest strategies for crypto round-trips (encrypt/decrypt, ciphertext overhead, determinism), format detection (never panics, known extensions, case insensitivity), version serialization round-trip, SHA-256 invariants
- [x] **Fuzz target expansion** (`fuzz/fuzz_targets/`, 3 new targets) — Pickle scanner (`fuzz_pickle_scanner`), diff engine (`fuzz_diff_engine`), model card parser (`fuzz_model_card_parser`)

### Performance Benchmarks

- [x] **Feature benchmarks** (`benches/feature_bench.rs`) — Criterion benchmarks for tags/search (100 models), ACL (50 principals), lineage graph (20-deep chains), plugins (20 installs), profiles, policies (50 models), validation (1–100 KB probes), webhooks (20 targets), signing (1–100 KB), scanning (1–100 KB), diff (1–100 KB), license scanning
- [x] **CI benchmark tracking** — `benchmarks` job in CI using `benchmark-action/github-action-benchmark` with 150% regression alert threshold, auto-push to `dev/bench`

### Documentation Completeness

- [x] **mkdocs nav expansion** — Added 8 missing docs to navigation: Examples, Model Download, Model Signing, Model Diffing, Engine Interop, Safety Scanning, License Scanning, Benchmarks
- [x] **mkdocs build validation** — `mkdocs` job in CI with `mkdocs build --strict`, Python 3.12 + mkdocs-material
- [x] **API reference generation** — Rustdoc auto-generated in CI docs job, copied to `website/public/mkdocs/api/`, uploaded as artifact, linked in mkdocs nav

### Maintenance

- [x] **Import fixes** — Restored incorrectly removed imports in `vault.rs` (`VersionRepo`) and `database.rs` (`ChunkInfo`, `Document`)
- [x] **Dependency audit** — `aws-lc-sys` upgraded to v0.39.1 (fixed RUSTSEC-2026-0044, RUSTSEC-2026-0048), 6 unmaintained transitive dep warnings documented in `deny.toml` ignore list, `cargo deny check` and `cargo audit` pass clean
- [x] **MSRV validation** — MSRV updated from 1.75 to 1.89 (ecosystem deps require edition 2024: `time-macros`, `async-graphql-value`, `asynk-strim`), verified with `cargo +1.89 check --features "full,graphql"`

---

## Completed (v0.1.0)

- [x] Core vault: create, unlock, store, retrieve, delete, verify
- [x] FIPS 140-3 encryption (AES-256-GCM, Argon2id KDF, SHA-256)
- [x] Persistent salt storage for reproducible key derivation
- [x] Passphrase change with full re-encryption
- [x] Version control with lineage tracking (JSON persistence)
- [x] XDG-compliant configuration (Linux/macOS/Windows)
- [x] 23+ model format detection (PyTorch, ONNX, SafeTensors, GGUF, etc.)
- [x] Compression (gzip, LZMA, zlib) with analysis
- [x] Cloud storage backends (AWS S3, Azure Blob) via async StorageBackend trait
- [x] CLI with 18+ commands (clap 4.4, `iv` binary)
- [x] Cloud CLI wired to real push/pull/list operations
- [x] Model card generation (JSON, YAML, Markdown)
- [x] RAG system: DocumentStore, KnowledgeBase, RuleEngine, RetrievalCache
- [x] MCP tool server with 4 builtin tools (search, chunk, add_doc, execute_rule)
- [x] Database backends: SQLite (bundled), Sled KV, InMemory, Qdrant stub
- [x] SQL injection prevention (identifier validation)
- [x] Mutex safety (no `.lock().unwrap()` in production code)
- [x] Utility suite: archive, compress, deduplicate, export, analyze, quantize, prune
- [x] Audit logging for compliance
- [x] Compliance checks: CVE scanning via `cargo audit`, FIPS/CMMC/MITRE assessment
- [x] Python bindings: neuralvault package (Vault, VaultConfig, ModelFormat)
- [x] 1,809 tests passing, zero warnings
- [x] Git repository initialized
- [x] 10 example programs, 30+ documentation files

---

## v0.1.1 — Hardening (complete)

All fixes identified by audit. No new features.

### Critical

- [x] **Remove panicking `.expect()` from production paths**
  - `VaultConfig::default()` calls `.expect()` — documented with `///` warning
  - `FipsCrypto::default()` and `KeyManager::default()` same — documented
  - `Vault::new()` changed to use `match` returning `Result` instead of panicking

- [x] **Guard `validate_sql_identifier()` against empty-string panic**
  - Changed `.unwrap()` to `.expect("BUG: empty check above should have returned")`

- [x] **Add Python tests**
  - Created `tests/test_neuralvault.py` with 40+ tests
  - Covers ModelFormat, VaultConfig, Vault, FIPSCrypto

### Important

- [x] **Fix deprecated GitHub Actions in CI**
  - Replaced `actions-rs/toolchain@v1` → `dtolnay/rust-toolchain@stable`
  - Replaced `actions/create-release@v1` → `softprops/action-gh-release@v2`
  - Fixed binary name `iv` → `iv` in release.yml
  - Updated all 3 workflows: ci.yml, security.yml, release.yml

- [x] **Document Rust/Python crypto mismatch**
  - Added warning docstring to `fips.py` explaining PBKDF2 vs Argon2id incompatibility
  - `vault.py` already documents that it delegates to `iv` CLI

- [x] **Sync Python `ModelFormat` enum with Rust**
  - Rewrote registry.py to be 1:1 mirror of Rust's 23-variant enum
  - Removed Python-only formats (JAX, FLAX, SKLEARN, etc.)
  - Added missing Rust formats (MLX, TVM, MNN, NCNN, RKNN, HDF5, etc.)

- [x] **Add missing doc comments on public types**
  - Added `///` to 7 config structs, 4 model_card builder methods
  - Added `///` to 6 formats.rs items, StorageConfig variants
  - Verified rag.rs items already had docs

- [x] **Add `#[must_use]` annotations on pure functions**
  - Applied to all 15 pure functions across utils.rs, crypto/mod.rs, vault.rs

### Minor

- [x] **Fix inconsistent test count claims in README**
  - Updated all references from 171/119 → 227
  - Added model card test counts to breakdown

- [x] **Update stale roadmap section in README**
  - Replaced inline checklist with link to ROADMAP.md

- [x] **Make heavyweight Python deps optional in pyproject.toml**
  - Moved torch, tensorflow, onnx, etc. to `[project.optional-dependencies] ml`

- [x] **Commit `Cargo.lock` for reproducible binary builds**
  - Removed from .gitignore with explanatory comment

---

## v0.2.0 — Code Quality & Architecture (complete)

Refactoring, quality improvements, and project cleanup.

- [x] **Split `rag.rs` (2,168 lines) into submodules**
  - `rag/mod.rs` — re-exports
  - `rag/documents.rs` — DocumentStore, Document, ChunkInfo
  - `rag/knowledge.rs` — KnowledgeBase, KnowledgeBaseConfig
  - `rag/rules.rs` — RuleEngine, Rule, RuleCondition, RuleAction
  - `rag/cache.rs` — RetrievalCache, CacheStats
  - `rag/database.rs` — Database trait, InMemoryDatabase, SQLiteDatabase, SledDatabase
  - `rag/mcp.rs` — MCPServer, MCPTool, ToolExecutor, ToolContext, ToolResult
  - `rag/vector.rs` — VectorStore, SimpleVectorStore, QdrantVectorStore

- [x] **Split `main.rs` (2,900+ lines) into submodules**
  - `cli/mod.rs` — CLI entry point, argument parsing
  - `cli/commands.rs` — Command enum definitions
  - `cli/handlers/` — One file per command group (cloud, card, convert, db, etc.)

- [x] **Resolve all 5 `#[allow(dead_code)]` suppressions**
  - Removed redundant `CachedResult.query_hash` field
  - Used timestamp in LRU eviction as tiebreaker
  - Added `VersionControl::vault_path()` getter
  - Added `Vault::key_manager()` getter
  - Gated `ComplianceChecker` methods with `enabled_checks` map

- [x] **Optimize string building in `model_card.rs`**
  - Replace `push_str(&format!(...))` with `write!(md, ...)` via `std::fmt::Write`
  - `String::with_capacity(2048)`, `add_metadata` uses `impl Into<String>`

- [x] **Make `ModelFormat::name()` return `&'static str`**
  - Zero-allocation for both `name()` and `extension()`

- [x] **Add missing test coverage (+19 tests, 246 total)**
  - `vault.rs` — `change_passphrase` (security-critical re-encryption)
  - `audit.rs` — `read_entries`, `log_auth`, `log_security_violation`
  - `formats.rs` — `FormatConverter` register, can_convert, convert, error paths
  - `version.rs` — `cleanup_old_versions`, `verify_checksum`
  - `compliance.rs` — `set_check_enabled`, `is_check_enabled`

- [x] **Add benchmarks (`vault_bench`)**
  - Store/retrieve throughput
  - Format detection
  - SHA-256 hashing
  - Model card serialization/deserialization

- [x] **License: Switch from MIT to AGPL-3.0-or-later dual-license**
  - GNU Affero General Public License v3.0 or later for open-source use
  - Commercial license option (COMMERCIAL_LICENSE.md)

- [x] **Root directory cleanup (~80 → ~30 entries)**
  - Deleted 10 temporary artifacts
  - Moved 23 status/completion files → `reports/`
  - Moved 12 guides/demos/scripts → `docs/`

---

## v0.3.0 — Python Bindings (PyO3) (complete)

Native Rust-backed Python bindings replacing CLI-wrapper architecture.

- [x] **PyO3/maturin integration**
  - Added `pyo3 = { version = "0.22", features = ["extension-module"], optional = true }` behind `python` feature flag
  - Configured maturin as build backend in `pyproject.toml` (replaced setuptools)
  - `module-name = "neuralvault._native"` for clean native import

- [x] **Native Python API (`src/python.rs`, ~640 lines)**
  - `Vault` — create, unlock, lock, store_model, get_model, list_models, list_versions, get_lineage, delete_version, get_stats, change_passphrase
  - `VaultConfig` — XDG-compliant config with optional custom vault_dir
  - `ModelFormat` — 23+ format detection, name/extension properties
  - `ModelMetadata` — builder-style constructor with description, framework, task, architecture, parameters
  - `ModelVersion` — read-only version snapshot (version, checkpoint_id, timestamp, format, size, checksum)
  - `ModelCard` — create, set_training_data, add_metric, add_metadata, to_json/to_yaml/to_markdown, from_json/from_yaml
  - `sha256_hex()` — FIPS-compliant SHA-256 digest
  - `version()` — native library version string

- [x] **Python `__init__.py` with native import + fallback**
  - Imports from `_native` module when available (`_NATIVE = True`)
  - Falls back to pure-Python CLI wrappers for source installs without Rust

- [x] **Python documentation**
  - Sphinx API reference (conf.py, index.rst, 5 API pages, 4 guide pages)
  - Quick start and installation guides (uv-based)

- [x] **Streaming support for large models**
  - `ModelStream` iterator (Rust + PyO3) with configurable chunk size
  - `Vault.store_model_streamed()` for chunked ingest
  - `Vault.get_model_streamed()` for chunked retrieval (default 8 MiB)

---

## v0.4.0 — Format Conversion ✅

Real model format conversion (not just export + guidance).

- [x] **Conversion pipeline architecture**
  - `Converter` trait with `convert(data, options, progress) -> Result`
  - `ConversionPipeline` with BFS multi-step path finding
  - Plugin system: `register(Box<dyn Converter>)` for custom converters
  - Progress reporting via `ProgressCallback` + `ConversionProgress` display

- [x] **Priority conversions** (10 built-in converters)
  - SafeTensors ↔ Raw (pure Rust)
  - SafeTensors ↔ PyTorch (shim/plan)
  - PyTorch → ONNX (shim/plan, configurable opset)
  - ONNX → TensorRT (shim/plan)
  - ONNX → CoreML (shim/plan)
  - SafeTensors → GGUF (shim/plan, quantization support)
  - GGUF header/metadata parser (pure Rust)
  - ONNX metadata extractor (pure Rust)

- [x] **Validation**
  - Magic-bytes integrity checks (SafeTensors, GGUF, PyTorch, ONNX, TFLite)
  - Size-ratio validation
  - `ValidationReport` + `ValidationCheck` structures
  - `--validate` CLI flag

- [x] **CLI integration**
  - `iv convert` with `--opset`, `--validate`, `--plan-only` flags
  - `iv list-conversions` command
  - 31 integration tests + 22 unit tests

---

## v0.5.0 — API & Web Interface ✅

Network-accessible vault management.

- [x] **REST API** (axum 0.7)
  - 14 endpoints: health, auth, models CRUD, versions, lineage, conversions, convert, stats, audit
  - JWT authentication with `Authorization: Bearer` header
  - OpenAPI 3.1 specification at `/api/v1/openapi.json`
  - CORS support (`--cors-permissive` flag) and request body limits (512 MiB default)
  - `api` feature flag — zero cost when unused

- [x] **Web dashboard**
  - Embedded single-page HTML application at `/`
  - Model inventory browser with version drill-down
  - Storage usage statistics (models, versions, size, files)
  - Audit log viewer (newest first)
  - Conversion registry browser
  - Passphrase-based login with JWT session

- [x] **CLI integration**
  - `iv serve` with `--host`, `--port`, `--jwt-secret`, `--token-expiry`, `--cors-permissive`, `--no-dashboard`
  - Environment variable support: `IRONVAULT_HOST`, `IRONVAULT_PORT`, `IRONVAULT_JWT_SECRET`

- [x] **GraphQL API** (`graphql` feature flag)
  - `async-graphql` 7.0 integration with axum
  - Queries: models, model, versions, lineage, stats, audit_log, conversions, health, version
  - Mutations: store_model, delete_model, delete_version, convert_model, unlock, lock
  - GraphQL Playground at `/graphql`

---

## v1.0.0 — Production Release ✅

- [x] FIPS 140-3 CMVP validation (formal, if needed)
- [x] Security audit by third party
- [x] crates.io publication
- [x] PyPI publication
- [~] Docker images (alpine, debian) — shipped in 1.0, **removed in 4.5.0**
- [~] Kubernetes Helm chart — shipped in 1.0, **removed in 4.5.0**
- [x] Comprehensive migration guide from v0.x
- [x] Long-term support commitment

---

## v1.1.0 — Advanced Features ✅

Distributed systems and hardware acceleration.

- [x] **GraphQL API** (`graphql` feature flag)
  - Full async-graphql 7.0 integration with axum
  - Queries: models, model, versions, lineage, stats, audit_log, conversions, health, version
  - Mutations: store_model, delete_model, delete_version, convert_model, unlock, lock
  - GraphQL Playground at `/graphql`

- [~] **GPU-Accelerated Encryption** — *removed after v1.7.0, not planned*
  - The OpenCL `gpu` feature never compiled: launching a kernel requires an
    `unsafe` block and the crate sets `unsafe_code = "forbid"`. Because it never
    built, its hand-written AES-256-CTR kernel was never executed or checked
    against NIST known-answer vectors.
  - Shipping unvalidated hand-rolled AES was a worse trade than losing the
    feature, so the module, the flag, and the `ocl` dependency were dropped.
    This also closed audit findings C-01 (unauthenticated CTR), C-02 (AES key
    left resident in GPU memory) and C-03 (unsafe OpenCL FFI).
  - If GPU offload is revisited, it should wrap a validated implementation
    rather than reimplement the cipher.

- [x] **Federated Vault Synchronization** (~800 lines)
  - Vector clocks for causal ordering (`VectorClock`, `ClockComparison`)
  - Peer configuration and discovery (`FederationConfig`, `PeerConfig`)
  - Sync protocol with delta computation (`FederationManager`)
  - Manifest generation and comparison (`SyncManifest`, `ModelManifestEntry`)
  - Conflict detection and resolution strategies (`SyncConflict`, `ConflictResolution`)

- [x] **Blockchain-Based Audit Trail** (~650 lines)
  - Merkle tree implementation with proof generation/verification
  - Audit block structure with hash chain integrity
  - JSON-based persistence with file-per-block storage
  - Complete chain verification (`ChainVerification`)
  - Cryptographic proof from entry to genesis (`AuditProof`)
  - Auto-finalization based on block size threshold

---

## v2.0.0 — Architecture v2 ✅

Internal architecture overhaul based on `docs/ARCHITECTURE_V2.md`. Introduces trait-based
dependency injection, a domain event system, streaming encryption, and a SQLite version
repository — all backward-compatible with existing CLI and API surfaces.

### Phase 1: Trait Extraction ✅

Extracted four core domain traits into `src/traits.rs` with concrete implementations on
existing types. Every subsystem now programs against a trait boundary.

- [x] **`CryptoProvider` trait** — `derive_key`, `encrypt`, `decrypt`, `hash`, `hash_hex`, `random_bytes`
  - Implemented by `FipsCrypto` (`src/crypto/mod.rs`)
- [x] **`BlobStore` trait** — `put`, `get`, `remove`, `exists`, `size`, `list_keys`, `stats`
  - Implemented by `Storage` (`src/storage.rs`)
- [x] **`VersionRepo` trait** — `add_version`, `get_version`, `list_versions`, `get_lineage`, `delete_version`, `cleanup_old_versions`, `verify_checksum`, `update_metadata`, `get_metadata`, `list_models`
  - Implemented by `VersionControl` (`src/version.rs`)
- [x] **`AuditSink` trait** — `emit`, `query`
  - Implemented by `AuditLogger` (`src/audit.rs`)
- [x] **`NullAuditSink`** — no-op implementation for testing / disabled audit
- [x] **`BlobStoreStats`** — storage statistics returned by `BlobStore::stats()`
- [x] **Re-exports in `lib.rs`** — all trait types are top-level public API

### Phase 2: Event System ✅

Domain event infrastructure for audit, metrics, and agent observability.

- [x] **`VaultEvent` enum** — 9 variants: `VaultCreated`, `VaultUnlocked`, `VaultLocked`, `ModelStored`, `ModelRetrieved`, `ModelDeleted`, `PassphraseChanged`, `IntegrityFailed`, `ComplianceChecked`
  - Helper methods: `timestamp()`, `vault_name()`, `event_type()`
- [x] **`EventBus`** — subscriber registry with `register()` + `dispatch()`, error-swallowing
- [x] **`EventSubscriber` trait** — `accepts()`, `on_event()`, `name()`
- [x] **`AuditLogSubscriber`** — converts `VaultEvent` → `AuditEntry`, forwards to `AuditSink`
- [x] **`VaultMetrics`** — atomic counters: models stored/retrieved/deleted, bytes stored, errors, vault unlocked flag
- [x] **`MetricsSubscriber`** — receives events and increments `VaultMetrics` counters
- [x] **`MetricsSnapshot`** — serializable point-in-time metrics snapshot
- [x] **`Vault` wiring** — `EventBus` integrated into `Vault` struct, events emitted in `unlock()`, `lock()`, `store_model()`, `get_model()`, `delete_version()`, `change_passphrase()`
- [x] **`VaultState` enum** — `Uninitialized`, `Locked`, `Unlocked`, `Error` — queryable via `Vault::state()`
- [x] **`Vault::event_bus()` / `event_bus_mut()`** — accessors for subscriber registration

### Phase 3: Agent-Addressable URIs ✅

- [x] **`IvUri` struct + parser** — full `iv://` URI scheme
  - Format: `iv://{vault}/{model}@{version}/{resource}?{query}`
  - Supports: root, vault, model, model@version, resource, query params
  - Roundtrip serialization (`Display` + `parse`)
  - 9 unit tests covering all URI forms

### Phase 4: Streaming Encryption ✅

Chunked encryption for large models — constant 8 MiB memory budget regardless of model size.

- [x] **`src/crypto/streaming.rs`** — `encrypt_chunked` / `decrypt_chunked` / `is_chunked_format`
- [x] **Wire format** — `[header: 32B][chunk_0: nonce(12)+ciphertext+tag(16)]...[stream_mac: 32B]`
- [x] **`StreamHeader`** — magic bytes `IronVault`, version, chunk size, total chunks, original size
  - Roundtrip serialization (`to_bytes` / `from_bytes`)
- [x] **Stream MAC** — SHA-256 over all chunk auth tags + chunk count (prevents truncation/reordering)
- [x] **Default chunk size** — 4 MiB (tuned for SSD page alignment)
- [x] **7 unit tests** — small data, exact boundaries, 1 MB, tamper detection, empty data, invalid magic

### Phase 5: Repository & Observability ✅

SQLite-backed version storage with ACID guarantees and auto-migration.

- [x] **`SqliteVersionRepo`** (`src/version_sqlite.rs`) — full `VersionRepo` trait implementation
  - SQLite WAL mode for concurrent reads
  - Indexed tables: `versions` (model_name, version) + `version_metadata` (key-value)
  - In-memory cache for reference-returning trait methods
- [x] **Auto-migration from `versions.json`** — on first open, imports legacy JSON, renames to `.migrated`
- [x] **`SqliteVersionRepo::in_memory()`** — for testing
- [x] **6 unit tests** — CRUD, cleanup, metadata, list_models, JSON migration
- [x] **Re-exports in `lib.rs`** — `SqliteVersionRepo` is top-level public API (behind `sqlite` feature)

### Phase 6: Async Unification ✅

Unified async blob storage trait bridging local and cloud backends.

- [x] **`AsyncBlobStore` trait** — async counterpart to sync `BlobStore`
  - Methods: `put`, `get`, `delete`, `exists`, `list`, `stat`
  - Returns `BlobReceipt` (key, size, timestamp) on put
  - Returns `BlobInfo` (key, size) on list/stat
- [x] **`AsyncBlobStoreAdapter<B: StorageBackend>`** — wraps any existing `StorageBackend` implementation
  - Bridges S3, Azure, Local async backends to the new trait
  - Zero-cost when unused (generic, monomorphized)
- [x] **`BlobReceipt` / `BlobInfo`** — typed return values for async storage operations
- [x] **Re-exports in `lib.rs`** — all async types are top-level public API

### Phase 7: API Observability ✅

REST API endpoints for agent introspection and monitoring.

- [x] **`GET /api/v1/metrics`** — vault metrics: state, model count, version count, storage bytes
- [x] **`GET /api/v1/events`** — audit events with filtering
  - `?limit=N` — maximum events to return
  - `?type=ModelStored` — filter by event type
  - Returns newest-first ordering
- [x] **Enhanced `GET /api/v1/health`** — now returns `vault_state`, `model_count` alongside status/version
- [x] **Route registration** — new endpoints wired into axum Router

### Phase 8: Pipeline Wiring ✅

Wired v2 components (VersionBackend, VaultBuilder, streaming) into the live vault pipeline.

- [x] **`VersionBackend` enum** — JSON / SQLite dispatch in vault.rs
- [x] **`VaultBuilder`** — fluent builder with `.config()`, `.sqlite_versions()`, `.subscriber()`, `.no_default_subscribers()`
- [x] **`Vault::version_backend_name()`** — reports active backend for CLI/API
- [x] **Streaming threshold** — `StorageSettings::streaming_threshold` in config for auto chunked I/O
- [x] **`store_streamed()` / `retrieve_auto()`** — hybrid streaming path in Storage

### Phase 9: CLI & Python Integration ✅

Surfaced VaultBuilder and backend selection to CLI users and Python consumers.

- [x] **`--sqlite-versions` CLI flag** — global arg on `Cli` struct (env: `IRONVAULT_SQLITE_VERSIONS`)
- [x] **`build_vault()` helper** — replaces all 20 `Vault::new()` call sites in CLI handlers
- [x] **Handler updates** — vault, analyze, archive, cloud, convert, card handlers accept `use_sqlite`
- [x] **main.rs dispatch** — extracts `use_sqlite` flag (feature-gated) and passes to all handlers
- [x] **Default EventBus subscribers** — `AuditLogSubscriber` + `MetricsSubscriber` auto-wired in `VaultBuilder::build()`
- [x] **`PyVaultBuilder`** — Python bindings: `VaultBuilder().config(cfg).sqlite_versions().build()`
- [x] **`basic_usage` example** — updated to showcase `VaultBuilder` pattern

### Phase 10: Testing & Bug Fixes ✅

Fixed critical wiring bugs and added comprehensive test coverage for v2 components.

- [x] **Fixed `AuditLogSubscriber`** — was wired with `NullAuditSink`, now uses real `AuditLogger` (events actually reach audit log)
- [x] **Fixed `VaultMetrics` exposure** — `Arc<VaultMetrics>` stored on `Vault`, exposed via `pub fn metrics() -> Option<MetricsSnapshot>`
- [x] **30 VaultBuilder integration tests** (`tests/vault_builder_tests.rs`):
  - Builder construction (JSON/SQLite backends, default/custom subscribers)
  - Metrics lifecycle (store, retrieve, delete operations update counters)
  - Event emission validation (custom subscriber receives ModelStored/Retrieved/Deleted)
  - Streaming API round-trip (`store_model_streamed`, `get_model_chunked`)
  - SQLite backend parity (CRUD, versioning, list, delete)
  - IvUri parsing and validation
  - Audit log file written via EventBus subscriber pipeline
- [x] **17 CLI integration tests** (`tests/cli_tests.rs`) using `assert_cmd`:
  - Help/version output, subcommand help for 5 commands
  - Vault lifecycle (init, list, stats, compliance)
  - Error cases (missing args, unknown subcommand)
  - `--sqlite-versions` flag accepted

### Test Results

- **391 tests total** (105 unit + 22 config + 31 conversion + 19 coverage + 14 crypto + 15 format + 8 integration + 30 vault_builder + 17 cli + 4 model_card_integration + 48 model_card + 38 rag + 38 utils + 2 doc-test)
- **0 failures**, 0 warnings
- Backward-compatible — no existing API surface changed

### Files Created

| File                           | Lines  | Purpose                                                           |
| ------------------------------ | ------ | ----------------------------------------------------------------- |
| `src/traits.rs`                | ~1,070 | Core traits, event system, URI parser, metrics, async BlobStore   |
| `src/crypto/streaming.rs`      | ~300   | Chunked encryption with stream MAC                                |
| `src/version_sqlite.rs`        | ~470   | SQLite-backed VersionRepo + migration                             |
| `tests/vault_builder_tests.rs` | ~700   | 30 integration tests: builder, metrics, events, streaming, SQLite |
| `tests/cli_tests.rs`           | ~200   | 17 CLI integration tests with `assert_cmd`                        |

### Files Modified

| File                      | Change                                                                       |
| ------------------------- | ---------------------------------------------------------------------------- |
| `src/lib.rs`              | Added `mod traits`, `mod version_sqlite`, comprehensive re-exports           |
| `src/crypto/mod.rs`       | `impl CryptoProvider for FipsCrypto`, `pub mod streaming`                    |
| `src/audit.rs`            | `impl AuditSink for AuditLogger`                                             |
| `src/version.rs`          | `impl VersionRepo for VersionControl`                                        |
| `src/storage.rs`          | `impl BlobStore for Storage`                                                 |
| `src/vault.rs`            | Added `EventBus`, `VaultState`, event emission in all state-changing methods |
| `src/api/routes.rs`       | Added `/metrics`, `/events` endpoints; enhanced `/health` with vault state   |
| `src/api/server.rs`       | Registered `/metrics`, `/events` routes                                      |
| `src/vault.rs`            | `VaultBuilder`, `VersionBackend`, `metrics()`, EventBus subscriber wiring    |
| `src/cli/args.rs`         | Added `--sqlite-versions` flag                                               |
| `src/cli/helpers.rs`      | Added `build_vault()` helper                                                 |
| `src/cli/handlers/*.rs`   | All 6 handler files updated to use `build_vault()`                           |
| `src/main.rs`             | Extract `use_sqlite`, pass to all 18 handler calls                           |
| `src/python.rs`           | Added `PyVaultBuilder` class                                                 |
| `examples/basic_usage.rs` | Updated to use `VaultBuilder` pattern                                        |

---

## v1.5.0 — Quantization, Evaluation, Backup & Multi-Vault ✅

4 new modules for model optimization, evaluation, backup scheduling, and multi-vault management.

- [x] **Quantization Pipeline** (`src/quantization.rs`, ~250 lines) — Profile-based quantization management with method selection (Q4_0, Q4_K_M, Q5_K_M, Q8_0, F16, F32), size estimation, and batch reporting. `QuantProfileStore` with `set`/`remove`/`get`/`list`. CLI: `iv quantize set/remove/list/estimate`
- [x] **Evaluation Harness** (`src/evaluation.rs`, ~250 lines) — Record, compare, and query model evaluation results across suites and metrics. `EvalStore` with `record`/`get_runs`/`compare`/`suites`/`count`. CLI: `iv eval record/list/compare/suites`
- [x] **Backup Scheduling** (`src/scheduler.rs`, ~200 lines) — Configurable vault backup schedules with cron-style intervals (daily, weekly, monthly, custom hours). `BackupScheduler` with `set_schedule`/`list_schedules`/`run_backup`/`history`. CLI: `iv backup schedule/list/run/history`
- [x] **Multi-Vault Registry** (`src/multi_vault.rs`, ~200 lines) — Manage multiple named vaults from a single installation. `MultiVaultRegistry` with `register`/`unregister`/`list`/`activate`/`active`. CLI: `iv vaults register/unregister/list/activate/active`
- [x] **CLI expansion** — 42+ commands (was 38+), 4 new CLI handler files
- [x] **API expansion** — 12 new REST endpoints for quantization, evaluation, backup, multi-vault, and vault management
- [x] **Python expansion** — 4 new PyO3 classes (`PyQuantProfileStore`, `PyEvalStore`, `PyBackupScheduler`, `PyMultiVaultRegistry`)
- [x] **Test expansion** — 1,932 tests (was 1,865), 67 new tests from 4 modules + 20 CLI integration tests
- [x] **Documentation** — 4 new docs (QUANTIZATION.md, EVALUATION.md, BACKUP_SCHEDULING.md, MULTI_VAULT.md)
- [x] **Benchmarks** — New `module_bench.rs` with criterion benchmarks for all 4 modules

### Files Created

| File                                 | Lines | Purpose                                      |
| ------------------------------------ | ----- | -------------------------------------------- |
| `src/quantization.rs`                | ~250  | Quantization pipeline & profile store        |
| `src/evaluation.rs`                  | ~250  | Evaluation harness                           |
| `src/scheduler.rs`                   | ~200  | Backup scheduling                            |
| `src/multi_vault.rs`                 | ~200  | Multi-vault registry & switching             |
| `src/cli/handlers/quantization.rs`   | ~60   | CLI handler for quantization commands        |
| `src/cli/handlers/evaluation.rs`     | ~60   | CLI handler for evaluation commands          |
| `src/cli/handlers/scheduler.rs`      | ~50   | CLI handler for backup scheduling            |
| `src/cli/handlers/multi_vault.rs`    | ~50   | CLI handler for multi-vault management       |
| `docs/QUANTIZATION.md`              | ~130  | Quantization pipeline documentation          |
| `docs/EVALUATION.md`                | ~100  | Evaluation harness documentation             |
| `docs/BACKUP_SCHEDULING.md`         | ~100  | Backup scheduling documentation              |
| `docs/MULTI_VAULT.md`               | ~100  | Multi-vault management documentation         |
| `benches/module_bench.rs`            | ~100  | Criterion benchmarks for 4 new modules       |
| `website/src/app/features/quantization/page.tsx` | ~60 | Website feature page             |
| `website/src/app/features/evaluation/page.tsx`   | ~60 | Website feature page             |
| `website/src/app/features/backup/page.tsx`       | ~60 | Website feature page             |
| `website/src/app/features/multi-vault/page.tsx`  | ~60 | Website feature page             |

### Files Modified

| File                    | Change                                                              |
| ----------------------- | ------------------------------------------------------------------- |
| `Cargo.toml`            | Version bump 1.4.0 → 1.5.0, added `module_bench` bench target      |
| `src/lib.rs`            | 4 new `pub mod` declarations, re-exports for new public types       |
| `src/cli/args.rs`       | 4 new Command variants (`Quantize`, `Eval`, `Backup`, `Vaults`)     |
| `src/cli/handlers/mod.rs` | 4 new `pub mod` declarations for handler files                   |
| `src/main.rs`           | Imports and match arms for 4 new command variants                   |
| `src/api/routes.rs`     | 12 new handler functions for v1.5.0 endpoints                       |
| `src/api/server.rs`     | 12 new route registrations                                          |
| `src/python.rs`         | 4 new PyO3 classes added to module                                  |
| `tests/cli_tests.rs`    | 20 new integration tests for v1.5.0 CLI commands                   |
| `CHANGELOG.md`          | v1.5.0 entry added                                                  |

---

## v1.4.0 — Vault Management & Extensibility ✅

12 new features for vault operations, access control, extensibility, and model governance.

- [x] **Model Tags & Search** (`src/tags.rs`, ~250 lines) — Tag models with arbitrary labels and key-value annotations. Full-text search by name pattern, tags, or annotations. `TagStore` with `add_tags`/`remove_tags`/`search`. CLI: `iv tag add/remove/list/annotate`, `iv search`
- [x] **Vault Export/Import** (`src/vault_bundle.rs`, ~200 lines) — Export entire vaults (or filtered subsets) as portable tar.gz bundles. Import bundles into new vaults with overwrite control. CLI: `iv vault-export <OUTPUT>`, `iv vault-import <ARCHIVE> [TARGET]`
- [x] **Garbage Collection** (`src/gc.rs`, ~200 lines) — Detect orphaned blobs, stale temp files, and reclaimable storage. Dry-run mode for safe preview. `GcReport` with stats. CLI: `iv gc [--dry-run]`
- [x] **TUI Dashboard** (`src/tui.rs`, ~150 lines) — Terminal UI browser showing all vault models with version counts, sizes, formats, and timestamps in a formatted table. CLI: `iv browse`
- [x] **Webhooks** (`src/webhooks.rs`, ~250 lines) — HTTP notification targets for vault events. `WebhookStore` with add/remove/list/fire. Implements `EventSubscriber` for automatic dispatch on VaultEvent. CLI: `iv webhook add/remove/list/test`
- [x] **Access Control** (`src/access_control.rs`, ~200 lines) — Role-based ACL (Reader/Writer/Admin) per principal. `AclGuard` with grant/revoke/resolve/require. JSON persistence. CLI: `iv acl grant/revoke/list/check`
- [x] **KMS Integration** (`src/kms.rs`, ~150 lines) — Fetch vault passphrases from external secrets managers. `KmsBackend` enum supporting env vars, AWS Secrets Manager, Azure Key Vault, HashiCorp Vault. Library API only.
- [x] **Model Validation** (`src/validation.rs`, ~250 lines) — Integrity probes with SHA-256 checksums per model version. Record expected hashes, validate against stored files. `ValidationStore` with probe management. CLI: `iv validate <NAME> [--version V]`
- [x] **Retention Policies** (`src/policies.rs`, ~250 lines) — Configurable retention rules per model: max versions, max age, keep minimum. Dry-run enforcement. `PolicyStore` with apply/apply_all. CLI: `iv policy set/remove/list/apply/apply-all`
- [x] **Cross-Model Lineage DAG** (`src/lineage_graph.rs`, ~200 lines) — Directed acyclic graph tracking model derivation chains (fine-tune, quantization, distillation, merge, prune, conversion). `LineageGraph` with add_edge/ancestors/descendants/display. CLI: `iv lineage-graph add/show/ancestors/descendants`
- [x] **Plugin System** (`src/plugins.rs`, ~200 lines) — Discover, install, and uninstall plugins via JSON manifests. `PluginRegistry` with directory scanning, manifest validation, capability listing. CLI: `iv plugin discover/install/uninstall/list/info`
- [x] **Config Profiles** (`src/profiles.rs`, ~200 lines) — Named configuration profiles with activate/deactivate switching. Override vault settings per profile. `ProfileStore` with set/remove/activate/deactivate. CLI: `iv profile create/remove/list/activate/deactivate/show`
- [x] **CLI expansion** — 38+ commands (was 25+), 11 new CLI handler files
- [x] **Test expansion** — 1,865 tests (was 1,809), 56 new tests from 12 modules

### Files Created

| File                                | Lines | Purpose                                  |
| ----------------------------------- | ----- | ---------------------------------------- |
| `src/tags.rs`                       | ~250  | Model tagging and search                 |
| `src/vault_bundle.rs`               | ~200  | Vault export/import as portable archives |
| `src/gc.rs`                         | ~200  | Garbage collection for orphaned blobs    |
| `src/tui.rs`                        | ~150  | Terminal UI dashboard                    |
| `src/webhooks.rs`                   | ~250  | Webhook notification system              |
| `src/access_control.rs`             | ~200  | Role-based access control                |
| `src/kms.rs`                        | ~150  | External secrets manager integration     |
| `src/validation.rs`                 | ~250  | Model integrity validation               |
| `src/policies.rs`                   | ~250  | Retention policy enforcement             |
| `src/lineage_graph.rs`              | ~200  | Cross-model lineage DAG                  |
| `src/plugins.rs`                    | ~200  | Plugin system with JSON manifests        |
| `src/profiles.rs`                   | ~200  | Configuration profiles                   |
| `src/cli/handlers/tags.rs`          | ~80   | CLI handler for tag/search commands      |
| `src/cli/handlers/vault_bundle.rs`  | ~30   | CLI handler for vault export/import      |
| `src/cli/handlers/gc.rs`            | ~30   | CLI handler for garbage collection       |
| `src/cli/handlers/browse.rs`        | ~10   | CLI handler for TUI browse               |
| `src/cli/handlers/webhooks.rs`      | ~60   | CLI handler for webhook management       |
| `src/cli/handlers/acl.rs`           | ~50   | CLI handler for access control           |
| `src/cli/handlers/validation.rs`    | ~45   | CLI handler for model validation         |
| `src/cli/handlers/policies.rs`      | ~70   | CLI handler for retention policies       |
| `src/cli/handlers/lineage_graph.rs` | ~50   | CLI handler for lineage DAG              |
| `src/cli/handlers/plugins.rs`       | ~60   | CLI handler for plugin management        |
| `src/cli/handlers/profiles.rs`      | ~60   | CLI handler for config profiles          |

### Files Modified

| File                      | Change                                                             |
| ------------------------- | ------------------------------------------------------------------ |
| `Cargo.toml`              | Version bump 1.3.0 → 1.4.0                                         |
| `src/lib.rs`              | 12 new `pub mod` declarations, re-exports for all new public types |
| `src/version.rs`          | Added `list_models_owned()` and `import_version()` helper methods  |
| `src/cli/args.rs`         | 13 new Commands variants, 8 new subcommand enums                   |
| `src/cli/handlers/mod.rs` | 11 new `pub mod` declarations for handler files                    |
| `src/main.rs`             | Imports and match arms for all 13 new command variants             |

---

## Out of Scope (Current)

These are tracked but not planned for any specific release:

- Google Cloud Storage (blocked by RUSTSEC-2025-0009/0010 in `cloud-storage` crate)
- Model training integration

## Completed in v1.3.0

- [x] **Model download** — Pull models from HuggingFace Hub (`hf:`), Ollama registry (`ollama:`), or arbitrary URLs with streaming SHA-256 verification; `iv pull` CLI command
- [x] **Model signing & verification** — HMAC-SHA256 model signing with detached `.sig` files for provenance; `iv sign` / `iv verify` CLI commands
- [x] **Pickle safety scanning** — Detect 7 dangerous opcodes and 12 suspicious patterns in PyTorch/pickle files; `iv scan` CLI command
- [x] **Model diffing** — Tensor-level comparison for SafeTensors/GGUF with generic binary fallback; `iv diff` CLI command with `name@version` syntax
- [x] **Engine interop** — Register models with Ollama (`ollama create`) and LM Studio (copy to models dir); `iv register` CLI command
- [x] **Benchmark metadata** — Store and query benchmark results per model version with JSON filesystem storage; `iv benchmark add/show` CLI commands
- [x] **License scanning** — Detect licenses from model cards, config.json, GGUF metadata, LICENSE files; 24 known licenses with SPDX normalization; `iv license-scan` CLI command
- [x] **CLI expansion** — 25+ commands (was 15+), 63 CLI integration tests (was 17)

## Completed in v1.2.1

- [x] **Test coverage expansion** — 39 new Rust unit tests (API routes, error handling, rate limiter, domain errors); 623 lib tests total
- [x] **Python binding tests** — 21 new pytest tests (VersionControl, FIPSCrypto extended); 83 total, 85% Python coverage
- [x] **Fuzz testing** — 2 new fuzz targets (version_parsing, conversion_pipeline); 5 total
- [x] **CI/CD hardening** — MSRV 1.75 job, Python test job, fuzz CI, cargo-doc check, cargo-llvm-cov, security audit job
- [x] **Coverage** — 85.4% overall (15,187 / 17,786 lines), library-only ~92%

## Completed in v1.2.0

- [x] **Error type granularity** — Split the monolithic `VaultError` enum into domain-specific error types (`CryptoError`, `StorageError`, `ConversionError`) with `From` conversions into `VaultError`
- [x] **API endpoint expansion** — Added REST endpoints for model cards (`GET/POST /models/{name}/card`), compliance checks (`GET /compliance`), and RAG operations (`POST /rag/search`, `POST /rag/documents`)
- [x] **GraphQL routing** — Wired existing `async-graphql` schema into the Axum router at `/graphql` (Playground + query endpoint)

---

## How to Use This File

This file is the single source of truth for project status. Update it as work progresses:

```
- [ ] Task description     ← not started
- [~] Task description     ← in progress  
- [x] Task description     ← complete
```

Run `git log --oneline` to correlate commits with roadmap items.
