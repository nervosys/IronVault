//! CLI handler for vault export/import (iv vault-export / iv vault-import).

use ironvault::{Result, VaultConfig};
use std::path::PathBuf;

pub fn handle_vault_export(output: PathBuf, config: VaultConfig) -> Result<()> {
    let report = ironvault::vault_bundle::export_vault(&config.dirs.vault_dir, &output, None)?;
    println!("Exported vault to {:?}", output);
    println!("  Models: {}", report.models_exported.len());
    println!("  Versions: {}", report.total_versions);
    println!("  Blobs: {}", report.total_blobs);
    Ok(())
}

pub fn handle_vault_import(
    archive: PathBuf,
    target: Option<PathBuf>,
    config: VaultConfig,
) -> Result<()> {
    let dest = target.unwrap_or_else(|| config.dirs.vault_dir.clone());
    let report = ironvault::vault_bundle::import_vault(&dest, &archive, false)?;
    println!("Imported vault from {:?}", archive);
    println!("  Models: {}", report.models_imported);
    println!("  Versions imported: {}", report.versions_imported);
    println!("  Versions skipped: {}", report.versions_skipped);
    if report.checksum_verified {
        println!("  Integrity: payload checksum verified");
    } else {
        // Do not let silence read as a passing check.
        println!(
            "  Integrity: NOT VERIFIED — this bundle predates reproducible \
             checksums (format version 1)"
        );
    }
    Ok(())
}
