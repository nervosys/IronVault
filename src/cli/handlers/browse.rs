//! CLI handler for TUI browse (iv browse).

use ironvault::{Result, VaultConfig};

pub fn handle_browse(config: VaultConfig) -> Result<()> {
    let output = ironvault::tui::browse(&config.dirs.vault_dir)?;
    println!("{}", output);
    Ok(())
}
