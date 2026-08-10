//! CLI handler for cross-model lineage graph (iv lineage-graph).

use ironvault::lineage_graph::{DerivationKind, LineageEdge};
use ironvault::{LineageGraph, Result, VaultConfig};

use crate::cli::args::LineageGraphCommands;

pub fn handle_lineage_graph(command: LineageGraphCommands, config: VaultConfig) -> Result<()> {
    let mut graph = LineageGraph::new(&config.dirs.vault_dir)?;

    match command {
        LineageGraphCommands::Add {
            parent,
            child,
            kind,
        } => {
            let k = match kind.to_lowercase().as_str() {
                "finetune" | "fine-tune" => DerivationKind::FineTune,
                "quantize" | "quantization" => DerivationKind::Quantization,
                "merge" => DerivationKind::Merge,
                "distill" | "distillation" => DerivationKind::Distillation,
                "convert" | "conversion" => DerivationKind::Conversion,
                "prune" => DerivationKind::Prune,
                _ => DerivationKind::Custom(kind),
            };
            let edge = LineageEdge {
                parents: vec![parent.clone()],
                child: child.clone(),
                kind: k,
                notes: std::collections::BTreeMap::new(),
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            graph.add_edge(edge)?;
            println!("Added lineage: {} -> {}", parent, child);
        }
        LineageGraphCommands::Show { model, format } => match format.as_str() {
            "json" => {
                let ancestors = graph.ancestors(&model);
                let descendants = graph.descendants(&model);
                let data = serde_json::json!({
                    "model": model,
                    "ancestors": ancestors,
                    "descendants": descendants,
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".to_string())
                );
            }
            _ => {
                println!("{}", graph.display());
            }
        },
    }

    Ok(())
}
