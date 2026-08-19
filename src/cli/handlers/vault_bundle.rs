//! CLI handler for vault export/import (iv vault-export / iv vault-import).

use ironvault::{Result, VaultConfig};
use std::path::PathBuf;

use crate::cli::helpers::{build_vault, prompt_passphrase};

pub fn handle_vault_export(output: PathBuf, config: VaultConfig) -> Result<()> {
    // 8.0 requires the passphrase: the bundle is built from the version index,
    // which is sealed.
    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
    let mut vault = build_vault(config, false)?;
    vault.unlock(passphrase)?;

    let report = vault.export_bundle(&output, None)?;
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
    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
    let mut vault = build_vault(config, false)?;
    vault.unlock(passphrase)?;

    let report = match target {
        // Importing into the configured vault: the vault holds its own key.
        None => vault.import_bundle(&archive, false)?,
        // Importing elsewhere. The target's index is sealed with the target's
        // key, so the passphrase entered above must be that vault's -- if it
        // is not, unlocking the target index fails the AEAD tag and this
        // returns `Authentication failed` rather than writing a mixed vault.
        Some(dest) => vault.import_bundle_into(&dest, &archive, false)?,
    };

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
