//! CLI handler for config profiles (iv profile).

use ironvault::profiles::Profile;
use ironvault::{ProfileStore, Result, VaultConfig, VaultError};

use crate::cli::args::ProfileCommands;

pub fn handle_profile(command: ProfileCommands, config: VaultConfig) -> Result<()> {
    let mut store = ProfileStore::new(&config.dirs.config_dir)?;

    match command {
        ProfileCommands::Create { name, set } => {
            let mut overrides = std::collections::BTreeMap::new();
            for kv in &set {
                if let Some((k, v)) = kv.split_once('=') {
                    overrides.insert(k.to_string(), v.to_string());
                }
            }
            let profile = Profile {
                name: name.clone(),
                description: None,
                overrides,
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            store.set(profile)?;
            println!("Profile '{}' created", name);
        }
        ProfileCommands::Activate { name } => {
            store.activate(&name)?;
            println!("Profile '{}' activated", name);
        }
        ProfileCommands::Deactivate => {
            store.deactivate()?;
            println!("Profile deactivated");
        }
        ProfileCommands::List => {
            let profiles = store.list();
            let active = store.active();
            let active_name = active.map(|p| p.name.as_str());
            if profiles.is_empty() {
                println!("No profiles.");
            } else {
                for p in profiles {
                    let marker = if active_name == Some(p.name.as_str()) {
                        " (active)"
                    } else {
                        ""
                    };
                    println!("  {}{}", p.name, marker);
                    for (k, v) in &p.overrides {
                        println!("    {} = {}", k, v);
                    }
                }
            }
        }
        ProfileCommands::Show { name } => {
            let Some(p) = store.get(&name) else {
                return Err(VaultError::NotFound(format!("profile '{name}'")));
            };
            println!("Profile '{}':", p.name);
            for (k, v) in &p.overrides {
                println!("  {} = {}", k, v);
            }
        }
        ProfileCommands::Delete { name } => {
            store.remove(&name)?;
            println!("Profile '{}' deleted", name);
        }
    }

    Ok(())
}
