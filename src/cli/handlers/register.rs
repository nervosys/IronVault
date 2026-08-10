//! CLI handler for inference engine registration (iv register).

use ironvault::{
    register_lm_studio, register_ollama, LmStudioOptions, OllamaOptions, Result, VaultConfig,
    VaultError,
};

use crate::cli::helpers::{build_vault, prompt_passphrase};

pub fn handle_register(
    name: String,
    engine: String,
    version: Option<u32>,
    alias: Option<String>,
    system_prompt: Option<String>,
    config: VaultConfig,
    use_sqlite: bool,
) -> Result<()> {
    // Export model from vault to a temp location
    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
    let mut vault = build_vault(config, use_sqlite)?;
    vault.unlock(passphrase)?;

    let data = vault.get_model(&name, version)?;

    let export_dir = vault.get_config().dirs.cache_dir.join("exports");
    std::fs::create_dir_all(&export_dir)?;

    let export_path = export_dir.join(&name);
    std::fs::write(&export_path, &data)?;

    let model_name = alias.unwrap_or_else(|| name.clone());

    match engine.to_lowercase().as_str() {
        "ollama" => {
            let opts = OllamaOptions {
                name: model_name,
                model_path: export_path,
                system_prompt,
                template: None,
                parameters: vec![],
            };
            let result = register_ollama(&opts)?;
            println!("{}", result.message);
            if let Some(path) = &result.path {
                println!("Modelfile: {}", path);
            }
        }
        "lm-studio" | "lmstudio" => {
            let opts = LmStudioOptions {
                name: model_name,
                model_path: export_path,
                models_dir: None,
            };
            let result = register_lm_studio(&opts)?;
            println!("{}", result.message);
            if let Some(path) = &result.path {
                println!("Model location: {}", path);
            }
        }
        _ => {
            return Err(VaultError::InvalidInput(format!(
                "Unknown engine '{}'. Supported: ollama, lm-studio",
                engine
            )));
        }
    }

    Ok(())
}
