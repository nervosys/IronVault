//! CLI handler for pickle safety scanning (iv scan).

use std::path::PathBuf;

use ironvault::{PickleScanner, Result, VaultConfig, VaultError};

use crate::cli::helpers::{build_vault, prompt_passphrase};

pub fn handle_scan(
    name: Option<String>,
    file: Option<PathBuf>,
    version: Option<u32>,
    format: String,
    config: VaultConfig,
    use_sqlite: bool,
) -> Result<()> {
    let report = if let Some(file_path) = file {
        // Scan a file on disk
        println!("Scanning: {}", file_path.display());
        PickleScanner::scan(&file_path)?
    } else if let Some(model_name) = name {
        // Scan a vault model
        let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
        let mut vault = build_vault(config, use_sqlite)?;
        vault.unlock(passphrase)?;

        let data = vault.get_model(&model_name, version)?;

        let temp_dir = tempfile::tempdir().map_err(VaultError::IoError)?;
        let temp_path = temp_dir.path().join(&model_name);
        std::fs::write(&temp_path, &data)?;

        println!(
            "Scanning vault model: {} (v{})",
            model_name,
            version.unwrap_or(0)
        );
        PickleScanner::scan(&temp_path)?
    } else {
        return Err(VaultError::InvalidInput(
            "Provide either a model name or --file".to_string(),
        ));
    };

    if format.as_str() == "json" {
        let json = serde_json::to_string_pretty(&report)
            .map_err(|e| VaultError::SerializationError(e.to_string()))?;
        println!("{}", json);
    } else {
        println!("File: {} ({} bytes)", report.file_path, report.file_size);
        println!(
            "Pickle format: {} | ZIP archive: {}",
            report.is_pickle_format, report.is_zip_archive
        );
        println!("Safe: {}\n", if report.safe { "YES" } else { "NO" });
        for finding in &report.findings {
            println!(
                "  [{}] {} — {} (×{})",
                finding.severity, finding.code, finding.description, finding.count
            );
        }
        println!("\n{}", report.recommendation);
    }

    Ok(())
}
