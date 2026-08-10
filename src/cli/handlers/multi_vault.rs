//! CLI handler for multi-vault management (iv vaults).

use ironvault::multi_vault::{VaultEntry, VaultRegistry};
use ironvault::{Result, VaultConfig, VaultError};

use crate::cli::args::VaultsCommands;

pub fn handle_vaults(command: VaultsCommands, config: VaultConfig) -> Result<()> {
    let mut reg = VaultRegistry::new(&config.dirs.config_dir)?;

    match command {
        VaultsCommands::Register {
            name,
            path,
            description,
        } => {
            let entry = VaultEntry {
                name: name.clone(),
                path,
                description,
                registered_at: chrono::Utc::now().to_rfc3339(),
            };
            reg.register(entry)?;
            println!("Vault '{}' registered", name);
        }
        VaultsCommands::Unregister { name } => {
            if !reg.unregister(&name)? {
                // Reporting success for a no-op removal makes a script that
                // checks the exit code believe the vault is gone.
                return Err(VaultError::NotFound(format!("registered vault '{name}'")));
            }
            println!("Vault '{}' unregistered", name);
        }
        VaultsCommands::Activate { name } => {
            reg.activate(&name)?;
            println!("Vault '{}' activated", name);
        }
        VaultsCommands::Deactivate => {
            reg.deactivate()?;
            println!("Active vault cleared");
        }
        VaultsCommands::List => {
            let vaults = reg.list();
            if vaults.is_empty() {
                println!("No registered vaults.");
            } else {
                for v in &vaults {
                    let active = if v.is_active { " (active)" } else { "" };
                    let exists = if v.exists { "" } else { " [missing]" };
                    println!("  {}{}{} — {}", v.name, active, exists, v.path.display());
                }
            }
        }
    }

    Ok(())
}
