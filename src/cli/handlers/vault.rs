//! Vault core command handlers (init, store, get, list, versions, lineage, delete, stats, compliance, change-passphrase, cache).

use ironvault::compliance::ComplianceChecker;
use ironvault::formats::{ModelFormat, ModelMetadata};
use ironvault::{Result, VaultConfig, VaultError};
use std::io::{self, Write};

use crate::cli::helpers::{build_vault, prompt_passphrase};

pub fn handle_init(name: String, config: VaultConfig, use_sqlite: bool) -> Result<()> {
    println!("Initializing vault: {}", name);
    let vault = build_vault(config, use_sqlite)?;
    println!("✓ Vault '{}' initialized successfully", name);
    println!("  Backend: {}", vault.version_backend_name());
    println!("Location: {:?}", vault.get_config().dirs.vault_dir);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn handle_store(
    name: String,
    path: std::path::PathBuf,
    format: Option<String>,
    description: Option<String>,
    framework: Option<String>,
    task: Option<String>,
    config: VaultConfig,
    use_sqlite: bool,
) -> Result<()> {
    // Read model file
    let data = std::fs::read(&path)?;
    println!("Read {} bytes from {:?}", data.len(), path);

    // Detect format
    let model_format = if let Some(fmt) = format {
        match fmt.to_lowercase().as_str() {
            // LLM formats
            "safetensors" => ModelFormat::Safetensors,
            "gguf" => ModelFormat::GGUF,
            "pytorch" | "pt" | "torch" => ModelFormat::PyTorch,
            "tensorrt" | "trt" => ModelFormat::TensorRT,
            "onnx" => ModelFormat::ONNX,
            "mlx" => ModelFormat::MLX,
            "coreml" | "mlmodel" => ModelFormat::CoreML,
            "torchscript" => ModelFormat::TorchScript,
            "tflite" | "tensorflow-lite" => ModelFormat::TFLite,
            // General DL formats
            "tensorflow" | "tf" | "savedmodel" => ModelFormat::TensorFlow,
            "keras" | "h5" => ModelFormat::Keras,
            "openvino" => ModelFormat::OpenVINO,
            "tvm" => ModelFormat::TVM,
            "ncnn" => ModelFormat::NCNN,
            "mnn" => ModelFormat::MNN,
            "rknn" => ModelFormat::RKNN,
            // Legacy formats
            "caffe" => ModelFormat::Caffe,
            "mxnet" => ModelFormat::MXNet,
            "darknet" => ModelFormat::Darknet,
            // Data formats
            "hdf5" => ModelFormat::HDF5,
            "pickle" | "pkl" => ModelFormat::Pickle,
            "numpy" | "npy" => ModelFormat::NumPy,
            _ => ModelFormat::Custom(fmt),
        }
    } else {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("bin");
        ModelFormat::from_extension(ext)
    };

    // Captured before `model_format` is moved into the metadata below.
    // `telemetry_name` rather than `name`: the latter returns the caller's own
    // string for `ModelFormat::Custom`.
    let format_label = model_format.telemetry_name();

    // Create metadata
    let mut metadata = ModelMetadata::new(name.clone(), model_format);
    if let Some(desc) = description {
        metadata = metadata.with_description(desc);
    }
    if let Some(fw) = framework {
        metadata = metadata.with_framework(fw);
    }
    if let Some(t) = task {
        metadata = metadata.with_task(t);
    }

    // Get passphrase
    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;

    // Store model
    let mut vault = build_vault(config, use_sqlite)?;
    vault.unlock(passphrase)?;

    // Size is bucketed inside `track_model_op`, never reported exactly.
    let size = data.len() as u64;
    let started = std::time::Instant::now();
    let stored = vault.store_model(&name, data, metadata, None);
    ironvault::telemetry::track_model_op(
        "store",
        format_label,
        size,
        started.elapsed(),
        stored.is_ok(),
    );
    let version = stored?;

    println!("✓ Model '{}' stored successfully", name);
    println!("  Version: {}", version.version);
    println!("  Checkpoint ID: {}", version.checkpoint_id);
    println!("  Original size: {} bytes", version.size_bytes);
    println!("  Compressed size: {} bytes", version.compressed_size_bytes);
    println!(
        "  Compression ratio: {:.1}%",
        (1.0 - version.compressed_size_bytes as f64 / version.size_bytes as f64) * 100.0
    );
    Ok(())
}

pub fn handle_get(
    name: String,
    output: std::path::PathBuf,
    version: Option<u32>,
    config: VaultConfig,
    use_sqlite: bool,
) -> Result<()> {
    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;

    let mut vault = build_vault(config, use_sqlite)?;
    vault.unlock(passphrase)?;

    // Format is read from the stored version rather than guessed from the
    // output path's extension, which the user chose and which may be anything.
    let format_label = vault
        .list_versions(&name)
        .iter()
        .find(|v| version.is_none_or(|want| v.version == want))
        .map_or("unknown", |v| {
            ModelFormat::from_stored(&v.format).telemetry_name()
        });

    let started = std::time::Instant::now();
    let fetched = vault.get_model(&name, version);
    ironvault::telemetry::track_model_op(
        "get",
        format_label,
        fetched.as_ref().map_or(0, |d| d.len() as u64),
        started.elapsed(),
        fetched.is_ok(),
    );
    let data = fetched?;
    std::fs::write(&output, &data)?;

    println!("✓ Model '{}' retrieved successfully", name);
    println!("  Written to: {:?}", output);
    println!("  Size: {} bytes", data.len());
    Ok(())
}

pub fn handle_list(config: VaultConfig, use_sqlite: bool, format: &str) -> Result<()> {
    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;

    let mut vault = build_vault(config, use_sqlite)?;
    vault.unlock(passphrase)?;

    let models = vault.list_models();

    if format == "json" {
        let payload: Vec<serde_json::Value> = models
            .iter()
            .map(|model| {
                let versions = vault.list_versions(model);
                serde_json::json!({
                    "name": model,
                    "versions": versions.len(),
                    "latest_version": versions.iter().map(|v| v.version).max(),
                })
            })
            .collect();
        // An empty vault serialises as an empty array rather than an absent
        // key, so a caller indexing the result never has to special-case it.
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "models": payload }))?
        );
        return Ok(());
    }

    if models.is_empty() {
        println!("No models in vault");
    } else {
        println!("Models in vault:");
        for model in models {
            let versions = vault.list_versions(&model);
            println!("  {} ({} versions)", model, versions.len());
        }
    }
    Ok(())
}

pub fn handle_versions(
    name: String,
    config: VaultConfig,
    use_sqlite: bool,
    format: &str,
) -> Result<()> {
    // 7.0 requires the passphrase here. Version metadata lives in
    // `versions.json`, which is not encrypted, so this command used to answer
    // with no passphrase at all -- model names, sizes, formats and timestamps
    // to anyone who could run the binary. Unlocking does not encrypt the file
    // (see SECURITY.md), but the tool no longer hands out the inventory to a
    // caller who cannot open the vault.
    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
    let mut vault = build_vault(config, use_sqlite)?;
    vault.unlock(passphrase)?;

    let versions = vault.list_versions(&name);

    if versions.is_empty() {
        // A model with no versions is not in this vault. Printing and exiting
        // 0 told every script that the lookup succeeded.
        return Err(VaultError::ModelNotFound(name));
    }

    if format == "json" {
        let payload: Vec<serde_json::Value> = versions
            .iter()
            .map(|v| {
                serde_json::json!({
                    "version": v.version,
                    "timestamp": v.timestamp.to_rfc3339(),
                    "size_bytes": v.size_bytes,
                    "format": v.format.to_string(),
                    "checkpoint_id": v.checkpoint_id,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({ "model": name, "versions": payload })
            )?
        );
        return Ok(());
    }

    println!("Versions of '{}':", name);
    for v in versions {
        println!(
            "  v{} - {} - {} bytes ({})",
            v.version,
            v.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            v.size_bytes,
            v.format
        );
    }
    Ok(())
}

pub fn handle_lineage(
    name: String,
    version: u32,
    config: VaultConfig,
    use_sqlite: bool,
    format: &str,
) -> Result<()> {
    // Requires the passphrase from 7.0 on -- same reasoning as `handle_versions`.
    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
    let mut vault = build_vault(config, use_sqlite)?;
    vault.unlock(passphrase)?;

    let lineage = vault.get_lineage(&name, version);

    if lineage.is_empty() {
        return Err(VaultError::VersionNotFound(version, name));
    }

    if format == "json" {
        let payload: Vec<serde_json::Value> = lineage
            .iter()
            .map(|v| {
                serde_json::json!({
                    "version": v.version,
                    "timestamp": v.timestamp.to_rfc3339(),
                    "checkpoint_id": v.checkpoint_id,
                    "parent_version": v.parent_version,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "model": name,
                "version": version,
                "lineage": payload,
            }))?
        );
        return Ok(());
    }

    println!("Lineage for '{}' v{}:", name, version);
    for (i, v) in lineage.iter().enumerate() {
        println!(
            "  {}v{} - {} - {}",
            "  ".repeat(i),
            v.version,
            v.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            v.checkpoint_id
        );
    }
    Ok(())
}

pub fn handle_delete(
    name: String,
    version: u32,
    force: bool,
    config: VaultConfig,
    use_sqlite: bool,
) -> Result<()> {
    if !force {
        print!(
            "Are you sure you want to delete '{}' v{}? [y/N]: ",
            name, version
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;

    let mut vault = build_vault(config, use_sqlite)?;
    vault.unlock(passphrase)?;

    let format_label = vault
        .list_versions(&name)
        .iter()
        .find(|v| v.version == version)
        .map_or("unknown", |v| {
            ModelFormat::from_stored(&v.format).telemetry_name()
        });

    let started = std::time::Instant::now();
    let deleted = vault.delete_version(&name, version);
    ironvault::telemetry::track_model_op(
        "delete",
        format_label,
        0,
        started.elapsed(),
        deleted.is_ok(),
    );

    if deleted? {
        println!("✓ Deleted '{}' v{}", name, version);
    } else {
        println!("Version not found");
    }
    Ok(())
}

pub fn handle_stats(config: VaultConfig, use_sqlite: bool, format: &str) -> Result<()> {
    // Requires the passphrase from 7.0 on -- same reasoning as `handle_versions`.
    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
    let mut vault = build_vault(config, use_sqlite)?;
    vault.unlock(passphrase)?;

    let stats = vault.get_stats()?;

    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "model_count": stats.model_count,
                "total_versions": stats.total_versions,
                "total_size_bytes": stats.total_size_bytes,
                "file_count": stats.file_count,
            }))?
        );
        return Ok(());
    }

    println!("Vault Statistics:");
    println!("  Models: {}", stats.model_count);
    println!("  Total versions: {}", stats.total_versions);
    println!(
        "  Total size: {} bytes ({:.2} MB)",
        stats.total_size_bytes,
        stats.total_size_bytes as f64 / 1_048_576.0
    );
    println!("  Files: {}", stats.file_count);
    Ok(())
}

/// Caveat attached to every compliance report, text or JSON.
const NOTE_TEXT: &str = "Only checks reported as VERIFIED were tested by this run. This is not a certification: FIPS 140-3 validation is issued by NIST's CMVP for a cryptographic module, and CMMC certification by a C3PAO assessment of an organisation.";

pub fn handle_compliance(format: &str) -> Result<()> {
    let checker = ComplianceChecker::new();
    let status = checker.run_all_checks()?;

    if format == "json" {
        let outcomes: Vec<serde_json::Value> = status
            .outcomes
            .iter()
            .map(|(name, outcome)| {
                serde_json::json!({
                    "check": name,
                    "result": outcome.label(),
                    "detail": outcome.detail(),
                })
            })
            .collect();
        let violations: Vec<serde_json::Value> = status
            .violations
            .iter()
            .map(|v| {
                serde_json::json!({
                    "severity": format!("{:?}", v.severity),
                    "standard": v.standard,
                    "control": v.control,
                    "description": v.description,
                })
            })
            .collect();
        // Carries the same caveat as the text output. A machine-readable
        // compliance report without it invites being pasted into evidence.
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "checks": outcomes,
                "violations": violations,
                "note": NOTE_TEXT,
            }))?
        );
        return Ok(());
    }

    println!("Running compliance checks...\n");
    println!("Compliance Status:");
    for (name, outcome) in &status.outcomes {
        println!("  {name}: {}", outcome.label());
        println!("      {}", outcome.detail());
    }

    if !status.violations.is_empty() {
        println!("\nViolations:");
        for violation in &status.violations {
            println!(
                "  [{:?}] {} - {}: {}",
                violation.severity, violation.standard, violation.control, violation.description
            );
        }
    }

    println!(
        "\nNote: only checks marked VERIFIED were tested by this run. \
         BY DESIGN entries describe how the software is built and are not \
         evidence of certification — FIPS 140-3 validation is issued by NIST's \
         CMVP for a cryptographic module, and CMMC certification by a C3PAO \
         assessment of an organisation. Neither is something this tool can grant."
    );

    let blocking: Vec<&String> = status
        .outcomes
        .iter()
        .filter(|(_, o)| o.is_blocking())
        .map(|(n, _)| n)
        .collect();

    if !blocking.is_empty() {
        return Err(VaultError::ComplianceViolation(format!(
            "compliance checks failed: {}",
            blocking
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    Ok(())
}

pub fn handle_change_passphrase(config: VaultConfig, use_sqlite: bool) -> Result<()> {
    let old_passphrase = prompt_passphrase("Enter current vault passphrase: ")?;
    let new_passphrase = prompt_passphrase("Enter new vault passphrase: ")?;
    let confirm_passphrase = prompt_passphrase("Confirm new vault passphrase: ")?;

    if new_passphrase != confirm_passphrase {
        return Err(VaultError::InvalidInput(
            "Passphrases do not match".to_string(),
        ));
    }

    let mut vault = build_vault(config, use_sqlite)?;
    vault.unlock(old_passphrase)?;
    let count = vault.change_passphrase(new_passphrase)?;

    println!("✓ Passphrase changed successfully");
    println!("  Re-encrypted {} model file(s)", count);
    Ok(())
}

pub fn handle_cache() -> Result<()> {
    use ironvault::utils::RetrievalOptimizer;
    let cache = RetrievalOptimizer::new(1024 * 1024 * 1024); // 1 GB
    let stats = cache.cache_stats();
    println!("Cache Statistics:");
    println!("  Capacity: {:.2} MB", stats.max_size as f64 / 1_048_576.0);
    println!("  Used: {:.2} MB", stats.total_size as f64 / 1_048_576.0);
    println!("  Entries: {}", stats.total_entries);
    println!("  Utilization: {:.1}%", stats.utilization);
    println!("\n💡 The cache is per-process. Use RetrievalOptimizer in your");
    println!("   application code for persistent caching.");
    Ok(())
}
