//! Analyze, deduplicate, and export command handlers.

use ironvault::formats::{ModelFormat, ModelMetadata};
use ironvault::utils::{CompressionAnalyzer, ModelAnalyzer, ModelDeduplicator, ModelExporter};
use ironvault::{Result, VaultConfig, VaultError};

use crate::cli::helpers::{build_vault, prompt_passphrase};

pub fn handle_analyze(
    name: String,
    version: Option<u32>,
    config: VaultConfig,
    use_sqlite: bool,
) -> Result<()> {
    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;

    let mut vault = build_vault(config, use_sqlite)?;
    vault.unlock(passphrase)?;

    // Get model data
    let data = vault.get_model(&name, version)?;

    // Get version info for compression stats
    let versions = vault.list_versions(&name);
    let version_info = if let Some(v) = version {
        versions.iter().find(|vi| vi.version == v)
    } else {
        versions.last()
    };

    if let Some(vi) = version_info {
        println!("Compression Analysis for '{}' v{}:", name, vi.version);
        println!("  Original size: {} bytes", vi.size_bytes);
        println!("  Compressed size: {} bytes", vi.compressed_size_bytes);

        let ratio = CompressionAnalyzer::compression_ratio(vi.size_bytes, vi.compressed_size_bytes);
        println!("  Compression ratio: {:.2}x", ratio);

        // Try to parse format
        let model_format = ModelFormat::from_extension(&vi.format);
        let report = CompressionAnalyzer::analyze_compression(
            vi.size_bytes,
            vi.compressed_size_bytes,
            &model_format,
        );
        println!("  Space saved: {:.2}%", report.space_saved_percent);
        println!("  Efficiency: {:.2}x expected", report.efficiency);

        // Model analysis
        let metadata = ModelMetadata::new(name.clone(), model_format);
        let analysis = ModelAnalyzer::analyze(&data, &metadata);

        println!("\nModel Analysis:");
        println!(
            "  Size: {}",
            ModelAnalyzer::format_size(analysis.size_bytes)
        );
        println!("  Format: {}", analysis.format);
        if let Some(params) = analysis.estimated_parameters {
            println!(
                "  Parameters: ~{}",
                ModelAnalyzer::format_parameters(params)
            );
        }
        if let Some(fw) = analysis.framework {
            println!("  Framework: {}", fw);
        }
        if let Some(task) = analysis.task {
            println!("  Task: {}", task);
        }
    } else {
        return Err(match version {
            Some(v) => VaultError::VersionNotFound(v, name),
            None => VaultError::ModelNotFound(name),
        });
    }
    Ok(())
}

pub fn handle_deduplicate(detailed: bool, config: VaultConfig, use_sqlite: bool) -> Result<()> {
    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;

    let mut vault = build_vault(config, use_sqlite)?;
    vault.unlock(passphrase)?;

    println!("Scanning for duplicate models...");

    let model_names = vault.list_models();
    let mut all_models = Vec::new();

    for name in &model_names {
        let data = vault.get_model(name, None)?;
        all_models.push((name.clone(), data));
    }

    // Create a copy for hash calculation
    let models_for_dedup = all_models
        .iter()
        .map(|(n, d)| (n.clone(), d.clone()))
        .collect();
    let duplicates = ModelDeduplicator::find_duplicates(models_for_dedup);

    if duplicates.is_empty() {
        println!("✓ No duplicate models found");
    } else {
        println!("\nFound {} duplicate groups:", duplicates.len());
        for (i, (_hash, names)) in duplicates.iter().enumerate() {
            println!("\nGroup {} ({} models):", i + 1, names.len());
            for n in names {
                println!("  - {}", n);
            }

            if detailed && names.len() == 2 {
                let data1 = all_models
                    .iter()
                    .find(|(n, _)| n == &names[0])
                    .map(|(_, d)| d.as_slice());
                let data2 = all_models
                    .iter()
                    .find(|(n, _)| n == &names[1])
                    .map(|(_, d)| d.as_slice());

                if let (Some(d1), Some(d2)) = (data1, data2) {
                    let similarity = ModelDeduplicator::similarity_score(d1, d2);
                    println!("    Similarity: {:.2}%", similarity * 100.0);
                }
            }
        }

        println!("\nYou can save space by removing duplicates.");
    }
    Ok(())
}

pub fn handle_export(
    name: String,
    output: std::path::PathBuf,
    version: Option<u32>,
    config: VaultConfig,
    use_sqlite: bool,
) -> Result<()> {
    let passphrase = prompt_passphrase("Enter vault passphrase: ")?;

    let mut vault = build_vault(config, use_sqlite)?;
    vault.unlock(passphrase)?;

    let data = vault.get_model(&name, version)?;

    // Get metadata
    let versions = vault.list_versions(&name);
    let version_info = if let Some(v) = version {
        versions.iter().find(|vi| vi.version == v)
    } else {
        versions.last()
    };

    if let Some(vi) = version_info {
        let model_format = ModelFormat::from_extension(&vi.format);
        let metadata = ModelMetadata::new(name.clone(), model_format.clone());

        std::fs::create_dir_all(&output)?;

        let _path = ModelExporter::export_with_metadata(data, &metadata, &output)?;

        println!("✓ Exported '{}' v{} to {:?}", name, vi.version, output);
        println!("  Model file: {}.{}", name, model_format.extension());
        println!("  Metadata: {}.meta.json", name);
    } else {
        return Err(match version {
            Some(v) => VaultError::VersionNotFound(v, name),
            None => VaultError::ModelNotFound(name),
        });
    }
    Ok(())
}
