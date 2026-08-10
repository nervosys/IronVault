//! CLI handler for license scanning (iv license-scan).

use std::path::PathBuf;

use ironvault::{LicenseScanner, Result, VaultError};

pub fn handle_license_scan(path: PathBuf, format: String) -> Result<()> {
    let report = if path.is_dir() {
        LicenseScanner::scan_directory(&path)?
    } else {
        LicenseScanner::scan_file(&path)?
    };

    match format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&report)
                .map_err(|e| VaultError::SerializationError(e.to_string()))?;
            println!("{}", json);
        }
        _ => {
            println!("{}", report.display());
        }
    }

    Ok(())
}
