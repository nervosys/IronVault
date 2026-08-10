//! CLI handler for garbage collection (iv gc).

use ironvault::{Result, VaultConfig};

pub fn handle_gc(dry_run: bool, config: VaultConfig) -> Result<()> {
    let report = ironvault::gc::gc(&config.dirs.vault_dir, dry_run)?;

    if dry_run {
        println!("Dry-run — no files removed.");
    }

    println!("Referenced blobs: {}", report.referenced_blobs);
    println!("Orphaned blobs:   {}", report.orphaned_blobs.len());
    println!("Temp files:       {}", report.temp_files.len());
    println!("Reclaimable:      {} bytes", report.reclaimable_bytes);

    Ok(())
}
