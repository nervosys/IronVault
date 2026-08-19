//! CLI handler for TUI browse (iv browse).

use ironvault::{Result, VaultConfig};

use crate::cli::helpers::{build_vault, prompt_passphrase};

pub fn handle_browse(config: VaultConfig) -> Result<()> {
    // 8.0 requires the passphrase: the dashboard reads the version index,
    // which is sealed.
    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
    let mut vault = build_vault(config, false)?;
    vault.unlock(passphrase)?;

    println!("{}", vault.browse()?);
    Ok(())
}
