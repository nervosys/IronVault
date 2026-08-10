//! CLI handler for model diffing (iv diff).

use std::path::Path;

use ironvault::{ModelDiffer, Result, VaultConfig, VaultError};

use crate::cli::helpers::{build_vault, prompt_passphrase};

pub fn handle_diff(
    left: String,
    right: String,
    format: String,
    config: VaultConfig,
    use_sqlite: bool,
) -> Result<()> {
    // Parse "name@version" or treat as file path
    let (left_path, left_label, _temp_left) = resolve_model(&left, &config, use_sqlite)?;
    let (right_path, right_label, _temp_right) = resolve_model(&right, &config, use_sqlite)?;

    let diff = ModelDiffer::diff_files(
        Path::new(&left_path),
        Path::new(&right_path),
        &left_label,
        &right_label,
    )?;

    match format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&diff)
                .map_err(|e| VaultError::SerializationError(e.to_string()))?;
            println!("{}", json);
        }
        _ => {
            println!("{}", diff.display());
        }
    }

    Ok(())
}

/// Resolve a model reference to a file path.
/// Returns (path, label, optional_tempdir_to_keep_alive).
fn resolve_model(
    reference: &str,
    config: &VaultConfig,
    use_sqlite: bool,
) -> Result<(String, String, Option<tempfile::TempDir>)> {
    // Check if it's a file path
    if std::path::Path::new(reference).exists() {
        return Ok((reference.to_string(), reference.to_string(), None));
    }

    // Parse name@version
    let (name, version) = if let Some(at_pos) = reference.rfind('@') {
        let name = &reference[..at_pos];
        let ver_str = &reference[at_pos + 1..];
        let ver: u32 = ver_str
            .strip_prefix('v')
            .unwrap_or(ver_str)
            .parse()
            .map_err(|_| {
                VaultError::InvalidInput(format!(
                    "Invalid version in '{}': '{}'",
                    reference, ver_str
                ))
            })?;
        (name.to_string(), Some(ver))
    } else {
        (reference.to_string(), None)
    };

    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
    let mut vault = build_vault(config.clone(), use_sqlite)?;
    vault.unlock(passphrase)?;

    let data = vault.get_model(&name, version)?;

    let temp_dir = tempfile::tempdir().map_err(VaultError::IoError)?;
    let temp_path = temp_dir.path().join(&name);
    std::fs::write(&temp_path, &data)?;

    let label = if let Some(v) = version {
        format!("{}@v{}", name, v)
    } else {
        format!("{}@latest", name)
    };

    let path_str = temp_path.display().to_string();
    Ok((path_str, label, Some(temp_dir)))
}
