# Model Diffing

Compare model versions at the tensor level — shapes, data types, parameter counts — by parsing headers only (no full model loading required).

## Quick Start

```bash
# Compare two files
iv diff model_v1.safetensors model_v2.safetensors

# Compare vault model versions
iv diff mymodel@v1 mymodel@v2

# JSON output
iv diff model_v1.gguf model_v2.gguf --format json
```

## CLI Reference

```
iv diff <LEFT> <RIGHT> [OPTIONS]

Arguments:
  <LEFT>              Left model (file path or name@version)
  <RIGHT>             Right model (file path or name@version)

Options:
  -f, --format <FMT>  Output format: text (default) or json
```

### Model References

- **File path**: `./models/model.safetensors`
- **Vault reference**: `mymodel@v1`, `mymodel@v2`

## Supported Formats

| Format | Diff Capability |
|--------|----------------|
| SafeTensors | Full tensor-level diff (shapes, dtypes, params) |
| GGUF | Full tensor-level diff from metadata headers |
| Other | Size comparison only |

## Output Example

```
=== Model Diff ===
Left:  model_v1.safetensors (2.4 GB)
Right: model_v2.safetensors (2.5 GB)

Size delta: +100 MB

Added tensors (3):
  + lm_head.bias          [32000]           f32    32,000 params
  + model.norm.weight     [4096]            f16    4,096 params
  + model.norm.bias       [4096]            f16    4,096 params

Removed tensors (0): none

Changed tensors (1):
  ~ model.embed.weight    [32000, 4096] → [32001, 4096]  +4,096 params

Summary:
  Left:  200 tensors | Right: 203 tensors
  Added: 3 | Removed: 0 | Changed: 1 | Unchanged: 199
  Parameter delta: +40,192
  Size change: +4.2%
```

## Rust API

```rust
use ironvault::diff::ModelDiffer;

let diff = ModelDiffer::diff_files(
    Path::new("model_v1.safetensors"),
    Path::new("model_v2.safetensors"),
    Some("v1"),
    Some("v2"),
)?;

println!("{}", diff.display());
println!("Added: {}", diff.added_tensors.len());
println!("Changed: {}", diff.changed_tensors.len());
println!("Param delta: {}", diff.param_delta);
```
