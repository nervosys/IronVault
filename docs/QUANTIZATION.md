# Quantization Pipeline

Profile-based quantization management for AI/ML models. Create named quantization profiles with specific methods, estimate output sizes, and apply consistent quantization across model versions.

## Quick Start

```bash
# Create a quantization profile
iv quantize set fast-q4 --method q4_k_m --description "Fast 4-bit quantization"

# List profiles
iv quantize list

# Estimate output size (1GB model → Q4_K_M)
iv quantize estimate --size 1000000000 --to q4_k_m

# Remove a profile
iv quantize remove fast-q4
```

## CLI Reference

```
iv quantize <COMMAND>

Commands:
  set       Create or update a quantization profile
  remove    Remove a quantization profile
  list      List quantization profiles
  estimate  Estimate output size for a quantization method
```

### `iv quantize set`

```
iv quantize set <NAME> --method <METHOD> [--description <DESC>]

Arguments:
  <NAME>              Profile name

Options:
  -m, --method <METHOD>       Quantization method
  -d, --description <DESC>    Description
```

### `iv quantize estimate`

```
iv quantize estimate --size <BYTES> --to <METHOD> [--from <METHOD>]

Options:
  -s, --size <BYTES>    Original file size in bytes
  -t, --to <METHOD>     Target quantization method
      --from <METHOD>   Source precision (default: f32)
```

## Quantization Methods

| Method   | Bits/Weight | Use Case                             |
| -------- | ----------- | ------------------------------------ |
| `q4_0`   | 4.0         | Maximum compression, lower quality   |
| `q4_k_m` | 4.5         | Good balance of size and quality     |
| `q5_k_m` | 5.5         | Higher quality, moderate compression |
| `q8_0`   | 8.0         | Near-lossless, minimal compression   |
| `f16`    | 16.0        | Half precision                       |
| `f32`    | 32.0        | Full precision (no quantization)     |

## Python API

```python
from ironvault import QuantProfileStore

store = QuantProfileStore("/path/to/vault")
store.set("fast-q4", "q4_k_m", "Fast 4-bit quantization")
profiles = store.list()
store.remove("fast-q4")

# Static method — estimate size
estimated = QuantProfileStore.estimate(1_000_000_000, "f32", "q4_k_m")
```

## REST API

| Method | Path                            | Description             |
| ------ | ------------------------------- | ----------------------- |
| `GET`  | `/api/v1/quantization/profiles` | List all profiles       |
| `POST` | `/api/v1/quantization/profiles` | Create/update a profile |
| `POST` | `/api/v1/quantization/estimate` | Estimate quantized size |

### Example: Create Profile

```bash
curl -X POST http://localhost:8080/api/v1/quantization/profiles \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "fast-q4", "method": "q4_k_m", "description": "Fast 4-bit"}'
```

### Example: Estimate Size

```bash
curl -X POST http://localhost:8080/api/v1/quantization/estimate \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"size": 1000000000, "from_method": "f32", "to_method": "q4_k_m"}'
```

## Library API

```rust
use ironvault::{QuantProfileStore, QuantMethod, QuantProfile};

let store = QuantProfileStore::new("/path/to/vault")?;
store.set(QuantProfile {
    name: "fast-q4".into(),
    method: QuantMethod::Q4KM,
    description: Some("Fast 4-bit quantization".into()),
})?;

let profiles = store.list()?;
store.remove("fast-q4")?;

// Estimate size
let estimated = ironvault::estimate_quantized_size(1_000_000_000, &QuantMethod::F32, &QuantMethod::Q4KM);
```
