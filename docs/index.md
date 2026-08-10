# IronVault

**Universal cross-platform secure vault for AI model storage, versioning, and management with military-grade encryption.**

---

## What is IronVault?

IronVault (`iv`) is a production-ready, FIPS-approved-algorithm encrypted storage system for AI/ML models. It provides:

- **Encrypted Storage** — AES-256-GCM encryption with Argon2id key derivation
- **Version Control** — Sequential versioning with parent lineage trees and instant rollback
- **23+ Model Formats** — SafeTensors, GGUF, ONNX, PyTorch, TensorRT, CoreML, MLX, and more
- **Format Conversion** — Convert between formats with optional quantization
- **Cloud Storage** — Push/pull to AWS S3, Azure Blob, Google Cloud Storage
- **RAG System** — Document store, knowledge base, rule engine with MCP tool integration
- **REST & GraphQL APIs** — Axum-based server with JWT authentication
- **Compliance** — FIPS 140-3, CMMC 2.0 Level 2, MITRE ATT&CK validation

## Quick Install

=== "From source (Rust)"

    ```bash
    git clone https://github.com/nervosys/IronVault.git
    cd IronVault
    cargo build --release
    ```

=== "Python bindings"

    ```bash
    pip install maturin
    git clone https://github.com/nervosys/IronVault.git
    cd IronVault
    maturin develop --features python
    ```

## First Steps

```bash
# Initialize an encrypted vault
iv init

# Store a model (format auto-detected)
iv store my-model ./model.safetensors -d "Fine-tuned LLaMA 7B"

# List stored models
iv list

# Retrieve and decrypt
iv get my-model ./output.safetensors
```

## Next Steps

| Guide                                 | Description                   |
| ------------------------------------- | ----------------------------- |
| [Quick Start](QUICKSTART.md)          | Full walkthrough in 5 minutes |
| [CLI Reference](CLI.md)               | All `iv` commands documented |
| [Top 10 Features](TOP_10_FEATURES.md) | Why users love IronVault |
| [Security Audit](SECURITY_AUDIT.md)   | Complete security review      |
| [Architecture](ARCHITECTURE.md)       | System design deep-dive       |

!!! info "1,831 tests passing"
    IronVault has comprehensive test coverage across encryption, format detection, version control, model cards, RAG, API, and CLI — all verified on every commit.
