//! CLI handler for model validation (iv validate).

use ironvault::{Result, ValidationStore, VaultConfig, VaultError};

pub fn handle_validate(
    name: String,
    version: Option<u32>,
    config: VaultConfig,
    _use_sqlite: bool,
) -> Result<()> {
    let store = ValidationStore::new(&config.dirs.vault_dir)?;

    // Resolve file path from version control
    let vc = ironvault::version::VersionControl::new(&config.dirs.vault_dir)?;
    let versions = vc.list_versions(&name);
    let ver = version.unwrap_or(0);
    let target = if ver == 0 {
        versions.last()
    } else {
        versions.iter().find(|v| v.version == ver)
    };

    let Some(v) = target else {
        return Err(match version {
            Some(requested) => VaultError::VersionNotFound(requested, name),
            None => VaultError::ModelNotFound(name),
        });
    };

    let file_path = std::path::PathBuf::from(&v.file_path);
    let report = store.validate(&name, &file_path)?;
    println!("Validation for '{}' (v{}):", name, v.version);
    let mut failures = Vec::new();
    for r in &report.results {
        let icon = if r.passed { "✓" } else { "✗" };
        println!("  {} {}: {}", icon, r.probe_label, r.message);
        if !r.passed {
            failures.push(r.probe_label.clone());
        }
    }

    if !report.overall_pass {
        // `iv validate` is an integrity gate. Printing "Some checks failed"
        // and exiting 0 meant every pipeline that ran it treated a failing
        // model as valid.
        println!("Some checks failed.");
        return Err(VaultError::IntegrityError(format!(
            "validation failed for '{}' v{}: {}",
            name,
            v.version,
            if failures.is_empty() {
                "see the report above".to_string()
            } else {
                failures.join(", ")
            }
        )));
    }

    println!("All checks passed.");
    Ok(())
}
