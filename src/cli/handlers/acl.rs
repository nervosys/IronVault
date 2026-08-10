//! CLI handler for access control (iv acl).

use ironvault::access_control::Role;
use ironvault::{AclGuard, Result, VaultConfig};

use crate::cli::args::AclCommands;

pub fn handle_acl(command: AclCommands, config: VaultConfig) -> Result<()> {
    let mut guard = AclGuard::new(&config.dirs.vault_dir)?;

    match command {
        AclCommands::Grant { identity, role } => {
            let r: Role = role.parse()?;
            guard.grant(&identity, r)?;
            println!("Granted '{}' to '{}'", role, identity);
        }
        AclCommands::Revoke { identity } => {
            guard.revoke(&identity)?;
            println!("Revoked access for '{}'", identity);
        }
        AclCommands::List => {
            let entries = guard.list();
            if entries.is_empty() {
                println!("No ACL entries.");
            } else {
                for entry in entries {
                    println!("  {} -> {:?}", entry.principal, entry.role);
                }
            }
        }
    }

    Ok(())
}
