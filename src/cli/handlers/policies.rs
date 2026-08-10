//! CLI handler for snapshot/retention policies (iv policy).

use ironvault::{PolicyStore, Result, VaultConfig};

use crate::cli::args::PolicyCommands;

pub fn handle_policy(
    command: PolicyCommands,
    config: VaultConfig,
    _use_sqlite: bool,
) -> Result<()> {
    let mut store = PolicyStore::new(&config.dirs.vault_dir)?;

    match command {
        PolicyCommands::Set {
            model,
            max_versions,
            max_age_days,
            keep_minimum,
        } => {
            use ironvault::policies::RetentionPolicy;
            let policy = RetentionPolicy {
                max_versions: max_versions.unwrap_or(usize::MAX),
                max_age_days: max_age_days.unwrap_or(u32::MAX as u64) as u32,
                keep_minimum: keep_minimum.unwrap_or(1),
            };
            store.set(&model, policy)?;
            println!("Policy set for '{}'", model);
        }
        PolicyCommands::Remove { model } => {
            store.remove(&model)?;
            println!("Policy removed for '{}'", model);
        }
        PolicyCommands::Show { model } => {
            if let Some(policy) = store.get(&model) {
                println!("Policy for '{}':", model);
                if policy.max_versions < usize::MAX {
                    println!("  Max versions: {}", policy.max_versions);
                }
                if (policy.max_age_days as u64) < u32::MAX as u64 {
                    println!("  Max age (days): {}", policy.max_age_days);
                }
                println!("  Keep minimum: {}", policy.keep_minimum);
            } else {
                println!("No policy set for '{}'", model);
            }
        }
        PolicyCommands::Apply { model, dry_run } => {
            let mut vc = ironvault::version::VersionControl::new(&config.dirs.vault_dir)?;
            if let Some(m) = model {
                let report = store.apply(&m, &mut vc, dry_run)?;
                if dry_run {
                    println!("Dry-run — no versions deleted.");
                }
                println!(
                    "'{}': {} -> {} versions ({} removed)",
                    report.model,
                    report.versions_before,
                    report.versions_after,
                    report.versions_removed.len()
                );
            } else {
                let reports = store.apply_all(&mut vc, dry_run)?;
                if dry_run {
                    println!("Dry-run — no versions deleted.");
                }
                for report in &reports {
                    println!(
                        "'{}': {} -> {} versions ({} removed)",
                        report.model,
                        report.versions_before,
                        report.versions_after,
                        report.versions_removed.len()
                    );
                }
            }
        }
    }

    Ok(())
}
