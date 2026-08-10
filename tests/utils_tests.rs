// Assertions in these tests compare literal constants that round-trip
// bit-for-bit and build fixed strings; the lints below are noise here.
#![allow(clippy::float_cmp)]
//! Comprehensive tests for model utilities

use ironvault::formats::{ModelFormat, ModelMetadata};
use ironvault::utils::{
    CompressionAnalyzer, ModelAnalyzer, ModelArchive, ModelDeduplicator, ModelExporter,
    PruningInfo, PruningMethod, QuantizationInfo, RetrievalOptimizer,
};
use tempfile::tempdir;

// ===== Compression Analysis Tests =====

#[test]
fn test_compression_ratio_calculation() {
    let ratio = CompressionAnalyzer::compression_ratio(1000, 500);
    assert_eq!(ratio, 2.0);

    let ratio = CompressionAnalyzer::compression_ratio(2000, 500);
    assert_eq!(ratio, 4.0);

    let ratio = CompressionAnalyzer::compression_ratio(1000, 1000);
    assert_eq!(ratio, 1.0);
}

#[test]
fn test_compression_ratio_edge_cases() {
    let ratio = CompressionAnalyzer::compression_ratio(1000, 0);
    assert_eq!(ratio, 0.0);

    let ratio = CompressionAnalyzer::compression_ratio(0, 500);
    assert_eq!(ratio, 0.0);
}

#[test]
fn test_estimate_compression_ratios() {
    let ratio = CompressionAnalyzer::estimate_ratio(&ModelFormat::Safetensors);
    assert!(ratio > 1.0);

    let ratio = CompressionAnalyzer::estimate_ratio(&ModelFormat::GGUF);
    assert_eq!(ratio, 1.0); // Pre-compressed

    let ratio = CompressionAnalyzer::estimate_ratio(&ModelFormat::PyTorch);
    assert!(ratio >= 1.2);
}

#[test]
fn test_compression_analysis_report() {
    let report = CompressionAnalyzer::analyze_compression(10000, 7500, &ModelFormat::PyTorch);

    assert_eq!(report.original_size, 10000);
    assert_eq!(report.compressed_size, 7500);
    assert_eq!(report.space_saved, 2500);
    assert_eq!(report.compression_ratio, 10000.0 / 7500.0);
    assert_eq!(report.space_saved_percent, 25.0);
}

#[test]
fn test_compression_efficiency() {
    let report = CompressionAnalyzer::analyze_compression(1000, 500, &ModelFormat::HDF5);

    assert_eq!(report.compression_ratio, 2.0);
    assert!(report.efficiency > 0.0); // Should be positive
}

// ===== Quantization Tests =====

#[test]
fn test_quantization_schemes() {
    let schemes = QuantizationInfo::schemes();
    assert!(schemes.contains(&"FP32"));
    assert!(schemes.contains(&"FP16"));
    assert!(schemes.contains(&"INT8"));
    assert!(schemes.contains(&"INT4"));
    assert!(schemes.contains(&"Q8_0"));
}

#[test]
fn test_quantization_size_estimation() {
    // FP32 to FP16
    let size = QuantizationInfo::estimate_size(1000, 32, 16);
    assert_eq!(size, 500);

    // FP32 to INT8
    let size = QuantizationInfo::estimate_size(1000, 32, 8);
    assert_eq!(size, 250);

    // FP32 to INT4
    let size = QuantizationInfo::estimate_size(1000, 32, 4);
    assert_eq!(size, 125);
}

#[test]
fn test_quantization_savings() {
    let savings = QuantizationInfo::memory_savings(10000, 2500);

    assert_eq!(savings.original_size, 10000);
    assert_eq!(savings.quantized_size, 2500);
    assert_eq!(savings.saved_bytes, 7500);
    assert_eq!(savings.saved_percent, 75.0);
    assert_eq!(savings.size_ratio, 4.0);
}

#[test]
fn test_quantization_scheme_validation() {
    assert!(QuantizationInfo::is_valid_scheme("FP32"));
    assert!(QuantizationInfo::is_valid_scheme("INT8"));
    assert!(QuantizationInfo::is_valid_scheme("Q4_K_M"));
    assert!(!QuantizationInfo::is_valid_scheme("INVALID"));
}

// ===== Pruning Tests =====

#[test]
fn test_pruning_info_creation() {
    let info = PruningInfo::new(PruningMethod::Magnitude, 0.5, 1_000_000, 500_000);

    assert_eq!(info.pruning_method, PruningMethod::Magnitude);
    assert_eq!(info.sparsity_level, 0.5);
    assert_eq!(info.original_params, 1_000_000);
    assert_eq!(info.remaining_params, 500_000);
}

#[test]
fn test_pruning_sparsity_calculation() {
    let info = PruningInfo::new(PruningMethod::Structured, 0.0, 1000, 500);
    assert_eq!(info.calculate_sparsity(), 0.5);

    let info = PruningInfo::new(PruningMethod::Unstructured, 0.0, 1000, 250);
    assert_eq!(info.calculate_sparsity(), 0.75);

    let info = PruningInfo::new(PruningMethod::GradientBased, 0.0, 1000, 900);
    let sparsity = info.calculate_sparsity();
    assert!((sparsity - 0.1).abs() < 0.0001); // Float comparison with epsilon
}

#[test]
fn test_pruning_size_reduction() {
    let info = PruningInfo::new(PruningMethod::Magnitude, 0.6, 1000, 400);
    assert_eq!(info.size_reduction(), 60.0);
}

#[test]
fn test_pruning_methods() {
    let magnitude = PruningInfo::new(PruningMethod::Magnitude, 0.5, 1000, 500);
    assert_eq!(magnitude.pruning_method, PruningMethod::Magnitude);

    let custom = PruningInfo::new(
        PruningMethod::Custom("my_method".to_string()),
        0.3,
        1000,
        700,
    );
    assert_eq!(
        custom.pruning_method,
        PruningMethod::Custom("my_method".to_string())
    );
}

// ===== Retrieval Optimizer Tests =====

#[test]
fn test_retrieval_optimizer_creation() {
    let optimizer = RetrievalOptimizer::new(10000);
    let stats = optimizer.cache_stats();

    assert_eq!(stats.total_entries, 0);
    assert_eq!(stats.total_size, 0);
    assert_eq!(stats.max_size, 10000);
}

#[test]
fn test_cache_model() {
    let mut optimizer = RetrievalOptimizer::new(10000);

    optimizer
        .cache_model("model1".to_string(), vec![0; 1000])
        .unwrap();

    let stats = optimizer.cache_stats();
    assert_eq!(stats.total_entries, 1);
    assert_eq!(stats.total_size, 1000);
}

#[test]
fn test_retrieve_cached_model() {
    let mut optimizer = RetrievalOptimizer::new(10000);

    let data = vec![1, 2, 3, 4, 5];
    optimizer
        .cache_model("model1".to_string(), data.clone())
        .unwrap();

    let cached = optimizer.get_cached("model1");
    assert!(cached.is_some());
    assert_eq!(cached.unwrap(), data);
}

#[test]
fn test_cache_miss() {
    let mut optimizer = RetrievalOptimizer::new(10000);
    let cached = optimizer.get_cached("nonexistent");
    assert!(cached.is_none());
}

#[test]
fn test_cache_eviction() {
    let mut optimizer = RetrievalOptimizer::new(1500);

    // Cache first model
    optimizer
        .cache_model("model1".to_string(), vec![0; 1000])
        .unwrap();
    assert_eq!(optimizer.cache_stats().total_entries, 1);

    // Cache second model (should evict first)
    optimizer
        .cache_model("model2".to_string(), vec![0; 1000])
        .unwrap();

    // After some time, cache third model
    std::thread::sleep(std::time::Duration::from_millis(10));
    optimizer
        .cache_model("model3".to_string(), vec![0; 1000])
        .unwrap();

    // Should have evicted oldest
    assert!(optimizer.cache_stats().total_size <= 1500);
}

#[test]
fn test_clear_cache() {
    let mut optimizer = RetrievalOptimizer::new(10000);

    optimizer
        .cache_model("model1".to_string(), vec![0; 1000])
        .unwrap();
    optimizer
        .cache_model("model2".to_string(), vec![0; 1000])
        .unwrap();

    assert_eq!(optimizer.cache_stats().total_entries, 2);

    optimizer.clear_cache();

    let stats = optimizer.cache_stats();
    assert_eq!(stats.total_entries, 0);
    assert_eq!(stats.total_size, 0);
}

#[test]
fn test_cache_oversized_model() {
    let mut optimizer = RetrievalOptimizer::new(1000);

    // Try to cache model larger than max cache size
    optimizer
        .cache_model("huge_model".to_string(), vec![0; 2000])
        .unwrap();

    // Should not be cached
    let stats = optimizer.cache_stats();
    assert_eq!(stats.total_entries, 0);
}

// ===== Model Analyzer Tests =====

#[test]
fn test_format_size_bytes() {
    assert_eq!(ModelAnalyzer::format_size(100), "100 B");
    assert_eq!(ModelAnalyzer::format_size(999), "999 B");
}

#[test]
fn test_format_size_kilobytes() {
    assert_eq!(ModelAnalyzer::format_size(1024), "1.00 KB");
    assert_eq!(ModelAnalyzer::format_size(2048), "2.00 KB");
    assert_eq!(ModelAnalyzer::format_size(1536), "1.50 KB");
}

#[test]
fn test_format_size_megabytes() {
    assert_eq!(ModelAnalyzer::format_size(1024 * 1024), "1.00 MB");
    assert_eq!(ModelAnalyzer::format_size(5 * 1024 * 1024), "5.00 MB");
}

#[test]
fn test_format_size_gigabytes() {
    assert_eq!(ModelAnalyzer::format_size(1024 * 1024 * 1024), "1.00 GB");
    assert_eq!(
        ModelAnalyzer::format_size(3 * 1024 * 1024 * 1024),
        "3.00 GB"
    );
}

#[test]
fn test_format_size_terabytes() {
    assert_eq!(
        ModelAnalyzer::format_size(1024u64 * 1024 * 1024 * 1024),
        "1.00 TB"
    );
}

#[test]
fn test_format_parameters() {
    assert_eq!(ModelAnalyzer::format_parameters(500), "500");
    assert_eq!(ModelAnalyzer::format_parameters(1_500), "1.50K");
    assert_eq!(ModelAnalyzer::format_parameters(1_500_000), "1.50M");
    assert_eq!(ModelAnalyzer::format_parameters(7_000_000_000), "7.00B");
    assert_eq!(ModelAnalyzer::format_parameters(13_000_000_000), "13.00B");
}

#[test]
fn test_analyze_model() {
    let data = vec![0u8; 10_000_000]; // 10 MB
    let metadata = ModelMetadata::new("test_model".to_string(), ModelFormat::PyTorch)
        .with_framework("PyTorch".to_string())
        .with_task("text-generation".to_string());

    let analysis = ModelAnalyzer::analyze(&data, &metadata);

    assert_eq!(analysis.size_bytes, 10_000_000);
    assert!(analysis.size_mb > 9.0 && analysis.size_mb < 10.0);
    assert!(analysis.estimated_parameters.is_some());
    assert_eq!(analysis.framework, Some("PyTorch".to_string()));
}

// ===== Model Archive Tests =====

#[test]
fn test_tar_archive_creation_and_extraction() {
    let temp_dir = tempdir().unwrap();
    let archive_path = temp_dir.path().join("models.tar");

    let models = vec![
        ("model1.pt".to_string(), vec![1, 2, 3, 4, 5]),
        ("model2.onnx".to_string(), vec![6, 7, 8, 9, 10]),
    ];

    // Create archive
    let size = ModelArchive::create_tar(models.clone(), &archive_path).unwrap();
    assert!(size > 0);
    assert!(archive_path.exists());

    // Extract archive
    let extracted = ModelArchive::extract_tar(&archive_path).unwrap();
    assert_eq!(extracted.len(), 2);

    // Verify contents
    assert!(extracted.iter().any(|(name, _)| name == "model1.pt"));
    assert!(extracted.iter().any(|(name, _)| name == "model2.onnx"));
}

#[test]
fn test_zip_archive_creation_and_extraction() {
    let temp_dir = tempdir().unwrap();
    let archive_path = temp_dir.path().join("models.zip");

    let models = vec![
        ("model1.safetensors".to_string(), vec![1, 2, 3, 4, 5]),
        ("model2.gguf".to_string(), vec![6, 7, 8, 9, 10]),
    ];

    // Create archive
    let size = ModelArchive::create_zip(models.clone(), &archive_path).unwrap();
    assert!(size > 0);
    assert!(archive_path.exists());

    // Extract archive
    let extracted = ModelArchive::extract_zip(&archive_path).unwrap();
    assert_eq!(extracted.len(), 2);
}

// ===== Model Exporter Tests =====

#[test]
fn test_export_with_metadata() {
    let temp_dir = tempdir().unwrap();

    let model_data = vec![1, 2, 3, 4, 5];
    let metadata = ModelMetadata::new("exported_model".to_string(), ModelFormat::PyTorch)
        .with_description("Test model".to_string())
        .with_framework("PyTorch".to_string());

    let path = ModelExporter::export_with_metadata(model_data, &metadata, temp_dir.path()).unwrap();

    assert!(path.exists());
    assert_eq!(path.file_name().unwrap(), "exported_model.pt");

    // Check metadata file
    let meta_path = temp_dir.path().join("exported_model.meta.json");
    assert!(meta_path.exists());
}

#[test]
fn test_export_to_directory() {
    let temp_dir = tempdir().unwrap();

    let models = vec![
        (
            vec![1, 2, 3],
            ModelMetadata::new("model1".to_string(), ModelFormat::ONNX),
        ),
        (
            vec![4, 5, 6],
            ModelMetadata::new("model2".to_string(), ModelFormat::TFLite),
        ),
    ];

    let paths = ModelExporter::export_to_directory(models, temp_dir.path()).unwrap();

    assert_eq!(paths.len(), 2);
    assert!(paths[0].exists());
    assert!(paths[1].exists());
}

// ===== Model Deduplicator Tests =====

#[test]
fn test_calculate_hash() {
    let data = b"test model data";
    let hash1 = ModelDeduplicator::calculate_hash(data);
    let hash2 = ModelDeduplicator::calculate_hash(data);

    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64); // SHA256 produces 64 hex characters
}

#[test]
fn test_calculate_hash_different_data() {
    let data1 = b"test model data";
    let data2 = b"different data";

    let hash1 = ModelDeduplicator::calculate_hash(data1);
    let hash2 = ModelDeduplicator::calculate_hash(data2);

    assert_ne!(hash1, hash2);
}

#[test]
fn test_find_duplicates() {
    let models = vec![
        ("model1".to_string(), vec![1, 2, 3, 4, 5]),
        ("model2".to_string(), vec![1, 2, 3, 4, 5]), // Duplicate
        ("model3".to_string(), vec![6, 7, 8, 9, 10]),
        ("model4".to_string(), vec![1, 2, 3, 4, 5]), // Duplicate
    ];

    let duplicates = ModelDeduplicator::find_duplicates(models);

    // Should find one group of duplicates (model1, model2, model4)
    assert_eq!(duplicates.len(), 1);

    let duplicate_group = duplicates.values().next().unwrap();
    assert_eq!(duplicate_group.len(), 3);
}

#[test]
fn test_find_no_duplicates() {
    let models = vec![
        ("model1".to_string(), vec![1, 2, 3]),
        ("model2".to_string(), vec![4, 5, 6]),
        ("model3".to_string(), vec![7, 8, 9]),
    ];

    let duplicates = ModelDeduplicator::find_duplicates(models);
    assert_eq!(duplicates.len(), 0);
}

#[test]
fn test_similarity_score_identical() {
    let data1 = b"test data";
    let data2 = b"test data";

    let score = ModelDeduplicator::similarity_score(data1, data2);
    assert_eq!(score, 100.0);
}

#[test]
fn test_similarity_score_different_length() {
    let data1 = b"test data";
    let data2 = b"different length data";

    let score = ModelDeduplicator::similarity_score(data1, data2);
    assert_eq!(score, 0.0);
}

#[test]
fn test_similarity_score_partial() {
    let data1 = b"test data";
    let data2 = b"best data";

    let score = ModelDeduplicator::similarity_score(data1, data2);
    assert!(score > 0.0 && score < 100.0);
}
