# AGENTS.md — AI Agent Discovery Guide

> Machine-readable project context for AI agents, LLM assistants, and automated tools.
> IronVault is **designed agent-first** — every capability is reachable from CLI, REST/GraphQL, and MCP, all derived from a single introspectable schema.

## Bootstrap in three commands

```bash
# 1. Get the full CLI schema (commands, flags, types, examples)
iv introspect --format json

# 2. List the 86 MCP tools (JSON Schema inputs)
cat .well-known/mcp-manifest.json | jq '.tools[] | {name, description}'

# 3. List the 56 REST endpoints
cat .well-known/openapi.yaml | grep -E '^  /api/v1/'
```

That is the minimum surface needed to plan a task. Everything else in this file is reference material.

## Worked examples for agents

Three runnable Rust examples cover the three canonical integration patterns:

| Example                                                            | Pattern                  | Shows                                                                                            |
| ------------------------------------------------------------------ | ------------------------ | ------------------------------------------------------------------------------------------------ |
| [`examples/agent_bootstrap.rs`](examples/agent_bootstrap.rs)       | Out-of-process via CLI   | Shell out to `iv introspect`, parse the schema, invoke a subcommand, handle the error envelope. |
| [`examples/agent_mcp_workflow.rs`](examples/agent_mcp_workflow.rs) | In-process via MCP       | Register vault-backed `MCPTool`s, drive them in an agent loop with JSON parameters.              |
| [`examples/agent_pipeline.rs`](examples/agent_pipeline.rs)         | Direct Rust API pipeline | End-to-end: scan → store → tag → search → sign → verify, emitting an audit envelope.             |

Run any of them with `cargo run --example <name>`.

## Running unattended

Passphrase-gated commands (`store`, `get`, `list`, `sign`, `cloud *`, …) resolve the
passphrase in this order, so no TTY is required:

1. `$IRONVAULT_PASSPHRASE` — a literal passphrase, or a KMS URI to resolve
2. A line piped on stdin, when stdin is not a terminal
3. An interactive masked prompt

```bash
IRONVAULT_PASSPHRASE='vault://secret/iv/passphrase' iv list --format json
printf '%s\n' "$PASSPHRASE" | iv list
```

An unresolvable KMS URI is a hard error — the CLI never falls back to an empty
passphrase. See [docs/KMS.md](docs/KMS.md) for the URI table and backend setup.

## Stability contract

| Guarantee         | Detail                                                                                                                                  |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| JSON output       | `list`, `versions`, `lineage`, `stats`, `compliance`, `search`, `scan`, `diff`, `license-scan` and `introspect` accept `--format json`. The grouped reads (`cloud list`, `database list`, `acl list`) are text-only. Schemas evolve with semver; breaking changes bump the major version of `ironvault`. |
| Exit codes        | `0` ok · `1` general · `2` auth failed · `3` not found · `4` permission denied · `5` integrity · `6` invalid input · `7` config · `8` compliance |
| Idempotent reads  | `list`, `get`, `search`, `versions`, `lineage`, `stats`, `compliance`, `introspect`, every `*/show` and `*/list` are side-effect free   |
| Destructive gates | `delete`, `policy apply`, `gc`, `vault-import` either require explicit names or accept `--dry-run`                                      |
| Error envelope    | Errors emit JSON `{ "code": "...", "message": "...", "hint": "..." }` on stderr; never bare strings                                     |
| No surprise I/O   | The CLI never makes network calls except `iv pull`, `iv cloud *`, and opt-in telemetry (off by default; honors `DO_NOT_TRACK=1`)      |
| URI scheme        | `iv://vault/model@version` resolves through any of the three surfaces                                                                 |
| Conversion honesty | `iv convert` and `POST /api/v1/convert` never emit a file or payload in the target format unless the bytes really are that format. When external tooling is required the REST response sets `converted: false` and carries a `plan`; the CLI writes `<output>.plan.json` and produces no target-format file |

## Project Identity

| Key            | Value                                    |
| -------------- | ---------------------------------------- |
| **Name**       | IronVault                           |
| **Binary**     | `iv`                                    |
| **Crate**      | `ironvault`                         |
| **Version**    | 7.2.0                                    |
| **Language**   | Rust (edition 2021, MSRV 1.89)           |
| **License**    | AGPL-3.0-or-later                        |
| **Repository** | https://github.com/nervosys/IronVault |

## What This Project Does

IronVault is an **encrypted AI/ML model management system**. It provides:

1. **Encrypted Storage** — AES-256-GCM encryption with Argon2id key derivation. FIPS-approved algorithms, but not a FIPS-validated module; see the compliance table below
2. **Version Control** — Sequential versioning with parent lineage trees and instant rollback
3. **Format Conversion** — 23+ formats detected. Native pure-Rust conversion for SafeTensors ↔ PyTorch, SafeTensors ↔ raw, and HuggingFace → GGUF (`iv convert --from-dir`; llama architecture, F16/BF16/F32); PyTorch→ONNX, ONNX→TensorRT, ONNX→CoreML and GGUF K-quants require an external Python toolchain and return a plan (`converted: false`) instead of a file
4. **Compliance** — reports posture against FIPS 140-3, CMMC 2.0 Level 2 and MITRE ATT&CK. It reports; it does not certify
5. **RAG System** — Document store, knowledge base, rule engine with MCP tool integration
6. **Cloud Storage** — Push/pull to AWS S3, Azure Blob, Google Cloud Storage
7. **Model Cards** — Google/HuggingFace standard model documentation
8. **Federation** — Sync vaults across peers with vector clocks
9. **Blockchain Audit** — Append-only audit trail with Merkle proofs
10. **Model Download** — Pull models from HuggingFace Hub, Ollama registry, or URLs with SHA-256 verification
11. **Model Signing** — HMAC-SHA256 signatures with detached `.sig` files for provenance
12. **Pickle Scanning** — Detect dangerous opcodes and patterns in PyTorch/pickle files
13. **Model Diffing** — Compare model versions at the tensor level (SafeTensors, GGUF, generic)
14. **Engine Interop** — Register models with Ollama (`ollama create`) and LM Studio
15. **Benchmark Metadata** — Store and query benchmark results per model version
16. **License Scanning** — Detect licenses from model cards, config.json, GGUF metadata, LICENSE files
17. **Model Tags & Search** — Tag models with labels and annotations, search by name/tags/annotations
18. **Vault Export/Import** — Portable tar.gz vault bundles with selective model export
19. **Garbage Collection** — Orphaned blob detection, temp file cleanup, space reclaim
20. **TUI Dashboard** — Terminal UI browser for vault contents
21. **Webhooks** — HTTP notification system with EventSubscriber integration
22. **Access Control** — Role-based ACL (Reader/Writer/Admin) per principal
23. **KMS Integration** — Fetch secrets from env, AWS Secrets Manager, Azure Key Vault, HashiCorp Vault
24. **Model Validation** — Integrity probes with SHA-256 checksums per model version
25. **Retention Policies** — Configurable max versions/age/minimum with dry-run enforcement
26. **Cross-Model Lineage DAG** — Directed acyclic graph tracking model derivation chains
27. **Plugin System** — Discover, install, uninstall plugins with JSON manifests
28. **Config Profiles** — Named configuration profiles with activate/deactivate switching
29. **Backup Scheduling** — scheduled vault backups with retention windows
30. **Evaluation** — store and query model evaluation runs and metrics
31. **Multi-Vault** — multiple named vaults, each with its own passphrase
32. **Quantization** — quantize models to q4/q5/q8 variants
33. **Telemetry** — opt-in, disclosed, honours `DO_NOT_TRACK`
34. **Agent Discovery** — `iv introspect` emits the whole CLI schema as JSON/JSON-LD
35. **Utilities** — the `iv://` URI scheme, checksums, and shared helpers

## Discovery Files

| File                                                             | Purpose                                                       |
| ---------------------------------------------------------------- | ------------------------------------------------------------- |
| [`.well-known/ai-plugin.json`](.well-known/ai-plugin.json)       | OpenAI-compatible plugin manifest                             |
| [`.well-known/ontology.jsonld`](.well-known/ontology.jsonld)     | JSON-LD ontology — all concepts, entities, relationships      |
| [`.well-known/mcp-manifest.json`](.well-known/mcp-manifest.json) | MCP tool definitions with JSON Schema inputs                  |
| [`.well-known/openapi.yaml`](.well-known/openapi.yaml)           | OpenAPI 3.1 specification for REST/GraphQL API                |
| [`.well-known/agents.json`](.well-known/agents.json)             | Agent discovery metadata — interfaces, capabilities, taxonomy |

## CLI Quick Reference

```bash
# Vault lifecycle
iv init [--name NAME]                    # Create encrypted vault
iv store <NAME> <PATH> [-f FORMAT]       # Store model (auto-detects format)
iv get <NAME> <OUTPUT> [-v VERSION]      # Retrieve & decrypt model
iv list                                  # List all models
iv versions <NAME>                       # List versions
iv lineage <NAME> <VERSION>              # Show ancestry tree
iv delete <NAME> <VERSION>               # Delete version
iv stats                                 # Storage statistics
iv compliance [--verbose]                # FIPS/CMMC/MITRE check

# Format conversion
iv convert <MODEL> --to-format <FMT> [--quantization q4_k_m] [--validate]
iv convert <NAME> --from-dir <HF_DIR> -t gguf   # HuggingFace → GGUF, no vault
iv list-conversions                      # Show conversion paths

# Cloud storage
iv cloud push <MODEL> --provider s3 --bucket <BUCKET>
iv cloud pull <MODEL> --provider s3 --bucket <BUCKET> --remote-path <PATH>
iv cloud list --provider s3 --bucket <BUCKET>

# RAG / Database
iv database init --path <P> --db-type sqlite
iv database store --path <P> --input <FILE>
iv database search --path <P> <QUERY>

# Utilities
iv archive <MODELS>... <OUTPUT> [-f tar|zip]
iv extract <ARCHIVE> [-o DIR]
iv analyze <NAME>
iv deduplicate
iv export <NAME> <OUTPUT>

# API server (requires --features api)
iv serve [--port 8080] [--jwt-secret SECRET]

# Agent discovery (machine-readable CLI schema)
iv introspect [--format json|yaml|jsonld] [--compact]

# Model download
iv pull <SOURCE> [-o DIR] [--sha256 HASH] [--token TOKEN] [--store] [--name NAME]

# Model signing & verification
# KEY is a file path or a KMS URI (e.g. azure-kv://vault/hmac-key)
iv sign <NAME> [--version V] [--key KEY] [--identity ID] [--file PATH]
iv verify <NAME> --signature <SIG> [--key KEY] [--file PATH]

# Safety scanning
iv scan [<NAME>] [--file PATH] [--version V] [--format text|json]

# Model diffing
iv diff <LEFT> <RIGHT> [--format text|json]   # LEFT/RIGHT: file path or name@version

# Engine registration
iv register <NAME> --engine <ollama|lm-studio> [--version V] [--alias NAME] [--system-prompt TEXT]

# Benchmark metadata
iv benchmark add <NAME> --version V --benchmark <BENCH> --score <N> --unit <UNIT>
iv benchmark show <NAME> [--version V] [--format text|json]

# License scanning
iv license-scan <PATH> [--format text|json]

# Model tags & search
iv tag add <MODEL> <TAGS>...                # Add tags to a model
iv tag remove <MODEL> <TAGS>...             # Remove tags from a model
iv tag list <MODEL>                          # List tags on a model
iv tag annotate <MODEL> --key <K> --value <V>  # Add key-value annotation
iv search <QUERY> [--tag TAG] [--format text|json]  # Search models

# Vault export/import
iv vault-export <OUTPUT>                     # Export vault as tar.gz bundle
iv vault-import <ARCHIVE> [TARGET]           # Import vault bundle

# Garbage collection
iv gc [--dry-run]                            # Clean orphaned blobs & temp files

# TUI dashboard
iv browse                                    # Browse vault in terminal UI

# Webhooks
iv webhook add --url <URL> [--secret SECRET]  # Add webhook target
iv webhook remove <ID>                       # Remove webhook target
iv webhook list                              # List webhook targets
iv webhook test <ID>                         # Test webhook delivery

# Access control
iv acl grant <PRINCIPAL> --role <ROLE>        # Grant role (reader/writer/admin)
iv acl revoke <PRINCIPAL>                    # Revoke access
iv acl list                                  # List ACL entries
iv acl check <PRINCIPAL> --role <ROLE>        # Check permission

# Model validation
iv validate <NAME> [--version V]             # Validate model integrity

# Retention policies
iv policy set <MODEL> [--max-versions N] [--max-age-days N] [--keep-minimum N]
iv policy remove <MODEL>                     # Remove retention policy
iv policy list                               # List all policies
iv policy apply <MODEL> [--dry-run]          # Apply policy to model
iv policy apply-all [--dry-run]              # Apply all policies

# Cross-model lineage DAG
iv lineage-graph add --child <C> --parents <P>... --kind <KIND>
iv lineage-graph show <MODEL>                # Display lineage for a model
iv lineage-graph ancestors <MODEL>           # Show ancestors
iv lineage-graph descendants <MODEL>         # Show descendants

# Plugin system
iv plugin discover                           # Scan for plugins
iv plugin install <PATH>                     # Install plugin from manifest
iv plugin uninstall <ID>                     # Uninstall plugin
iv plugin list                               # List installed plugins
iv plugin info <ID>                          # Show plugin details

# Config profiles
iv profile create <NAME> [--description TEXT] [--override KEY=VALUE]...
iv profile remove <NAME>                     # Remove profile
iv profile list                              # List all profiles
iv profile activate <NAME>                   # Activate profile
iv profile deactivate                        # Deactivate current profile
iv profile show <NAME>                       # Show a profile's details

# Quantization pipeline
iv quantize set <MODEL> --method <METHOD> [--version V] [--bits N]
iv quantize remove <MODEL> [--version V]     # Remove quantization profile
iv quantize list                             # List quantization profiles
iv quantize estimate <MODEL> --method <METHOD>  # Estimate output size

# Evaluation harness
iv eval record <MODEL> --suite <SUITE> --metric <METRIC> --score <N> [--version V]
iv eval list <MODEL> [--version V] [--suite SUITE] [--format text|json]
iv eval compare <MODEL> --versions <V1,V2,...> [--format text|json]
iv eval suites                               # List known evaluation suites

# Backup scheduling
iv backup schedule <VAULT> --interval <daily|weekly|monthly|custom> [--hour H]
iv backup list                               # List backup schedules
iv backup run [VAULT]                        # Run backup now
iv backup history [VAULT] [--format text|json]  # Show backup history

# Multi-vault management
iv vaults register <NAME> <PATH>             # Register a vault
iv vaults unregister <NAME>                  # Unregister a vault
iv vaults list                               # List all registered vaults
iv vaults activate <NAME>                    # Switch active vault
iv vaults active                             # Show active vault
```

## Supported Model Formats (23+)

| Category       | Formats                                                                                                                |
| -------------- | ---------------------------------------------------------------------------------------------------------------------- |
| **LLM**        | SafeTensors, GGUF, PyTorch (.pt/.pth/.bin), TensorRT (.plan), ONNX, MLX (.npz), CoreML (.mlmodel), TorchScript, TFLite |
| **General DL** | TensorFlow (.pb), Keras (.h5/.keras), OpenVINO (.xml+.bin), TVM (.so), NCNN (.param+.bin), MNN (.mnn), RKNN (.rknn)    |
| **Legacy**     | Caffe (.caffemodel), MXNet (.params), Darknet (.weights)                                                               |
| **Data**       | HDF5 (.h5/.hdf5), Pickle (.pkl), NumPy (.npy/.npz)                                                                     |

## Conversion Paths

`iv list-conversions` is the authoritative list. Ten converters are registered,
in two classes:

**Native (pure Rust — produces a real file):**

```
SafeTensors ↔ PyTorch
SafeTensors ↔ raw
GGUF        → metadata JSON   (header parser)
ONNX        → metadata JSON   (metadata extractor)
HuggingFace → GGUF            (`iv convert --from-dir`; llama only, F16/BF16/F32)
```

`HuggingFace → GGUF` is not a registered converter and is not reachable from
the `Converter` trait: a GGUF needs the whole checkpoint directory —
`config.json` and `tokenizer.model` as well as the weights — where the trait
takes a single `&[u8]`. It is its own CLI route, and it never opens the vault.

**Plan-only (needs an external Python toolchain — produces no file):**

```
PyTorch     → ONNX            (torch, onnx)
ONNX        → TensorRT        (tensorrt / trtexec)
ONNX        → Core ML         (coremltools)
SafeTensors → GGUF            (K-quants and non-llama architectures only, via
                               llama.cpp; for llama at F16/BF16/F32 use
                               `iv convert --from-dir`, which needs no Python)
```

Multi-step paths (e.g. PyTorch → ONNX → TensorRT) are found by BFS but stop at
the first plan-only step. For those, `POST /api/v1/convert` returns
`converted: false` with a `plan`, and `iv convert` writes `<output>.plan.json`
and no target-format file.

## MCP Tools (Model Context Protocol)

| Tool               | Description                                    |
| ------------------ | ---------------------------------------------- |
| `search_documents` | Vector similarity search in RAG knowledge base |
| `add_document`     | Add document with metadata and embeddings      |
| `chunk_text`       | Split text into overlapping chunks for RAG     |
| `execute_rule`     | Execute business rule from rule engine         |

Custom tools can be registered via `MCPServer::register_tool(tool, executor_fn)`.

## Cargo Features

| Feature        | Description                         |
| -------------- | ----------------------------------- |
| `default`      | SafeTensors, ndarray, SQLite        |
| `full`         | All non-system features             |
| `sqlite`       | SQLite RAG backend                  |
| `kv-store`     | Sled KV backend                     |
| `vector-db`    | Qdrant vector database              |
| `s3`           | AWS S3 cloud storage                |
| `azure`        | Azure Blob storage                  |
| `cloud`        | All cloud backends                  |
| `api`          | REST API (Axum + JWT)               |
| `graphql`      | GraphQL API                         |
| `python`       | Python bindings (PyO3)              |

## Environment Variables

| Variable                                                     | Purpose                              |
| ------------------------------------------------------------ | ------------------------------------ |
| `IRONVAULT_PASSPHRASE`                                    | Vault passphrase for CI/automation — a literal value or a KMS URI (`env://`, `file://`, `aws-sm://`, `azure-kv://`, `vault://`). See [docs/KMS.md](docs/KMS.md) |
| `IRONVAULT_VAULT`                                         | Default vault name                   |
| `IRONVAULT_CONFIG`                                        | Config directory override (`config.yaml`, profiles, plugins) |
| `IRONVAULT_HOME`                                          | Relocates all config/data/cache directories under one root — use for test isolation, containers, and per-project vaults |
| `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` / `AWS_REGION` | AWS S3 credentials                   |
| `AZURE_STORAGE_ACCOUNT` / `AZURE_STORAGE_SAS_TOKEN`          | Azure: account + SAS, or Entra ID (`AZURE_TENANT_ID` / `AZURE_CLIENT_ID` / `AZURE_CLIENT_SECRET`). Shared keys are not supported |
| `GOOGLE_APPLICATION_CREDENTIALS` / `GCP_PROJECT`             | GCS credentials                      |
| `IRONVAULT_TELEMETRY_ENABLED`                                      | Set to `false` to disable telemetry  |
| `IRONVAULT_TELEMETRY_DISABLED`                                     | Set to `1` to disable telemetry      |
| `DO_NOT_TRACK`                                               | Set to `1` to disable telemetry      |
| `IRONVAULT_SQLITE_VERSIONS`                                        | Use SQLite version backend           |

## Project Layout

```bash
src/
├── lib.rs              # Library root (pub modules)
├── main.rs             # CLI entry point
├── cli/                # CLI subcommand handlers
├── vault.rs            # Core vault logic + VaultBuilder
├── traits.rs           # Core traits, event system, URI parser, metrics
├── crypto/             # AES-256-GCM, Argon2id, streaming encryption
├── storage.rs          # Storage backends
├── version.rs          # Version control (JSON backend)
├── version_sqlite.rs   # Version control (SQLite backend)
├── formats.rs          # 23+ format detection
├── conversion.rs       # Format conversion pipeline (10 converters)
├── audit.rs            # Security audit logging
├── compliance.rs       # FIPS/CMMC/MITRE checks
├── model_card.rs       # Model Cards
├── rag/                # RAG system (7 submodules)
├── utils.rs            # Utilities
├── blockchain.rs       # Blockchain audit trail
├── federation.rs       # Federated vault sync
├── api.rs              # REST/GraphQL API
├── telemetry.rs        # Anonymous telemetry (opt-in)
├── config.rs           # XDG-compliant configuration
├── error.rs            # Error types
├── download.rs         # Model download (HuggingFace, Ollama, URLs)
├── signing.rs          # HMAC-SHA256 model signing & verification
├── scanning.rs         # Pickle safety scanning
├── diff.rs             # Model diffing (tensor-level comparison)
├── interop.rs          # Ollama & LM Studio registration
├── benchmark.rs        # Benchmark metadata storage
├── license_scan.rs     # License detection & SPDX normalization
├── tags.rs             # Model tagging and search
├── vault_bundle.rs     # Vault export/import bundles
├── gc.rs               # Garbage collection
├── tui.rs              # Terminal UI dashboard
├── webhooks.rs         # Webhook notification system
├── access_control.rs   # Role-based access control
├── kms.rs              # External secrets manager integration
├── validation.rs       # Model integrity validation
├── policies.rs         # Retention policy enforcement
├── lineage_graph.rs    # Cross-model lineage DAG
├── plugins.rs          # Plugin system
├── profiles.rs         # Configuration profiles
├── quantization.rs     # Quantization pipeline & profile store
├── evaluation.rs       # Model evaluation harness
├── scheduler.rs        # Vault backup scheduling
├── multi_vault.rs      # Multi-vault registry & switching
└── python.rs           # Python bindings (PyO3)
```

## Security Model

| Layer          | Technology                                         |
| -------------- | -------------------------------------------------- |
| Encryption     | AES-256-GCM (12-byte nonce, 16-byte auth tag)      |
| Key Derivation | Argon2id (64MB memory, 3 iterations, 32-byte salt) |
| Integrity      | SHA-256 checksums on every operation               |
| Memory         | Zeroize (secure memory zeroing)                    |
| Audit          | Append-only blockchain with Merkle proofs          |
| Permissions    | 700 dirs / 600 files (Unix), ACLs (Windows)        |

## Compliance

| Standard         | Status                                                                          |
| ---------------- | ------------------------------------------------------------------------------- |
| FIPS 140-3       | **Not validated.** Uses FIPS-approved AES-256-GCM and SHA-256, but the RustCrypto implementations hold no CMVP certificate, and Argon2id is not a FIPS-approved KDF |
| CMMC 2.0 Level 2 | **Not certified.** Supporting features for 17 controls (AC, AU, IA, SC); certification is granted to an organisation by a C3PAO |
| MITRE ATT&CK     | Design-level mitigations for T1552, T1486, T1078, T1005 (not a pentest)          |

Certification claims are deliberately absent: no software product can grant
itself FIPS 140-3 validation or CMMC certification. `iv compliance` reports
which checks it actually verified and which are design assertions.

## Agent Interaction Patterns

### Bootstrap (agent-first discovery)
```bash
# 1. Get the full CLI schema as JSON (pipe to jq, parse, etc.)
iv introspect --format json

# 2. Compact mode omits descriptions and examples for smaller payloads
iv introspect --format json --compact

# 3. JSON-LD output links to the ontology for semantic interop
iv introspect --format jsonld
```

### Store a model
```bash
iv init
iv store my-llm ./model.safetensors -d "Fine-tuned LLaMA" --framework pytorch --task text-generation
```

### Convert for edge deployment
```bash
# HuggingFace checkpoint → GGUF F16, pure Rust, streaming
iv convert tinyllama --from-dir ./TinyLlama-1.1B --to-format gguf

# K-quants stay llama-quantize's job; this returns a plan, not a file
iv convert my-llm --to-format gguf --quantization q4_k_m --validate
```

### Check compliance
```bash
iv compliance --verbose
```

### Search RAG knowledge base
```bash
iv database init --path ./kb --db-type sqlite
iv database store --path ./kb --input paper.pdf
iv database search --path ./kb "transformer attention mechanism"
```

### Push to cloud
```bash
iv cloud push my-llm --provider s3 --bucket my-models
```
