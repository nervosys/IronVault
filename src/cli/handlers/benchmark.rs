//! CLI handler for benchmark metadata (iv benchmark).

use ironvault::{BenchmarkStore, Result, VaultConfig, VaultError};

use crate::cli::args::BenchmarkCommands;

pub fn handle_benchmark(command: BenchmarkCommands, config: VaultConfig) -> Result<()> {
    let bench_dir = config.dirs.data_dir.join("benchmarks");
    let store = BenchmarkStore::new(&bench_dir)?;

    match command {
        BenchmarkCommands::Add {
            name,
            version,
            benchmark,
            score,
            unit,
            higher_is_better,
            hardware,
            dataset,
        } => {
            let mut record = store.get_or_create(&name, version as u64)?;

            if let Some(hw) = hardware {
                record.hardware = Some(hw);
            }

            let dataset_str = dataset;
            let mut metadata = std::collections::BTreeMap::new();
            if let Some(ref ds) = dataset_str {
                metadata.insert("dataset".to_string(), ds.clone());
            }

            record.add_detailed_result(
                &benchmark,
                score,
                &unit,
                higher_is_better,
                dataset_str.as_deref(),
                metadata,
            );

            store.save(&record)?;
            println!(
                "Recorded {} = {} {} for {} v{}",
                benchmark, score, unit, name, version
            );
        }
        BenchmarkCommands::Show {
            name,
            version,
            format,
        } => {
            if let Some(v) = version {
                let record = store.get_or_create(&name, v as u64)?;
                match format.as_str() {
                    "json" => {
                        let json = serde_json::to_string_pretty(&record)
                            .map_err(|e| VaultError::SerializationError(e.to_string()))?;
                        println!("{}", json);
                    }
                    _ => {
                        println!("{}", record.display());
                    }
                }
            } else {
                let records = store.list_for_model(&name)?;
                if records.is_empty() {
                    println!("No benchmark records found for '{}'", name);
                } else {
                    for record in &records {
                        match format.as_str() {
                            "json" => {
                                let json = serde_json::to_string_pretty(record)
                                    .map_err(|e| VaultError::SerializationError(e.to_string()))?;
                                println!("{}", json);
                            }
                            _ => {
                                println!("{}", record.display());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
