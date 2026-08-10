//! Example: Engine interop — register models with Ollama and LM Studio
//!
//! Run with: cargo run --example interop_demo

use ironvault::interop::{InferenceEngine, LmStudioOptions, OllamaOptions};
use std::path::PathBuf;

fn main() -> ironvault::Result<()> {
    println!("=== IronVault Engine Interop Example ===\n");

    // 1. Ollama registration
    println!("1. Ollama registration...");
    println!("   Creates a Modelfile and runs 'ollama create'");
    println!();

    let ollama_opts = OllamaOptions {
        name: "my-assistant".to_string(),
        model_path: PathBuf::from("model.gguf"),
        system_prompt: Some("You are a helpful AI assistant.".to_string()),
        template: None,
        parameters: Vec::new(),
    };
    println!("   Options:");
    println!("     Name: {}", ollama_opts.name);
    println!("     Model: {}", ollama_opts.model_path.display());
    println!("     System prompt: {:?}", ollama_opts.system_prompt);

    // Note: This would fail without Ollama installed, so we show the workflow
    println!("   → Would generate Modelfile:");
    println!("     FROM model.gguf");
    println!("     SYSTEM You are a helpful AI assistant.");
    println!("   → Would run: ollama create my-assistant -f Modelfile\n");

    // 2. LM Studio registration
    println!("2. LM Studio registration...");
    println!("   Copies GGUF model to LM Studio models directory");
    println!();

    let lm_opts = LmStudioOptions {
        name: "my-model".to_string(),
        model_path: PathBuf::from("model.gguf"),
        models_dir: None,
    };
    println!("   Options:");
    println!("     Name: {}", lm_opts.name);
    println!("     Model: {}", lm_opts.model_path.display());
    println!("     Dir: platform default (~/.cache/lm-studio/models/)\n");

    // 3. CLI usage
    println!("3. CLI commands:");
    println!("   iv register my-model --engine ollama");
    println!(
        "   iv register my-model --engine ollama --alias chat-bot --system-prompt 'Be helpful'"
    );
    println!("   iv register my-model --engine lm-studio");
    println!("   iv register my-model --engine lm-studio --version 2\n");

    // 4. Supported engines
    println!("4. Supported engines:");
    println!(
        "   {:?} — Generates Modelfile, runs 'ollama create'",
        InferenceEngine::Ollama
    );
    println!(
        "   {:?} — Copies to LM Studio models directory\n",
        InferenceEngine::LmStudio
    );

    println!("=== Interop example complete ===");
    Ok(())
}
