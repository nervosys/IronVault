//! Introspect CLI command handler.
//!
//! Outputs a complete machine-readable CLI schema for agent discovery.
//! Supports JSON, YAML, and JSON-LD output formats.

use ironvault::Result;
use serde_json::{json, Value};

/// Handle the introspect command — output full CLI schema for agents.
pub fn handle_introspect(format: String, compact: bool) -> Result<()> {
    let schema = ironvault::cli_schema::build(compact);

    match format.to_lowercase().as_str() {
        "json" => {
            let output = if compact {
                serde_json::to_string(&schema)
                    .map_err(|e| ironvault::VaultError::SerializationError(e.to_string()))?
            } else {
                serde_json::to_string_pretty(&schema)
                    .map_err(|e| ironvault::VaultError::SerializationError(e.to_string()))?
            };
            println!("{output}");
        }
        "yaml" | "yml" => {
            let output = serde_yaml_ng::to_string(&schema)
                .map_err(|e| ironvault::VaultError::SerializationError(e.to_string()))?;
            println!("{output}");
        }
        "jsonld" | "json-ld" => {
            let mut ld = serde_json::Map::new();
            ld.insert(
                "@context".to_string(),
                json!({
                    // Must match the `iv` term in `.well-known/ontology.jsonld`.
                    // It previously pointed at a different host, so the CLI's
                    // JSON-LD and the published ontology minted distinct IRIs
                    // for the same terms — consumers that joined them saw two
                    // unrelated vocabularies.
                    "iv": "https://nervosys.com/ontology/iv#",
                    "schema": "https://schema.org/",
                    "rdfs": "http://www.w3.org/2000/01/rdf-schema#"
                }),
            );
            ld.insert("@type".to_string(), json!("iv:CLISchema"));
            ld.insert("@id".to_string(), json!("iv://cli/introspect"));
            if let Value::Object(map) = schema {
                for (k, v) in map {
                    ld.insert(format!("iv:{k}"), v);
                }
            }
            let output = serde_json::to_string_pretty(&Value::Object(ld))
                .map_err(|e| ironvault::VaultError::SerializationError(e.to_string()))?;
            println!("{output}");
        }
        other => {
            return Err(ironvault::VaultError::InvalidInput(format!(
                "Unknown format {other:?}. Supported: json, yaml, jsonld"
            )))
        }
    }

    Ok(())
}
