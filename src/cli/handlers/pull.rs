//! CLI handler for model download (iv pull).

use std::path::PathBuf;

use ironvault::formats::{ModelFormat, ModelMetadata};
use ironvault::{ModelDownloader, ModelSource, Result, VaultConfig};

use crate::cli::helpers::{build_vault, prompt_passphrase};

#[allow(clippy::too_many_arguments)]
pub fn handle_pull(
    source: String,
    output: Option<PathBuf>,
    sha256: Option<String>,
    token: Option<String>,
    store: bool,
    name: Option<String>,
    config: VaultConfig,
    use_sqlite: bool,
) -> Result<()> {
    let parsed = ModelSource::parse(&source)?;

    let output_dir = output.unwrap_or_else(|| PathBuf::from("."));

    let mut downloader = ModelDownloader::new(&output_dir);
    if let Some(tok) = token {
        downloader = downloader.with_hf_token(tok);
    }

    println!("Downloading from: {}", source);
    let result = downloader.download(&parsed, sha256.as_deref())?;

    println!(
        "Downloaded: {} ({} bytes, SHA-256: {})",
        result.path.display(),
        result.size_bytes,
        result.sha256
    );

    if store {
        let model_name = name.unwrap_or_else(|| {
            result
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("model")
                .to_string()
        });

        let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
        let mut vault = build_vault(config, use_sqlite)?;
        vault.unlock(passphrase)?;

        let format_str = result
            .path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");
        let format = ModelFormat::from_extension(format_str);

        let data = std::fs::read(&result.path)?;
        let metadata = ModelMetadata::new(model_name.clone(), format);

        vault.store_model(&model_name, data, metadata, None)?;
        println!("Stored as '{}' in vault", model_name);
    }

    Ok(())
}
