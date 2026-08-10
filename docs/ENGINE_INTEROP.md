# Engine Interop

Register vault models with local inference engines — Ollama and LM Studio — for direct serving and experimentation.

## Quick Start

```bash
# Register with Ollama
iv register my-model --engine ollama

# Register with custom alias and system prompt
iv register my-model --engine ollama --alias my-assistant --system-prompt "You are a helpful assistant."

# Register with LM Studio
iv register my-model --engine lm-studio
```

## CLI Reference

```
iv register <NAME> --engine <ENGINE> [OPTIONS]

Arguments:
  <NAME>                  Model name in vault

Options:
  --engine <ENGINE>       Target engine: ollama or lm-studio
  -v, --version <V>       Model version (default: latest)
  --alias <NAME>          Override the registered model name
  --system-prompt <TEXT>  System prompt (Ollama only)
```

## Ollama Integration

The `register` command builds an Ollama Modelfile and runs `ollama create`:

1. Exports the model from the vault to a temporary GGUF file
2. Generates a Modelfile with `FROM`, `SYSTEM`, `TEMPLATE`, and `PARAMETER` directives
3. Runs `ollama create <name> -f <Modelfile>`
4. Reports success or failure with the registered model name

```bash
# Register and then use with Ollama
iv register llama2-finetuned --engine ollama --alias my-llama
ollama run my-llama
```

### Requirements

- Ollama must be installed and accessible in `PATH`
- Model should be in GGUF format (or convertible to it)

## LM Studio Integration

For LM Studio, the model is copied to the LM Studio models directory:

1. Exports the model from the vault
2. Copies to `~/.cache/lm-studio/models/<name>/`
3. Model appears in LM Studio's model browser on next refresh

### Default Model Directories

| Platform | Path |
|----------|------|
| Linux | `~/.cache/lm-studio/models/` |
| macOS | `~/.cache/lm-studio/models/` |
| Windows | `%LOCALAPPDATA%/lm-studio/models/` |

## Rust API

```rust
use ironvault::interop::{register_ollama, register_lm_studio, OllamaOptions, LmStudioOptions};

// Ollama
let result = register_ollama(&OllamaOptions {
    name: "my-model".to_string(),
    model_path: PathBuf::from("model.gguf"),
    system_prompt: Some("You are a helpful assistant.".to_string()),
    template: None,
    parameters: HashMap::new(),
})?;

// LM Studio
let result = register_lm_studio(&LmStudioOptions {
    name: "my-model".to_string(),
    model_path: PathBuf::from("model.gguf"),
    models_dir: None, // uses platform default
})?;

println!("{}: {}", result.engine, result.message);
```
