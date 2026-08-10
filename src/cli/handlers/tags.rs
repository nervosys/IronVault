//! CLI handler for model tags and search (iv tag / iv search).

use ironvault::tags::SearchQuery;
use ironvault::{Result, TagStore, VaultConfig, VaultError};

use crate::cli::args::TagCommands;

pub fn handle_tag(command: TagCommands, config: VaultConfig) -> Result<()> {
    let mut store = TagStore::new(&config.dirs.vault_dir)?;

    match command {
        TagCommands::Add { model, tags } => {
            store.add_tags(&model, &tags)?;
            println!("Added {} tag(s) to '{}'", tags.len(), model);
        }
        TagCommands::Remove { model, tags } => {
            store.remove_tags(&model, &tags)?;
            println!("Removed {} tag(s) from '{}'", tags.len(), model);
        }
        TagCommands::List { model } => {
            let tags = store.get_tags(&model);
            if tags.is_empty() {
                println!("No tags for '{}'", model);
            } else {
                for tag in &tags {
                    println!("  {}", tag);
                }
            }
        }
    }

    Ok(())
}

pub fn handle_search(
    query: String,
    tags: Vec<String>,
    format: Option<String>,
    config: VaultConfig,
) -> Result<()> {
    let store = TagStore::new(&config.dirs.vault_dir)?;

    let sq = SearchQuery {
        name_pattern: if query.is_empty() { None } else { Some(query) },
        tags,
        ..Default::default()
    };

    // We need model names for the search — get from version control
    let vc = ironvault::version::VersionControl::new(&config.dirs.vault_dir)?;
    let models = vc.list_models_owned();

    let results = store.search(&sq, &models);

    let fmt = format.unwrap_or_else(|| "text".into());
    match fmt.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&results)
                .map_err(|e| VaultError::SerializationError(e.to_string()))?;
            println!("{}", json);
        }
        _ => {
            if results.is_empty() {
                println!("No models matched the search.");
            } else {
                for r in &results {
                    println!("  {} tags: {:?}", r.model, r.tags);
                }
            }
        }
    }

    Ok(())
}
