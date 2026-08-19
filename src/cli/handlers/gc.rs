//! CLI handler for garbage collection (iv gc).

use ironvault::{Result, VaultConfig};

use crate::cli::helpers::{build_vault, prompt_passphrase};

pub fn handle_gc(dry_run: bool, config: VaultConfig) -> Result<()> {
    // 8.0 requires the passphrase here. The version index is sealed, and gc
    // deletes blobs the index does not reference -- so running it against a
    // locked vault would see an empty index, call every blob an orphan, and
    // delete the entire vault.
    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
    let mut vault = build_vault(config, false)?;
    vault.unlock(passphrase)?;

    let report = vault.gc(dry_run)?;

    if dry_run {
        println!("Dry-run — no files removed.");
    }

    println!("Referenced blobs: {}", report.referenced_blobs);
    println!("Orphaned blobs:   {}", report.orphaned_blobs.len());
    println!("Temp files:       {}", report.temp_files.len());
    println!("Reclaimable:      {} bytes", report.reclaimable_bytes);

    Ok(())
}
