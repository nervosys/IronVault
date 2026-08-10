# Model Download

Download models from HuggingFace Hub, Ollama registry, or HTTPS URLs with SHA-256 integrity verification.

## Quick Start

```bash
# Download from HuggingFace
iv pull hf://TheBloke/Llama-2-7B-GGUF/llama-2-7b.Q4_K_M.gguf

# Download from Ollama
iv pull ollama://llama2:7b

# Download from URL with checksum verification
iv pull https://example.com/model.safetensors --sha256 abc123...

# Download and auto-store in vault
iv pull hf://user/repo/model.safetensors --store --name my-model
```

## Source Formats

| Prefix | Source | Example |
|--------|--------|---------|
| `hf://` | HuggingFace Hub | `hf://TheBloke/repo/file.gguf` |
| `ollama://` | Ollama Registry | `ollama://llama2:7b` |
| `https://` | Direct URL | `https://example.com/model.bin` |

## CLI Reference

```
iv pull <SOURCE> [OPTIONS]

Arguments:
  <SOURCE>          Model source URI (hf://, ollama://, https://)

Options:
  -o, --output <DIR>     Output directory (default: current directory)
  --sha256 <HASH>        Expected SHA-256 hash for verification
  --token <TOKEN>        HuggingFace API token (or HF_TOKEN env var)
  --store                Auto-store downloaded model in vault
  --name <NAME>          Model name when using --store
```

## Authentication

For private HuggingFace repositories:

```bash
# Via CLI flag
iv pull hf://private/repo/model.safetensors --token hf_xxxxx

# Via environment variable
export HF_TOKEN=hf_xxxxx
iv pull hf://private/repo/model.safetensors
```

## Security

- Only HTTPS URLs are accepted (HTTP is rejected)
- SHA-256 checksums are computed for every download
- Optional `--sha256` flag enforces integrity verification
- Downloads are written atomically to prevent partial files

## Rust API

```rust
use ironvault::download::{ModelDownloader, ModelSource};

let source = ModelSource::parse("hf://TheBloke/repo/model.gguf")?;
let downloader = ModelDownloader::new("./models")
    .with_hf_token("hf_xxxxx".to_string());
let result = downloader.download(&source, None)?;

println!("Downloaded to: {}", result.path.display());
println!("SHA-256: {}", result.sha256);
println!("Size: {} bytes", result.size_bytes);
```
