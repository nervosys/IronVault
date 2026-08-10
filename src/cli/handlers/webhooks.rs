//! CLI handler for webhook management (iv webhook).

use ironvault::webhooks::WebhookTarget;
use ironvault::{Result, VaultConfig, WebhookStore};

use crate::cli::args::WebhookCommands;

pub fn handle_webhook(command: WebhookCommands, config: VaultConfig) -> Result<()> {
    let mut store = WebhookStore::new(&config.dirs.vault_dir)?;

    match command {
        WebhookCommands::Add {
            id,
            url,
            secret,
            events,
        } => {
            let target = WebhookTarget {
                id: id.clone(),
                url,
                secret,
                events,
                enabled: true,
            };
            store.add(target)?;
            println!("Webhook '{}' registered", id);
        }
        WebhookCommands::Remove { id } => {
            if store.remove(&id)? {
                println!("Webhook '{}' removed", id);
            } else {
                println!("Webhook '{}' not found", id);
            }
        }
        WebhookCommands::List => {
            let targets = store.list();
            if targets.is_empty() {
                println!("No webhooks registered.");
            } else {
                for t in targets {
                    let status = if t.enabled { "enabled" } else { "disabled" };
                    let events = if t.events.is_empty() {
                        "all".to_string()
                    } else {
                        t.events.join(", ")
                    };
                    println!("  {} -> {} [{}] events: {}", t.id, t.url, status, events);
                }
            }
        }
    }

    Ok(())
}
