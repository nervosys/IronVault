//! Demonstration of IronVault utilities
//!
//! This example showcases the model utilities including:
//! - Model archiving (TAR/ZIP)
//! - LRU caching for performance
//! - Deduplication
//! - Model analysis
//! - Quantization metadata
//! - Pruning information

use ironvault::formats::{ModelFormat, ModelMetadata};
use ironvault::utils::{
    CompressionAnalyzer, ModelAnalyzer, ModelArchive, ModelDeduplicator, ModelExporter,
    PruningInfo, PruningMethod, QuantizationInfo, RetrievalOptimizer,
};
use tempfile::tempdir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== IronVault Utilities Demo ===\n");

    // Demo data - simulating model files
    let model_data_1 = vec![0u8; 1_000_000]; // 1 MB
    let model_data_2 = vec![1u8; 5_000_000]; // 5 MB
    let model_data_3 = vec![0u8; 1_000_000]; // 1 MB (duplicate of model_data_1)

    demo_compression_analysis()?;
    demo_quantization_info()?;
    demo_pruning_info()?;
    demo_model_analysis(&model_data_2)?;
    demo_caching(&model_data_1, &model_data_2)?;
    demo_archiving()?;
    demo_deduplication(model_data_1, model_data_2, model_data_3)?;
    demo_export()?;

    println!("\n=== Demo Complete! ===");
    Ok(())
}

/// Demonstrate compression analysis
fn demo_compression_analysis() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Compression Analysis");
    println!("─────────────────────────");

    let original = 10_000_000; // 10 MB
    let compressed = 7_500_000; // 7.5 MB

    let ratio = CompressionAnalyzer::compression_ratio(original, compressed);
    println!("Compression ratio: {:.2}x", ratio);

    let report =
        CompressionAnalyzer::analyze_compression(original, compressed, &ModelFormat::PyTorch);

    println!("Original size: {} bytes", report.original_size);
    println!("Compressed size: {} bytes", report.compressed_size);
    println!(
        "Space saved: {} bytes ({:.1}%)",
        report.space_saved, report.space_saved_percent
    );
    println!("Efficiency: {:.2}x of expected", report.efficiency);

    // Estimate for different formats
    println!("\nEstimated compression ratios by format:");
    for format in &[
        ModelFormat::Safetensors,
        ModelFormat::GGUF,
        ModelFormat::PyTorch,
        ModelFormat::HDF5,
    ] {
        let ratio = CompressionAnalyzer::estimate_ratio(format);
        println!("  {}: {:.2}x", format.name(), ratio);
    }

    println!();
    Ok(())
}

/// Demonstrate quantization information
fn demo_quantization_info() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔢 Quantization Analysis");
    println!("─────────────────────────");

    // Show available quantization schemes
    println!("Supported quantization schemes:");
    for scheme in QuantizationInfo::schemes() {
        println!("  • {}", scheme);
    }

    // Estimate size reduction
    let fp32_size = 4_000_000_000; // 4 GB in FP32
    let int8_size = QuantizationInfo::estimate_size(fp32_size, 32, 8);
    let int4_size = QuantizationInfo::estimate_size(fp32_size, 32, 4);

    println!("\nSize estimates for 4GB FP32 model:");
    println!(
        "  FP32 → INT8: {} ({:.2} GB)",
        ModelAnalyzer::format_size(int8_size),
        int8_size as f64 / 1_000_000_000.0
    );
    println!(
        "  FP32 → INT4: {} ({:.2} GB)",
        ModelAnalyzer::format_size(int4_size),
        int4_size as f64 / 1_000_000_000.0
    );

    // Calculate savings
    let savings = QuantizationInfo::memory_savings(fp32_size, int8_size);
    println!("\nFP32 → INT8 savings:");
    println!(
        "  Space saved: {}",
        ModelAnalyzer::format_size(savings.saved_bytes)
    );
    println!("  Percentage: {:.1}%", savings.saved_percent);
    println!("  Size ratio: {:.1}x smaller", savings.size_ratio);

    println!();
    Ok(())
}

/// Demonstrate pruning information
fn demo_pruning_info() -> Result<(), Box<dyn std::error::Error>> {
    println!("✂️  Pruning Analysis");
    println!("─────────────────────────");

    let pruning = PruningInfo::new(
        PruningMethod::Magnitude,
        0.5,           // 50% sparsity target
        7_000_000_000, // 7B parameters originally
        3_500_000_000, // 3.5B parameters remaining
    );

    println!("Pruning method: {:?}", pruning.pruning_method);
    println!("Target sparsity: {:.1}%", pruning.sparsity_level * 100.0);
    println!(
        "Original parameters: {}",
        ModelAnalyzer::format_parameters(pruning.original_params)
    );
    println!(
        "Remaining parameters: {}",
        ModelAnalyzer::format_parameters(pruning.remaining_params)
    );
    println!(
        "Actual sparsity: {:.1}%",
        pruning.calculate_sparsity() * 100.0
    );
    println!("Size reduction: {:.1}%", pruning.size_reduction());

    println!();
    Ok(())
}

/// Demonstrate model analysis
fn demo_model_analysis(data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    println!("📈 Model Analysis");
    println!("─────────────────────────");

    let metadata = ModelMetadata::new("demo-model".to_string(), ModelFormat::Safetensors)
        .with_framework("PyTorch 2.1".to_string())
        .with_task("text-generation".to_string())
        .with_parameters(7_000_000_000);

    let analysis = ModelAnalyzer::analyze(data, &metadata);

    println!("Format: {}", analysis.format);
    println!("Size: {}", ModelAnalyzer::format_size(analysis.size_bytes));
    println!("Size (MB): {:.2}", analysis.size_mb);
    println!("Size (GB): {:.2}", analysis.size_gb);

    if let Some(params) = analysis.estimated_parameters {
        println!(
            "Estimated parameters: {}",
            ModelAnalyzer::format_parameters(params)
        );
    }

    if let Some(framework) = analysis.framework {
        println!("Framework: {}", framework);
    }

    if let Some(task) = analysis.task {
        println!("Task: {}", task);
    }

    println!();
    Ok(())
}

/// Demonstrate LRU caching
fn demo_caching(model1: &[u8], model2: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    println!("⚡ Retrieval Optimization (LRU Cache)");
    println!("─────────────────────────────────────");

    let cache_size = 10 * 1024 * 1024; // 10 MB cache
    let mut optimizer = RetrievalOptimizer::new(cache_size);

    println!(
        "Cache size limit: {}",
        ModelAnalyzer::format_size(cache_size as u64)
    );

    // Cache first model
    optimizer.cache_model("llama-7b".to_string(), model1.to_vec())?;
    println!("✓ Cached 'llama-7b' ({} bytes)", model1.len());

    // Cache second model
    optimizer.cache_model("gpt2".to_string(), model2.to_vec())?;
    println!("✓ Cached 'gpt2' ({} bytes)", model2.len());

    // Retrieve from cache
    if let Some(_data) = optimizer.get_cached("llama-7b") {
        println!("✓ Retrieved 'llama-7b' from cache (fast!)");
    }

    // Cache statistics
    let stats = optimizer.cache_stats();
    println!("\nCache statistics:");
    println!("  Entries: {}", stats.total_entries);
    println!(
        "  Total size: {}",
        ModelAnalyzer::format_size(stats.total_size as u64)
    );
    println!(
        "  Max size: {}",
        ModelAnalyzer::format_size(stats.max_size as u64)
    );
    println!("  Utilization: {:.1}%", stats.utilization);

    println!();
    Ok(())
}

/// Demonstrate archiving
fn demo_archiving() -> Result<(), Box<dyn std::error::Error>> {
    println!("📦 Model Archiving");
    println!("─────────────────────────");

    let temp_dir = tempdir()?;

    let models = vec![
        ("model1.safetensors".to_string(), vec![0u8; 100]),
        ("model2.gguf".to_string(), vec![1u8; 200]),
        ("model3.onnx".to_string(), vec![2u8; 150]),
    ];

    // Create TAR archive
    let tar_path = temp_dir.path().join("models.tar");
    let tar_size = ModelArchive::create_tar(models.clone(), &tar_path)?;
    println!("✓ Created TAR archive: {}", tar_path.display());
    println!("  Size: {} bytes", tar_size);

    // Create ZIP archive
    let zip_path = temp_dir.path().join("models.zip");
    let zip_size = ModelArchive::create_zip(models.clone(), &zip_path)?;
    println!("✓ Created ZIP archive: {}", zip_path.display());
    println!("  Size: {} bytes", zip_size);

    // Extract TAR
    let extracted = ModelArchive::extract_tar(&tar_path)?;
    println!("✓ Extracted {} models from TAR", extracted.len());

    // Extract ZIP
    let extracted = ModelArchive::extract_zip(&zip_path)?;
    println!("✓ Extracted {} models from ZIP", extracted.len());

    println!();
    Ok(())
}

/// Demonstrate deduplication
fn demo_deduplication(
    data1: Vec<u8>,
    data2: Vec<u8>,
    data3: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Deduplication");
    println!("─────────────────────────");

    let models = vec![
        ("model-v1".to_string(), data1.clone()),
        ("model-v2".to_string(), data2),
        ("model-v1-copy".to_string(), data3), // Duplicate of model-v1
    ];

    // Calculate hashes
    let hash1 = ModelDeduplicator::calculate_hash(&data1);
    println!("Hash of model-v1: {}...", &hash1[..16]);

    // Find duplicates
    let duplicates = ModelDeduplicator::find_duplicates(models);

    if duplicates.is_empty() {
        println!("No duplicates found");
    } else {
        println!("\nFound {} duplicate group(s):", duplicates.len());
        for (hash, names) in duplicates {
            println!("  Hash {}...:", &hash[..16]);
            for name in names {
                println!("    - {}", name);
            }
        }
    }

    println!();
    Ok(())
}

/// Demonstrate model export
fn demo_export() -> Result<(), Box<dyn std::error::Error>> {
    println!("📤 Model Export");
    println!("─────────────────────────");

    let temp_dir = tempdir()?;

    let model_data = vec![42u8; 1000];
    let metadata = ModelMetadata::new("exported-model".to_string(), ModelFormat::ONNX)
        .with_description("Demo model for export".to_string())
        .with_framework("PyTorch".to_string())
        .with_task("classification".to_string());

    // Export with metadata
    let path = ModelExporter::export_with_metadata(model_data.clone(), &metadata, temp_dir.path())?;

    println!("✓ Exported model: {}", path.display());

    let meta_path = temp_dir.path().join("exported-model.meta.json");
    if meta_path.exists() {
        println!("✓ Exported metadata: {}", meta_path.display());
        let meta_content = std::fs::read_to_string(&meta_path)?;
        println!("\nMetadata preview:");
        let lines: Vec<_> = meta_content.lines().take(5).collect();
        for line in lines {
            println!("  {}", line);
        }
    }

    println!();
    Ok(())
}
