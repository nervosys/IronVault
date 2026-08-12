<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/nervosys/IronVault/master/media/banner_dark.png">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/nervosys/IronVault/master/media/banner_light.png">
    <img src="https://raw.githubusercontent.com/nervosys/IronVault/master/media/banner_light.png" alt="IronVault" width="360">
  </picture>
</p>

> Universal cross-platform encrypted vault for AI/ML model storage, versioning, conversion, and lifecycle management — **agent-first by design**, military-grade security, 23+ formats, 29 production features.

[![Rust](https://img.shields.io/badge/rust-1.89%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue.svg)](LICENSE)
[![Security](https://img.shields.io/badge/crypto-AES--256--GCM%20%2B%20Argon2id-green.svg)](SECURITY.md)
[![CMMC](https://img.shields.io/badge/CMMC%202.0%20L2-controls%20supported-blue.svg)](docs/SECURITY_HARDENING.md)
[![Tests](https://img.shields.io/badge/tests-2%2C227%20passing-brightgreen.svg)](reports/)
[![Coverage](https://img.shields.io/badge/coverage-85.4%25-brightgreen.svg)](docs/PERFORMANCE.md)
[![Version](https://img.shields.io/badge/version-5.1.2-blue.svg)](CHANGELOG.md)
[![crates.io](https://img.shields.io/crates/v/ironvault.svg)](https://crates.io/crates/ironvault)
[![PyPI](https://img.shields.io/pypi/v/ironvault.svg)](https://pypi.org/project/ironvault/)
[![Clippy](https://img.shields.io/badge/clippy-clean-brightgreen.svg)](validate.ps1)
[![Agent-ready](https://img.shields.io/badge/agent--ready-AGENTS.md-blueviolet.svg)](AGENTS.md)

A production-ready secure vault, built on FIPS-approved cryptographic algorithms, for storing and managing AI models. Every capability is exposed through **three parallel surfaces — CLI, REST/GraphQL, and MCP** — with a single source of truth (`iv introspect`) and self-describing manifests in [`.well-known/`](.well-known/). Built for autonomous agents, scriptable for CI, friendly for humans.

---

## Coming from AI Model Vault 4.x?

This project was renamed to **IronVault** in 5.0.0. Every identity moved at once:

| | 4.x | 5.0 |
| --- | --- | --- |
| crates.io | `ai-model-vault` | `ironvault` |
| PyPI | `aimodelvault` | `ironvault` |
| Rust import | `use ai_model_vault::…` | `use ironvault::…` |
| Python import | `import aimodelvault` | `import ironvault` |
| Binary | `aim` | `iv` |
| Environment | `aimodelvault_*`, `AIM_*` | `IRONVAULT_*` |

**Your vault does not move and your data does not need converting.** The Rust
XDG layout was already name-neutral (`~/.config/ai/models/`), and the encrypted
formats are read under both their old and new identifiers — objects sealed by
4.x still decrypt, and stored `aimv://` URIs still parse. The old environment
variable names still work too, warning once each; they stop being read in 6.0.

The old packages remain installable at 4.6.x and receive no further releases.
Full details and the systemd migration steps: **[docs/MIGRATION.md](docs/MIGRATION.md)**.

---

## For AI Agents — Read This First

If you are an LLM agent, IDE assistant, or automation pipeline, **start here** instead of scanning the rest of this README.

### One-line bootstrap

```bash
iv introspect --format json          # entire CLI schema, machine-readable
```

### Discovery surface (all in [.well-known/](.well-known/))

| File                                                 | Purpose                                                           |
| ---------------------------------------------------- | ----------------------------------------------------------------- |
| [`agents.json`](.well-known/agents.json)             | Capability catalog (29 features), taxonomy, interface inventory   |
| [`mcp-manifest.json`](.well-known/mcp-manifest.json) | **86 MCP tools** with full JSON Schema inputs, resources, prompts |
| [`openapi.yaml`](.well-known/openapi.yaml)           | OpenAPI 3.1 — **56 REST endpoints**, checked against the router in CI          |
| [`ontology.jsonld`](.well-known/ontology.jsonld)     | JSON-LD ontology — every concept, class, and relationship         |
| [`ai-plugin.json`](.well-known/ai-plugin.json)       | OpenAI-compatible plugin manifest cross-linking the above         |
| [`AGENTS.md`](AGENTS.md)                             | Canonical project context — features, CLI cheat sheet, layout     |

> **The spec and the router are checked against each other.** Through 5.0 they
> had drifted badly: the spec documented 53 paths against 44 registered routes,
> and 14 of those paths had no handler at all, so any client generated from the
> published spec called them and got a 404. 5.1.0 implemented the missing 14,
> documented the missing 5, and added
> [`tests/openapi_drift_test.rs`](tests/openapi_drift_test.rs) — adding a route
> without documenting it, or documenting a path without implementing it, now
> fails the build. The spec is a contract again, not a wish list.

### Canonical agent integration pattern

```bash
# 1. Discover — get every command, flag, type
iv introspect --format jsonld > schema.jsonld

# 2. Speak any surface
iv <subcommand> --format json        # local CLI, JSON out
curl  http://host:8080/api/v1/...     # REST (see openapi.yaml)
# or call MCP tools from mcp-manifest.json over your MCP client
```

### Stability contract for agents

- **JSON output:** every read-style subcommand accepts `--format json`. Output schema versioned alongside the crate.
- **Exit codes:** `0` success · `1` general error · `2` authentication failed · `3` not found · `4` permission denied · `5` integrity / verification failure · `6` invalid input (including usage errors) · `7` configuration error · `8` compliance violation. Non-zero ⇒ failure, always. Enforced by [`VaultError::exit_code`](src/error.rs) and pinned by tests.
- **Idempotent reads:** `list`, `get`, `search`, `versions`, `lineage`, `stats`, `compliance`, `introspect`, `*/show`, `*/list` are side-effect free.
- **Destructive ops gated:** `delete`, `policy apply`, `gc`, `vault-import` accept `--dry-run` (where applicable) or require an explicit name argument.
- **Self-describing errors:** error JSON includes `code`, `message`, and `hint`; never just a string.
- **URIs:** Vault resources are addressable via the [`iv://`](docs/UTILITIES.md) scheme — agents can pass `iv://vault/model@version` between tools.
- **No surprise network:** the CLI never phones home except `iv pull` (explicit), `iv cloud` (explicit), and opt-in telemetry — off by default and honors `DO_NOT_TRACK=1`. When enabled it posts to `telemetry.endpoint`, which defaults to `https://telemetry.nervosys.ai/v1/events` — **a host that currently has no DNS record**, so enabling telemetry without setting your own endpoint sends nothing anywhere. Two events, no model names or paths: see [docs/TELEMETRY.md](docs/TELEMETRY.md).

### Three-surface coverage matrix

Every one of the 29 features in [AGENTS.md](AGENTS.md) is reachable from **all
three** of: CLI subcommand, REST endpoint, and MCP tool. This became true in
5.1.0 — `sign`, `verify`, `scan`, `diff`, `pull`, `license-scan`, `register`,
`benchmarks`, the card operations, and vault `export` / `import` had CLI and MCP
surfaces but no REST handler, despite the spec claiming otherwise.

See the parity table in [agents.json](.well-known/agents.json) for the precise
mapping.

---

## Table of Contents

| For Agents                                                       | For Humans                                          | Operations                                     |
| ---------------------------------------------------------------- | --------------------------------------------------- | ---------------------------------------------- |
| [AGENTS.md](AGENTS.md) — canonical context                       | [Quick Start](#quick-start)                         | [Security & Compliance](#security--compliance) |
| [`.well-known/`](.well-known/) — discovery manifests             | [Installation](#installation)                       | [Build & Validate](#build--validate)           |
| [`iv introspect`](#for-ai-agents--read-this-first) — CLI schema | [CLI Reference](docs/CLI.md)                        | [Architecture](#architecture)                  |
| [MCP tools](docs/MCP_TOOLS.md) — 86 tools                        | [Rust API Quickstart](#rust-library-api-quickstart) | [Performance](docs/PERFORMANCE.md)             |
| [OpenAPI 3.1](.well-known/openapi.yaml) — 56 endpoints           | [REST API Reference](docs/REST_API.md)              | [Deployment](#deployment)                      |
|                                                                  | [Demos](#interactive-demos)                         |                                                |
|                                                                  | [Telemetry](docs/TELEMETRY.md) — opt-in, disclosed  | [Contributing](CONTRIBUTING.md)                |

---

## Why IronVault?

- **Agent-first** — three coequal surfaces (CLI / REST+GraphQL / MCP), one schema, self-describing via `introspect` and `.well-known/`
- **Secure by default** — AES-256-GCM with Argon2id KDF; aligned to CMMC 2.0 L2 and MITRE ATT&CK control families. Not a FIPS-validated module — see [Security & Compliance](#security--compliance)
- **Format-agnostic** — auto-detect 23+ formats; convert natively between SafeTensors, PyTorch, and raw. Conversions that need a Python toolchain (→ ONNX, → TensorRT, → Core ML, → GGUF) return a runnable plan rather than a silently wrong file
- **Provenance built-in** — SHA-256 checksums, HMAC signatures, an automatic append-only audit log, license & pickle scanning (plus a Merkle-chained block store available as a library primitive)
- **Operational** — version control, retention policies, garbage collection, multi-vault, profiles, plugins, scheduled backups
- **Integrated** — REST + GraphQL APIs, 86 MCP tools, Python bindings, Ollama / LM Studio interop, HuggingFace / Ollama / URL pull
- **Quality** — 2,227 Rust + 84 Python tests, 0 clippy warnings, fuzz targets, property-based tests, criterion benchmarks

---

## Quick Start

### Install

```bash
# From crates.io
cargo install ironvault --features full,api
```

```bash
# Prebuilt binary (Linux / macOS / Windows, no toolchain needed)
# https://github.com/nervosys/IronVault/releases/latest
curl -sSLO https://github.com/nervosys/IronVault/releases/latest/download/iv-linux-amd64
curl -sSLO https://github.com/nervosys/IronVault/releases/latest/download/iv-linux-amd64.sha256
sha256sum -c iv-linux-amd64.sha256 && chmod +x iv-linux-amd64 && sudo mv iv-linux-amd64 /usr/local/bin/iv
```

> Prebuilt binaries include the REST API (`iv serve`) from **5.1.1** onward.
> Downloads up to and including 5.1.0 were default-feature builds, where
> `iv serve` answered *"unrecognized subcommand"*.

```bash
# Python bindings
pip install ironvault
```

```bash
# From source
git clone https://github.com/nervosys/IronVault.git
cd IronVault
cargo build --release --features full,api
# Binary at target/release/iv (~17 MB, LTO + stripped)
```

`full` covers the storage backends but not the REST API — add `api` if you
want `iv serve`. See [Cargo feature flags](#cargo-feature-flags).

### 30-second walkthrough

```bash
# 1. Initialize an encrypted vault
iv init

# 2. Store a model (auto-detects format)
iv store llama-7b ./model.safetensors \
  --description "Fine-tuned Llama 7B" --framework pytorch --task text-generation

# 3. Pull from HuggingFace, Ollama, or a URL
iv pull hf:mistralai/Mistral-7B-v0.1 --store --name mistral-7b
iv pull ollama:llama3 --store --name llama3

# 4. Convert SafeTensors → GGUF Q4_K_M for edge deployment
iv convert llama-7b --to-format gguf --quantization q4_k_m --validate

# 5. Sign, scan, and tag
iv sign llama-7b --identity "trainer@company.com"
iv scan llama-7b
iv tag add llama-7b production fine-tuned

# 6. Check security & compliance
iv compliance --verbose

# 7. Browse the vault interactively
iv browse
```

---

## Feature Matrix

All features below are fully implemented, tested, and exposed via both CLI and library API unless noted.

### Storage & Encryption

| Feature                 | CLI           | Notes                                                      |
| ----------------------- | ------------- | ---------------------------------------------------------- |
| AES-256-GCM encryption  | (default)     | Argon2id KDF (64 MB / 3 iterations / 32-byte salt)         |
| Streaming encryption    | (auto)        | Constant 8 MiB memory for multi-GB models                  |
| KMS integration         | `$IRONVAULT_PASSPHRASE` | `env://`, `file://`, `azure-kv://`, `vault://`, `aws-sm://` (`--features s3`) |
| 23+ model formats       | (auto-detect) | See [Supported Formats](#supported-model-formats)          |
| Cloud storage           | `iv cloud`   | AWS S3, Azure Blob. Uploads sealed client-side (AES-256-GCM) |

### Version Control & Lineage

| Feature                 | CLI                     | Notes                                           |
| ----------------------- | ----------------------- | ----------------------------------------------- |
| Sequential versioning   | `iv versions`          | Unique checkpoint IDs per version               |
| Parent lineage          | `iv lineage`           | Parent-child genealogy with branching           |
| Cross-model lineage DAG | `iv lineage-graph`     | Ancestors / descendants of derived models       |
| Instant rollback        | `iv get -v N`          | Time-travel to any historical checkpoint        |
| Retention policies      | `iv policy`            | Max versions / age / minimum keep, with dry-run |
| SQLite version backend  | `IRONVAULT_SQLITE_VERSIONS=1` | ACID-compliant, auto-migrates from JSON         |

### Conversion & Quantization

| Feature                  | CLI                | Notes                                                 |
| ------------------------ | ------------------ | ----------------------------------------------------- |
| Format conversion (10×)  | `iv convert`      | Native: PyTorch ↔ SafeTensors, ↔ raw. Plan-only (needs Python): → ONNX/TensorRT/Core ML/GGUF |
| GGUF quantization        | `--quantization …` | Q4_0, Q4_K_M, Q5_K_M, Q8_0, F16, F32                  |
| Quantization profiles    | `iv quantize`     | Per-model method selection, size estimation           |
| ONNX → TensorRT/OpenVINO | `iv convert`      | Edge & GPU deployment paths                           |

### Safety, Signing & Validation

| Feature              | CLI                 | Notes                                                |
| -------------------- | ------------------- | ---------------------------------------------------- |
| HMAC-SHA256 signing  | `iv sign / verify` | Detached `.sig` files for provenance                 |
| Pickle scanner       | `iv scan`          | Detects `REDUCE`, `GLOBAL`, `os.system`, `eval`, …   |
| License scanner      | `iv license-scan`  | Model cards, `config.json`, GGUF meta, LICENSE; SPDX |
| Integrity validation | `iv validate`      | SHA-256 integrity probe per version                  |
| Tensor-level diff    | `iv diff`          | SafeTensors / GGUF / generic binary fallback         |

### Provenance, Audit & Compliance

| Feature            | CLI              | Notes                                             |
| ------------------ | ---------------- | ------------------------------------------------- |
| Audit log          | (automatic)      | Every operation; structured, append-only          |
| Blockchain audit   | `iv chain`      | Merkle-proofed hash chain; opt-in, mirrors the audit log |
| Model cards        | (via API)        | Google / HuggingFace standard, JSON/YAML/Markdown |
| Compliance check   | `iv compliance` | FIPS 140-3, CMMC 2.0 L2, MITRE ATT&CK             |
| Benchmark metadata | `iv benchmark`  | MMLU, HellaSwag, etc., per model version          |
| Evaluation harness | `iv eval`       | Record, compare, query across suites and metrics  |

### Discovery, Operations & Lifecycle

| Feature               | CLI                      | Notes                                    |
| --------------------- | ------------------------ | ---------------------------------------- |
| Tags & search         | `iv tag` / `iv search` | Labels + key-value annotations           |
| Garbage collection    | `iv gc`                 | Orphan blobs, temp files; `--dry-run`    |
| Vault export / import | `iv vault-export`       | Portable `.tar.gz` bundles               |
| Multi-vault registry  | `iv vaults`             | Register, switch active vault            |
| Backup scheduling     | `iv backup`             | Daily / weekly / monthly / custom        |
| Config profiles       | `iv profile`            | Named overrides, activate / deactivate   |
| Plugin system         | `iv plugin`             | Discover, install JSON-manifest plugins  |
| TUI dashboard         | `iv browse`             | Terminal UI vault browser                |
| Webhooks              | `iv webhook`            | HTTP notifications via `EventSubscriber` |
| Access control (RBAC) | `iv acl`                | Reader / Writer / Admin per principal    |

### Integration & APIs

| Feature              | Surface               | Notes                                              |
| -------------------- | --------------------- | -------------------------------------------------- |
| REST API             | `iv serve`           | Axum + JWT + 56 endpoints, [reference](docs/REST_API.md) |
| GraphQL API          | `iv serve --graphql` | `async-graphql` with playground                    |
| MCP tools            | library               | 4 built-in tools + custom registration             |
| Python bindings      | `pip install` (PyO3)  | `--features python`                                |
| Engine interop       | `iv register`        | Ollama (`ollama create`) + LM Studio               |
| Model download       | `iv pull`            | HuggingFace, Ollama, URLs (+ SHA-256 verification) |
| Federation           | `iv federation`      | Vector-clock peer sync; opt-in, sealed in transit  |
| RAG / Knowledge base | `iv database`        | SQLite / Sled / Qdrant backends                    |
| `iv://` URI scheme | library               | Agent-addressable vault resources                  |
| Agent introspection  | `iv introspect`      | JSON / YAML / JSON-LD CLI schema                   |

> Full machine-readable surface (29 features, all CLI subcommands, ontology, OpenAPI, MCP manifest) is in [`.well-known/`](.well-known/) and [`AGENTS.md`](AGENTS.md).

---

## Supported Model Formats

| Category    | Formats                                                                                                                |
| ----------- | ---------------------------------------------------------------------------------------------------------------------- |
| **LLM**     | SafeTensors, GGUF, PyTorch (.pt/.pth/.bin), TensorRT (.plan), ONNX, MLX (.npz), CoreML (.mlmodel), TorchScript, TFLite |
| **General** | TensorFlow (.pb), Keras (.h5/.keras), OpenVINO (.xml+.bin), TVM (.so), NCNN (.param+.bin), MNN (.mnn), RKNN (.rknn)    |
| **Legacy**  | Caffe (.caffemodel), MXNet (.params), Darknet (.weights)                                                               |
| **Data**    | HDF5 (.h5/.hdf5), Pickle (.pkl), NumPy (.npy/.npz)                                                                     |

**Conversion paths**

```
PyTorch     → SafeTensors, ONNX, TorchScript, CoreML, MLX
SafeTensors → GGUF (q4_0, q4_k_m, q5_k_m, q8_0, f16, f32)
ONNX        → TensorRT, OpenVINO, TFLite
TensorFlow  → TFLite
```

See [docs/PROVIDERS_FORMATS.md](docs/PROVIDERS_FORMATS.md) and [FORMATS.md](FORMATS.md) for full details.

---

## Installation

### From a registry

```bash
cargo install ironvault --features full,api   # Rust CLI + library
pip install ironvault                           # Python bindings
```

Prebuilt binaries for Linux (gnu and musl), macOS (x86-64 and arm64), and
Windows are attached to every [release](https://github.com/nervosys/IronVault/releases/latest),
each with a `.sha256` alongside it.

### From source

```bash
git clone https://github.com/nervosys/IronVault.git
cd IronVault

# Default build (SQLite)
cargo build --release

# Storage backends + REST API + GraphQL
cargo build --release --features full,graphql

# Or use the helpers
./build.sh release           # Linux/macOS
.\build.ps1 release          # Windows
```

The release binary lives at `target/release/iv` (~17 MB, LTO + stripped).

### Cargo feature flags

| Feature     | Description                                                |
| ----------- | ---------------------------------------------------------- |
| `default`   | SQLite. Also accepts `safetensors` / `ndarray`, no-ops since 4.6.0 |
| `full`      | `default` + Sled + Qdrant. **Not** the APIs, cloud, or otel |
| `sqlite`    | SQLite RAG backend                                         |
| `kv-store`  | Sled KV backend                                            |
| `vector-db` | Qdrant vector database                                     |
| `s3`        | AWS S3 cloud storage                                       |
| `azure`     | Azure Blob storage                                         |
| `cloud`     | All cloud backends                                         |
| `api`       | REST API (Axum + JWT) — required for `iv serve`           |
| `graphql`   | GraphQL API (implies `api`)                                |
| `python`    | Python bindings (PyO3)                                     |
| `otel`      | OTLP export for telemetry events                           |

`full` is narrower than the name suggests: it enables the storage backends
only. To get the server, ask for it explicitly:

```bash
cargo build --release --features full,api      # + iv serve
cargo build --release --features full,cloud    # + S3 / Azure
```

### Optional system dependencies

- **HashiCorp Vault / AWS / Azure** — only if you use the corresponding KMS / cloud features.

---

## Rust Library API Quickstart

```rust
use ironvault::{Vault, VaultConfig};
use ironvault::formats::{ModelFormat, ModelMetadata};

let mut vault = Vault::new(None)?;
vault.unlock(b"your-secure-passphrase".to_vec())?;

// Store
let data = std::fs::read("model.safetensors")?;
let metadata = ModelMetadata::new("llama-7b".into(), ModelFormat::Safetensors)
    .with_description("Fine-tuned Llama 7B".into())
    .with_framework("PyTorch".into())
    .with_task("text-generation".into())
    .with_parameters(7_000_000_000);
let version = vault.store_model("llama-7b", data, metadata, None)?;

// Retrieve specific version
let v2 = vault.get_model("llama-7b", Some(2))?;

// List history
for v in vault.list_versions("llama-7b") {
    println!("v{}: {} bytes", v.version, v.original_size);
}
```

### Trait-based dependency injection (advanced)

```rust
use ironvault::{VaultBuilder, AuditLogSubscriber, MetricsSubscriber};

let vault = VaultBuilder::new()
    .config(VaultConfig::default())
    .sqlite_versions(true)
    .subscriber(Box::new(AuditLogSubscriber::default()))
    .subscriber(Box::new(MetricsSubscriber::default()))
    .build()?;
```

`CryptoProvider`, `BlobStore`, `VersionRepo`, `AuditSink`, and `EventSubscriber` are all swappable traits. See [docs/ARCHITECTURE_V2.md](docs/ARCHITECTURE_V2.md).

### MCP / RAG tools

```rust
use ironvault::rag::*;

let mut server = MCPServer::new();
server.register_builtin_tools()?;

let ctx = ToolContext::new()
    .with_knowledge_base("research_kb".into())
    .with_data("user_id".into(), "researcher_1".into());

let result = server.execute_tool("search_documents", &ctx, /* args */ ..)?;
```

Built-in tools: `search_documents`, `add_document`, `chunk_text`, `execute_rule`. Custom tools via `MCPServer::register_tool(tool, executor_fn)`.

---

## Cloud Storage

```bash
# Push, list, pull
iv cloud push  llama-7b --provider s3 --bucket my-models
iv cloud list  --provider s3 --bucket my-models
iv cloud pull  llama-7b --provider s3 --bucket my-models --remote-path llama-7b/safetensors/v1.vault
```

| Provider             | Status                                                  |
| -------------------- | ------------------------------------------------------- |
| AWS S3               | ✅ `--features s3`                                       |
| Azure Blob           | ✅ `--features azure`                                    |
| Google Cloud Storage | ❌ Removed — no `gcs` feature exists                     |

Uploads are **sealed client-side** (4.3.0+): AES-256-GCM under an Argon2id key
derived from your vault passphrase, fresh salt per object, so the bucket holds
ciphertext and can be treated as untrusted. The salt travels with the object,
so a peer who knows the passphrase can `pull` into a *different* vault.

> Objects pushed by a version **before** 4.3.0 are plaintext. `pull` still
> accepts them so nothing is stranded, but warns — re-push to seal, then
> delete the old object. See [docs/CLOUD_STORAGE.md](docs/CLOUD_STORAGE.md#security-model).

Credentials come from standard environment variables. S3 uses the normal AWS
chain (`AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` / `AWS_REGION`, profiles,
or an instance role). Azure takes `AZURE_STORAGE_ACCOUNT` plus either
`AZURE_STORAGE_SAS_TOKEN` or Entra ID — `AZURE_STORAGE_KEY` is **not**
supported, as the Azure SDK for Rust v1 has no shared-key credential.

Full guide: [docs/CLOUD_STORAGE.md](docs/CLOUD_STORAGE.md) · CLI: [docs/CLOUD_CLI.md](docs/CLOUD_CLI.md).

---

## Deployment

Running `iv serve` as a service. Both paths keep configuration **service-scoped** — nothing is written to `/etc/environment` or a profile script, so no other process on the host inherits the API secret or a telemetry token.

> [!IMPORTANT]
> **`iv serve` refuses to bind a non-loopback address without TLS.** Set
> `tls_cert` and `tls_key` to PEM paths and it serves HTTPS directly; loopback
> binds still serve plain HTTP, because those packets never reach a network.
>
> This is enforced, not advised, because `POST /api/v1/auth/token` carries the
> vault **passphrase** — the value the encryption key is derived from. Unlike a
> leaked token it never expires, revocation cannot reach it, and it decrypts any
> copy of the vault taken at any time. To terminate TLS at a reverse proxy, run
> the proxy on the same host and leave the server on `127.0.0.1`; a proxy on a
> different host is a real network hop that needs its own TLS.

### systemd

```bash
sudo ./deploy/systemd/install.sh --dry-run    # see every change first
sudo ./deploy/systemd/install.sh
sudo systemctl enable --now ironvault-server
```

Creates the `ironvault` system user and `/var/lib/ironvault`, writes `/etc/ironvault/server.env` at `0600` root-owned, generates `IRONVAULT_JWT_SECRET` if absent, and installs a hardened unit using `EnvironmentFile=` rather than `Environment=` — the latter is readable by any local user via `systemctl show`.

To configure OTLP export at install time, pass the credential as a *file*, never a flag (arguments are world-readable through `/proc/<pid>/cmdline`):

```bash
printf 'Authorization=Bearer %s' "$TOKEN" > /tmp/hdr && chmod 600 /tmp/hdr
sudo ./deploy/systemd/install.sh \
    --otlp-endpoint https://collector.example.com/otlp \
    --otlp-headers-file /tmp/hdr \
    --enable-telemetry
shred -u /tmp/hdr
```

Details: [docs/TELEMETRY.md](docs/TELEMETRY.md#service-scoped-configuration) · [docs/SECURITY_HARDENING.md](docs/SECURITY_HARDENING.md).

> **Containers were removed in 4.5.0.** The `Dockerfile`, the image published
> to `ghcr.io`, and the Helm chart are gone. `iv` ships as a static binary,
> a crate, and a Python wheel; run it directly or under systemd. Images
> already published to `ghcr.io` remain pullable but receive no further
> updates.

---

## Security & Compliance

| Layer            | Implementation                                           |
| ---------------- | -------------------------------------------------------- |
| Symmetric crypto | AES-256-GCM (12-byte nonce, 16-byte auth tag)            |
| Key derivation   | Argon2id (64 MB memory, 3 iterations, 32-byte salt)      |
| Integrity        | SHA-256 checksums on every operation                     |
| Memory hygiene   | `zeroize` on key material                                |
| Audit trail      | Append-only, `0600`; Merkle proofs via library API only  |
| Permissions      | `0700` directories / `0600` files (Unix), ACLs (Windows) |
| Signing          | HMAC-SHA256 with detached `.sig`                         |
| Scanning         | Pickle opcode scanner + license/SPDX scanner             |
| Access control   | Per-principal RBAC (Reader / Writer / Admin)             |

### Standards

| Standard         | Status                                        |
| ---------------- | --------------------------------------------- |
| **FIPS 140-3**   | **Not validated.** Uses FIPS-approved AES-256-GCM (FIPS 197 / SP 800-38D) and SHA-256 (FIPS 180-4). The RustCrypto implementations hold no CMVP certificate, and Argon2id is not a FIPS-approved KDF — SP 800-132 approves PBKDF2. A genuine FIPS obligation needs a validated module (AWS-LC-FIPS, BoringCrypto, or an HSM). |
| **CMMC 2.0 L2**  | **Not certified.** Supporting features for 17 controls (AC, AU, IA, SC). CMMC certification is granted to an *organisation* by a C3PAO, never to a software product. |
| **MITRE ATT&CK** | Design-level mitigations for T1552, T1486, T1078, T1005. Not a penetration test. |
| **OWASP Top 10** | Reviewed; no known issues in first-party code |

`iv compliance` distinguishes what it actually verified at runtime from what
is asserted by design, and exits non-zero only on a real, verified failure.

### Dependency security

Current status of [`cargo audit`](https://github.com/rustsec/rustsec) on `master`:

- ✅ `rustls-webpki` 0.103.13 in the primary `reqwest` / `hyper-rustls` path (RUSTSEC-2026-0098/0099/0104 patched)
- ⚠️ A handful of advisories remain in **transitive** dependencies (`aws-smithy-http-client` 1.1.12 → old `rustls` 0.21; sled, hdf5, azure SDK unmaintained helpers). All are documented and tracked in [`deny.toml`](deny.toml) with justification; `cargo deny check` passes.

These will clear automatically once AWS SDK upgrades to a Smithy client that uses `hyper-rustls` ≥ 0.27. No first-party code is affected.

Reporting vulnerabilities: **security@nervosys.ai** — do **not** open public issues. See [SECURITY.md](SECURITY.md).

---

## Build & Validate

```bash
# Full validation pipeline (fmt + clippy + build + test + doc)
.\validate.ps1          # Windows
./validate.sh           # Linux/macOS

# Individually
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --features full,graphql
cargo doc --no-deps --all-features
```

Current `master` status:

- ✅ `cargo fmt` clean
- ✅ `cargo clippy` — 0 warnings
- ✅ `cargo build --features full,graphql` — clean
- ✅ `cargo test` — **2,026+ tests passing** across 18 suites
- ✅ `cargo doc` — no warnings
- ✅ `cargo deny check` — pass

### Quality engineering

- 51 cross-module integration tests
- 11 property-based test strategies (proptest)
- 8 fuzz targets (pickle scanner, diff engine, model card parser, …)
- Criterion benchmarks with CI regression tracking (`benches/`)

---

## Interactive Demos

```bash
# Quick 2-minute tour
.\docs\demo.ps1 -Quick           # Windows
./docs/demo.sh   --quick          # Linux/macOS

# Specific feature demos
.\docs\demo.ps1 -HuggingFace
.\docs\demo.ps1 -Security
```

### Cargo examples

```bash
cargo run --example basic_usage             # End-to-end vault flow
cargo run --example version_control_demo    # Versioning, lineage, rollback
cargo run --example providers_formats_demo  # 23+ formats walkthrough
cargo run --example signing_demo            # HMAC signing & verification
cargo run --example scanning_demo           # Pickle safety scanning
cargo run --example diff_demo               # Tensor-level diffing
cargo run --example download_demo           # HF / Ollama / URL pull
cargo run --example interop_demo            # Ollama + LM Studio registration
cargo run --example benchmark_demo          # Benchmark metadata
cargo run --example license_scan_demo       # License detection
cargo run --example model_card_demo         # Model cards (Google/HF)
cargo run --example mcp_tools_demo          # MCP tool usage
cargo run --example rag_demo                # RAG with knowledge base
cargo run --example security_demo           # Compliance + audit
cargo run --example utilities_demo          # Archive / analyze / dedupe
cargo run --example xdg_demo                # XDG paths
cargo run --example api_demo                # REST + GraphQL
cargo run --example huggingface_demo        # HF integration
```

Full demo guide: [docs/DEMO_GUIDE.md](docs/DEMO_GUIDE.md).

### Environment variables

| Variable                                                     | Purpose                            |
| ------------------------------------------------------------ | ---------------------------------- |
| `IRONVAULT_PASSPHRASE`                                    | Vault passphrase (CI / automation) — literal value or KMS URI, see [docs/KMS.md](docs/KMS.md) |
| `IRONVAULT_VAULT`                                         | Default vault name                 |
| `IRONVAULT_CONFIG`                                        | Config directory override          |
| `IRONVAULT_HOME`                                          | Relocates all config/data/cache directories under one root |
| `IRONVAULT_SQLITE_VERSIONS`                                        | Use SQLite version backend         |
| `IRONVAULT_TELEMETRY_DISABLED=1` / `DO_NOT_TRACK=1`                | Disable anonymous telemetry        |
| `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` / `AWS_REGION` | AWS S3 credentials                 |
| `AZURE_STORAGE_ACCOUNT` / `AZURE_STORAGE_SAS_TOKEN`          | Azure: account + SAS. Or Entra ID via `AZURE_TENANT_ID` / `AZURE_CLIENT_ID` / `AZURE_CLIENT_SECRET`. Shared keys (`AZURE_STORAGE_KEY`) are not supported |
| `OTEL_EXPORTER_OTLP_ENDPOINT` / `_PROTOCOL` / `_HEADERS`     | OTLP export (`--features otel`). Setting these does not enable telemetry |
| `OTEL_SERVICE_NAME`                                          | Reported as `service.name`         |

---

## Architecture

```text
src/
├── lib.rs / main.rs           # Library root + CLI entry
├── cli/                       # CLI dispatcher + per-command handlers
├── crypto/                    # AES-256-GCM, Argon2id, streaming
├── rag/                       # 7 RAG submodules (docs, KB, MCP, rules…)
├── vault.rs                   # Core vault logic + VaultBuilder
├── traits.rs                  # CryptoProvider, BlobStore, EventBus, URI parser
├── storage.rs                 # Local + S3/Azure backends
├── version.rs / version_sqlite.rs  # Version control (JSON + SQLite backends)
├── formats.rs                 # 23+ format detection
├── conversion.rs              # 10 format converters
├── model_card.rs              # Google / HuggingFace model cards
├── api.rs                     # REST (Axum) + GraphQL (async-graphql)
├── blockchain.rs              # Append-only audit chain with Merkle proofs
├── federation.rs              # Vector-clock peer sync
├── compliance.rs / audit.rs   # FIPS / CMMC / MITRE checks + audit log
├── download.rs                # HuggingFace / Ollama / URL pull (+ SHA-256)
├── signing.rs                 # HMAC-SHA256 signing
├── scanning.rs                # Pickle opcode scanner
├── diff.rs                    # Tensor-level diffing
├── interop.rs                 # Ollama + LM Studio
├── benchmark.rs / evaluation.rs  # Benchmark + eval metadata
├── license_scan.rs            # License detection + SPDX
├── tags.rs                    # Tags + key-value annotations
├── vault_bundle.rs            # Export / import bundles
├── gc.rs                      # Garbage collection
├── tui.rs                     # Terminal UI dashboard
├── webhooks.rs                # HTTP notification system
├── access_control.rs          # RBAC
├── kms.rs                     # AWS / Azure / HashiCorp / env
├── validation.rs              # Integrity probes
├── policies.rs                # Retention policies
├── lineage_graph.rs           # Cross-model DAG
├── plugins.rs                 # Plugin discovery + install
├── profiles.rs                # Config profiles
├── quantization.rs            # Quantization profile store
├── scheduler.rs               # Backup scheduling
├── multi_vault.rs             # Multi-vault registry
├── telemetry.rs               # Anonymous opt-in usage
├── config.rs                  # XDG-compliant config
└── python.rs                  # PyO3 bindings
```

Deep dives: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) · [docs/ARCHITECTURE_V2.md](docs/ARCHITECTURE_V2.md).

---

## Documentation

| Topic                                   | Document                                                                                                           |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| CLI reference                           | [docs/CLI.md](docs/CLI.md)                                                                                         |
| Cloud storage                           | [docs/CLOUD_STORAGE.md](docs/CLOUD_STORAGE.md) · [docs/CLOUD_CLI.md](docs/CLOUD_CLI.md)                            |
| RAG & MCP                               | [docs/RAG.md](docs/RAG.md) · [docs/MCP_TOOLS.md](docs/MCP_TOOLS.md) · [docs/MCP_QUICKREF.md](docs/MCP_QUICKREF.md) |
| Model cards                             | [docs/MODEL_CARDS.md](docs/MODEL_CARDS.md) · [docs/MODEL_CARDS_QUICKREF.md](docs/MODEL_CARDS_QUICKREF.md)          |
| Version control                         | [docs/VERSION_CONTROL.md](docs/VERSION_CONTROL.md)                                                                 |
| Model download                          | [docs/MODEL_DOWNLOAD.md](docs/MODEL_DOWNLOAD.md)                                                                   |
| Model signing                           | [docs/MODEL_SIGNING.md](docs/MODEL_SIGNING.md)                                                                     |
| Safety scanning                         | [docs/SAFETY_SCANNING.md](docs/SAFETY_SCANNING.md)                                                                 |
| Model diffing                           | [docs/MODEL_DIFFING.md](docs/MODEL_DIFFING.md)                                                                     |
| License scanning                        | [docs/LICENSE_SCANNING.md](docs/LICENSE_SCANNING.md)                                                               |
| Engine interop (Ollama, LM Studio)      | [docs/ENGINE_INTEROP.md](docs/ENGINE_INTEROP.md)                                                                   |
| Quantization                            | [docs/QUANTIZATION.md](docs/QUANTIZATION.md)                                                                       |
| Evaluation harness                      | [docs/EVALUATION.md](docs/EVALUATION.md)                                                                           |
| Backup scheduling                       | [docs/BACKUP_SCHEDULING.md](docs/BACKUP_SCHEDULING.md)                                                             |
| Multi-vault                             | [docs/MULTI_VAULT.md](docs/MULTI_VAULT.md)                                                                         |
| Python bindings                         | [docs/PYTHON_BINDINGS.md](docs/PYTHON_BINDINGS.md)                                                                 |
| Security hardening                      | [docs/SECURITY_HARDENING.md](docs/SECURITY_HARDENING.md) · [docs/SECURITY_AUDIT.md](docs/SECURITY_AUDIT.md)        |
| XDG compliance                          | [docs/XDG_COMPLIANCE.md](docs/XDG_COMPLIANCE.md) · [docs/XDG_QUICKREF.md](docs/XDG_QUICKREF.md)                    |
| Architecture                            | [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) · [docs/ARCHITECTURE_V2.md](docs/ARCHITECTURE_V2.md)                  |
| Performance benchmarks                  | [docs/PERFORMANCE.md](docs/PERFORMANCE.md) · [docs/BENCHMARKS.md](docs/BENCHMARKS.md)                              |
| Agent discovery (JSON-LD, MCP, OpenAPI) | [AGENTS.md](AGENTS.md) · [`.well-known/`](.well-known/)                                                            |
| Roadmap                                 | [ROADMAP.md](ROADMAP.md)                                                                                           |
| Changelog                               | [CHANGELOG.md](CHANGELOG.md)                                                                                       |

---

## Contributing

Pull requests welcome. Please:

1. Read [CONTRIBUTING.md](CONTRIBUTING.md).
2. Sign the [CLA](CLA.md) — required for all PRs.
3. Run `./validate.ps1` (or `./validate.sh`) before submitting. PRs must pass fmt, clippy, tests, and docs.

---

## License

Dual-licensed:

- **AGPL-3.0-or-later** — free for open-source use. Any modified version or network-facing service must release its source under the AGPL. See [LICENSE](LICENSE).
- **Commercial License** — for proprietary, SaaS, or closed-source use without AGPL obligations. See [COMMERCIAL_LICENSE.md](COMMERCIAL_LICENSE.md) or email **licensing@nervosys.ai**.

---

## Support

- 📖 [Documentation site](https://github.com/nervosys/IronVault/tree/master/docs) · [Local website/](website/)
- 💬 [GitHub Discussions](https://github.com/nervosys/IronVault/discussions)
- 🐛 [Issue tracker](https://github.com/nervosys/IronVault/issues)
- 📧 General: dev@nervosys.ai · Security: security@nervosys.ai · Licensing: licensing@nervosys.ai

---

**Built with 🦀 Rust for maximum security, performance, and reliability.**
