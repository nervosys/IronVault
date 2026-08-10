//! CLI handler for quantization profiles (iv quantize).

use ironvault::quantization::{QuantMethod, QuantProfile, QuantProfileStore};
use ironvault::{Result, VaultConfig, VaultError};

use crate::cli::args::QuantizeCommands;

pub fn handle_quantize(command: QuantizeCommands, config: VaultConfig) -> Result<()> {
    let mut store = QuantProfileStore::new(&config.dirs.vault_dir)?;

    match command {
        QuantizeCommands::Set {
            name,
            method,
            description,
        } => {
            let method: QuantMethod = method.parse().map_err(|e: VaultError| e)?;
            let profile = QuantProfile {
                name: name.clone(),
                method,
                description,
                metadata: std::collections::BTreeMap::new(),
            };
            store.set(profile)?;
            println!("Quantization profile '{}' set (method: {})", name, method);
        }
        QuantizeCommands::Remove { name } => {
            if !store.remove(&name)? {
                return Err(VaultError::NotFound(format!(
                    "quantization profile '{name}'"
                )));
            }
            println!("Profile '{}' removed", name);
        }
        QuantizeCommands::List => {
            let profiles = store.list();
            if profiles.is_empty() {
                println!("No quantization profiles.");
            } else {
                for p in profiles {
                    let desc = p.description.as_deref().unwrap_or("");
                    println!("  {} — {} {}", p.name, p.method, desc);
                }
            }
        }
        QuantizeCommands::Estimate { size, from, to } => {
            let from_method: QuantMethod = from.parse()?;
            let to_method: QuantMethod = to.parse()?;
            let estimated =
                ironvault::quantization::estimate_quantized_size(size, from_method, to_method);
            let ratio = size as f64 / estimated as f64;
            println!(
                "Estimated: {} → {} bytes ({:.1}× compression, {} → {})",
                size, estimated, ratio, from_method, to_method
            );
        }
    }

    Ok(())
}
