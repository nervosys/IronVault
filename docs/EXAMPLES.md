# Examples Guide

IronVault ships with 11 runnable examples covering every major feature.

## Running Examples

```bash
# Basic pattern
cargo run --example <NAME> --release

# Examples requiring specific features
cargo run --example api_demo --features api --release
```

## Example Catalog

| #   | Example                  | Features Required | Description                                          |
| --- | ------------------------ | ----------------- | ---------------------------------------------------- |
| 1   | `basic_usage`            | default           | Core vault operations: create, store, retrieve, list |
| 2   | `api_demo`               | `api`             | Start REST API server and interact via HTTP          |
| 3   | `security_demo`          | default           | FIPS 140-3 / CMMC / MITRE compliance checks          |
| 4   | `model_card_demo`        | default           | Create model cards, export to JSON/YAML/Markdown     |
| 5   | `version_control_demo`   | default           | Multi-version storage, lineage, rollback             |
| 6   | `providers_formats_demo` | default           | Format detection and conversion paths                |
| 7   | `rag_demo`               | default           | RAG document store, search, rule engine              |
| 8   | `mcp_tools_demo`         | default           | MCP tool registration and execution                  |
| 9   | `utilities_demo`         | default           | Analysis, deduplication, quantization, pruning       |
| 10  | `xdg_demo`               | default           | XDG Base Directory compliance and paths              |
| 11  | `huggingface_demo`       | default           | End-to-end HuggingFace model workflow                |

## Detailed Descriptions

### 1. basic_usage

The starting point for new users. Demonstrates the `VaultBuilder`, unlocking
with a passphrase, storing a model with metadata, retrieving and decrypting it,
listing vault contents, and version history.

```bash
cargo run --example basic_usage --release
```

### 2. api_demo

Starts an in-process Axum HTTP server on a random port and exercises the REST
API programmatically: health check, JWT authentication, listing models, vault
statistics, available conversion paths, compliance checks, and the OpenAPI spec.

```bash
cargo run --example api_demo --features api --release
```

### 3. security_demo

Runs the full compliance suite — FIPS 140-3, CVE scanning, MITRE ATT&CK
mapping, and CMMC 2.0 Level 2 assessment — and prints the results.

```bash
cargo run --example security_demo --release
```

### 4. model_card_demo

Creates model cards for an LLM and a medical imaging model, demonstrates
fairness analysis, environmental impact tracking, and export to JSON, YAML,
and HuggingFace-compatible Markdown.

```bash
cargo run --example model_card_demo --release
```

### 5. version_control_demo

Stores multiple versions of a model, queries version history, navigates the
lineage tree, and demonstrates rollback to a previous version.

```bash
cargo run --example version_control_demo --release
```

### 6. providers_formats_demo

Detects model formats from file extensions and magic bytes, lists all 23+
supported formats, and shows available conversion paths between formats.

```bash
cargo run --example providers_formats_demo --release
```

### 7. rag_demo

Initializes a RAG document store, adds documents with metadata and embeddings,
performs similarity search, and executes rules from the rule engine.

```bash
cargo run --example rag_demo --release
```

### 8. mcp_tools_demo

Registers custom MCP (Model Context Protocol) tools and executes them,
demonstrating the tool definition schema, executor functions, and result
handling.

```bash
cargo run --example mcp_tools_demo --release
```

### 9. utilities_demo

Exercises the model utilities: size analysis, hash-based deduplication,
similarity-based deduplication, quantization estimation, pruning analysis,
compression analysis, and model export.

```bash
cargo run --example utilities_demo --release
```

### 10. xdg_demo

Shows XDG Base Directory Specification compliance: config, data, cache, and
log directory resolution on Linux, macOS, and Windows.

```bash
cargo run --example xdg_demo --release
```

### 11. huggingface_demo

End-to-end workflow with a HuggingFace model: store a SafeTensors model,
create a model card, inspect format metadata, and convert between formats.

```bash
cargo run --example huggingface_demo --release
```
