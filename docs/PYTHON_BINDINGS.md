# Python Bindings

IronVault provides native Python bindings via [PyO3](https://pyo3.rs/), delivering near-native performance for all vault operations. A pure-Python fallback (CLI subprocess wrapper) is available when the native extension is not installed.

## Installation

### Native (recommended)

Requires Rust toolchain and Python 3.9+:

```bash
pip install maturin
maturin develop --features python
```

Or build a wheel:

```bash
maturin build --release --features python
pip install target/wheels/ironvault-*.whl
```

### Pure-Python fallback

```bash
pip install -e .
```

The fallback wraps the `iv` CLI binary via subprocess. It provides `Vault`, `VaultConfig`, and `ModelFormat` but not `ModelCard`, `ModelMetadata`, `ModelVersion`, `ModelStream`, or `VaultBuilder`.

## Quick Start

```python
from ironvault import Vault, ModelMetadata, ModelCard

# Create and unlock a vault
vault = Vault()
vault.unlock(b"my-secret-passphrase")

# Store a model
with open("model.safetensors", "rb") as f:
    data = f.read()

meta = ModelMetadata("my-llm", "safetensors",
                     description="Fine-tuned LLaMA 7B",
                     framework="pytorch",
                     task="text-generation")
version = vault.store_model("my-llm", data, meta)
print(f"Stored version {version.version}, checksum: {version.checksum_sha256}")

# Retrieve a model
retrieved = vault.get_model("my-llm")  # returns bytes

# List models and versions
for name in vault.list_models():
    versions = vault.list_versions(name)
    print(f"{name}: {len(versions)} version(s)")

# Lock when done
vault.lock()
```

## API Reference

### Check native availability

```python
import ironvault
print(ironvault._NATIVE)  # True if native bindings are loaded
```

### `ModelFormat`

Identifies an AI model format.

```python
from ironvault import ModelFormat

# From name
fmt = ModelFormat("safetensors")
print(fmt.name)       # "SafeTensors"
print(fmt.extension)  # "safetensors"

# Auto-detect from filename
fmt = ModelFormat.detect("model.gguf")
print(fmt.name)       # "GGUF"

# Equality
assert ModelFormat("pytorch") == ModelFormat("pt")
```

Supported format names: `safetensors`, `gguf`, `pytorch`/`pt`/`pth`, `tensorrt`/`plan`, `onnx`, `mlx`, `coreml`/`mlmodel`, `torchscript`, `tflite`, `tensorflow`/`tf`/`pb`, `keras`/`h5`, `openvino`, `tvm`, `ncnn`, `mnn`, `rknn`, `caffe`, `mxnet`, `darknet`, `hdf5`, `pickle`/`pkl`, `numpy`/`npy`/`npz`.

### `ModelMetadata`

Metadata attached to a stored model.

```python
from ironvault import ModelMetadata

meta = ModelMetadata("my-model", "safetensors",
                     description="A fine-tuned model",
                     framework="pytorch",
                     task="text-generation",
                     architecture="transformer",
                     parameters=7_000_000_000)

# Read-only properties
meta.name           # "my-model"
meta.format         # ModelFormat
meta.description    # "A fine-tuned model"
meta.framework      # "pytorch"
meta.task           # "text-generation"
meta.architecture   # "transformer"
meta.parameters     # 7000000000

# Custom fields
meta.add_custom_field("quantization", "q4_k_m")
```

### `VaultConfig`

Vault configuration with XDG-compliant paths.

```python
from ironvault import VaultConfig

# Default XDG location
config = VaultConfig()
print(config.vault_path)

# Custom directory
config = VaultConfig("/path/to/vault")
```

### `Vault`

The main vault for storing and retrieving encrypted models.

```python
from ironvault import Vault, VaultConfig, ModelMetadata

# With default config
vault = Vault()

# With custom config
vault = Vault(config=VaultConfig("/my/vault"))

# Unlock / lock
vault.unlock(b"passphrase")
assert vault.is_unlocked
vault.lock()

# Store
vault.unlock(b"passphrase")
ver = vault.store_model("name", data_bytes, metadata)
ver = vault.store_model("name", data_bytes, metadata, parent_version=1)

# Retrieve
data = vault.get_model("name")             # latest version (bytes)
data = vault.get_model("name", version=2)  # specific version

# List
names = vault.list_models()                # list[str]
versions = vault.list_versions("name")     # list[ModelVersion]
lineage = vault.get_lineage("name", 3)     # list[ModelVersion]

# Delete
existed = vault.delete_version("name", 1)  # bool

# Stats
stats = vault.get_stats()  # dict: model_count, total_versions, total_size_bytes

# Change passphrase (re-encrypts all models)
count = vault.change_passphrase(b"new-passphrase")

# Streaming (for large models)
ver = vault.store_model_streamed("name", chunks_iterable, metadata)
stream = vault.get_model_streamed("name", chunk_size=8*1024*1024)
for chunk in stream:
    process(chunk)
print(stream.total_size, stream.remaining)
```

### `VaultBuilder`

Builder pattern for configuring vaults with optional backends.

```python
from ironvault import VaultBuilder, VaultConfig

vault = (VaultBuilder()
         .config(VaultConfig("/my/vault"))
         .sqlite_versions()          # use SQLite version backend
         .no_default_subscribers()   # disable audit/metrics subscribers
         .build())
vault.unlock(b"passphrase")
```

### `ModelVersion`

Read-only snapshot of a stored model version (returned by `store_model`, `list_versions`, `get_lineage`).

```python
ver.version             # int (sequential)
ver.checkpoint_id       # str (unique ID)
ver.timestamp           # str (RFC 3339)
ver.parent_version      # int | None
ver.format              # str
ver.size_bytes          # int
ver.compressed_size_bytes  # int
ver.checksum_sha256     # str
ver.metadata            # dict[str, str]
```

### `ModelCard`

Model documentation following Google/HuggingFace standards.

```python
from ironvault import ModelCard

card = ModelCard(
    name="my-llm",
    version="1.0",
    model_type="transformer",
    description="A fine-tuned LLaMA model",
    developers=["AI Team"],
    license="Apache-2.0",
    primary_use="Text generation",
    out_of_scope=["Medical advice"],
)

# Training data
card.set_training_data("OpenWebText corpus",
                       source="https://example.com",
                       preprocessing="Tokenized with BPE")

# Evaluation metrics
card.add_metric("perplexity", 12.5, "Lower is better")
card.add_metric("accuracy", 0.95, "Validation set accuracy")

# Custom metadata
card.add_metadata("fine_tune_epochs", "3")

# Serialization
json_str = card.to_json()
yaml_str = card.to_yaml()
md_str   = card.to_markdown()

# Deserialization
card = ModelCard.from_json(json_str)
card = ModelCard.from_yaml(yaml_str)
```

### `ModelStream`

Iterator for chunked model retrieval. Implements `__iter__` and `__next__`.

```python
stream = vault.get_model_streamed("my-model", chunk_size=4*1024*1024)
print(f"Total: {stream.total_size}, Remaining: {stream.remaining}")

for chunk in stream:
    file.write(chunk)  # each chunk is bytes
```

### Utility functions

```python
from ironvault import sha256_hex, rust_version

# SHA-256 hex digest
digest = sha256_hex(b"hello")

# Library version
print(rust_version())  # e.g. "1.2.1"
```

## Architecture

```
ironvault/
├── _native          ← PyO3 Rust extension (src/python.rs)
├── __init__.py      ← Auto-selects native or fallback
├── core/
│   ├── config.py    ← Pure-Python VaultConfig (fallback)
│   └── vault.py     ← Pure-Python Vault via CLI subprocess (fallback)
├── formats/
│   └── registry.py  ← Pure-Python ModelFormat enum (fallback)
├── crypto/
│   ├── fips.py      ← Standalone FIPS crypto (Python-native)
│   └── compression.py
└── version/
    └── control.py
```

## Testing

### Rust-side tests (requires Python 3.9+)

```bash
cargo test --lib --features python python::tests
```

These test the `parse_format` helper and format mapping logic.

### Python-side tests

```bash
pip install -e ".[dev]"
pytest tests/test_ironvault.py -v
```

## Feature matrix

| Feature         | Native (PyO3) | Fallback (CLI) |
| --------------- | -------------- | -------------- |
| Vault           | ✅             | ✅             |
| VaultConfig     | ✅             | ✅             |
| VaultBuilder    | ✅             | ❌             |
| ModelFormat     | ✅             | ✅             |
| ModelMetadata   | ✅             | ❌             |
| ModelVersion    | ✅             | ❌             |
| ModelCard       | ✅             | ❌             |
| ModelStream     | ✅             | ❌             |
| sha256_hex      | ✅             | ❌             |
| FIPSCrypto      | ❌             | ✅             |
