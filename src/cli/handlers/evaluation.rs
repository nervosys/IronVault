//! CLI handler for model evaluations (iv eval).

use ironvault::evaluation::{EvalRun, EvalStore, MetricResult};
use ironvault::{Result, VaultConfig, VaultError};

use crate::cli::args::EvalCommands;

pub fn handle_eval(command: EvalCommands, config: VaultConfig) -> Result<()> {
    let mut store = EvalStore::new(&config.dirs.vault_dir)?;

    match command {
        EvalCommands::Record {
            name,
            version,
            suite,
            metric,
            unit,
            higher_is_better,
        } => {
            let metrics: Vec<MetricResult> = metric
                .iter()
                .map(|m| {
                    let (mname, mval) = m.split_once('=').ok_or_else(|| {
                        VaultError::InvalidInput(format!(
                            "Metric must be in name=value format, got: {m}"
                        ))
                    })?;
                    let value: f64 = mval.parse().map_err(|_| {
                        VaultError::InvalidInput(format!("Invalid metric value: {mval}"))
                    })?;
                    Ok(MetricResult {
                        name: mname.to_string(),
                        value,
                        unit: unit.clone(),
                        higher_is_better,
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            let run = EvalRun {
                suite: suite.clone(),
                model: name.clone(),
                version,
                metrics,
                timestamp: chrono::Utc::now().to_rfc3339(),
                context: std::collections::BTreeMap::new(),
            };
            store.record(run)?;
            println!(
                "Recorded evaluation for {} v{} on suite '{}'",
                name, version, suite
            );
        }
        EvalCommands::List {
            name,
            version,
            format,
        } => {
            let runs = store.get_runs(&name, version);
            if runs.is_empty() {
                println!("No evaluation runs for '{}'", name);
            } else {
                match format.as_str() {
                    "json" => {
                        let json = serde_json::to_string_pretty(&runs)
                            .map_err(|e| VaultError::SerializationError(e.to_string()))?;
                        println!("{}", json);
                    }
                    _ => {
                        for run in &runs {
                            println!(
                                "  [{}] {} v{} — {}",
                                run.timestamp, run.model, run.version, run.suite
                            );
                            for m in &run.metrics {
                                let arrow = if m.higher_is_better { "↑" } else { "↓" };
                                println!("    {} = {} {} {}", m.name, m.value, m.unit, arrow);
                            }
                        }
                    }
                }
            }
        }
        EvalCommands::Compare {
            a,
            b,
            suite,
            format,
        } => {
            let (model_a, version_a) = parse_model_version(&a)?;
            let (model_b, version_b) = parse_model_version(&b)?;

            match store.compare(&model_a, version_a, &model_b, version_b, &suite) {
                Some(cmp) => {
                    if format.as_str() == "json" {
                        let json = serde_json::to_string_pretty(&cmp)
                            .map_err(|e| VaultError::SerializationError(e.to_string()))?;
                        println!("{}", json);
                    } else {
                        println!(
                            "Comparing {} v{} vs {} v{} on '{}':",
                            cmp.model_a, cmp.version_a, cmp.model_b, cmp.version_b, cmp.suite
                        );
                        for d in &cmp.deltas {
                            let symbol = if d.improved { "✓" } else { "✗" };
                            println!(
                                "  {} {} : {:.4} → {:.4} (Δ {:.4}) {}",
                                symbol, d.metric, d.value_a, d.value_b, d.delta, symbol
                            );
                        }
                    }
                }
                None => {
                    // `eval compare` is a regression gate. Printing a line and
                    // exiting 0 with no comparison output told a CI job that
                    // nothing regressed, when in fact nothing was compared.
                    return Err(VaultError::NotFound(format!(
                        "no evaluation runs on suite '{suite}' for both \
                         {model_a} v{version_a} and {model_b} v{version_b} \
                         — nothing was compared"
                    )));
                }
            }
        }
        EvalCommands::Suites => {
            let suites = store.suites();
            if suites.is_empty() {
                println!("No evaluation suites recorded.");
            } else {
                for s in &suites {
                    println!("  {}", s);
                }
            }
        }
    }

    Ok(())
}

fn parse_model_version(s: &str) -> Result<(String, u64)> {
    let (name, ver) = s
        .rsplit_once('@')
        .ok_or_else(|| VaultError::InvalidInput(format!("Expected name@version, got: {s}")))?;
    let version: u64 = ver
        .parse()
        .map_err(|_| VaultError::InvalidInput(format!("Invalid version number: {ver}")))?;
    Ok((name.to_string(), version))
}
