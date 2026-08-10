# AI Model Utilities

Comprehensive utilities for working with AI models in IronVault. This module provides popular operations for model management, optimization, and analysis.

## Features

### 📦 Model Archiving

Create and extract TAR/ZIP archives of multiple models:

```rust
use ironvault::ModelArchive;

// Create TAR archive
let models = vec![
    ("model1.safetensors".to_string(), model1_data),
    ("model2.gguf".to_string(), model2_data),
];
ModelArchive::create_tar(models, Path::new("models.tar"))?;

// Extract TAR archive
let extracted = ModelArchive::extract_tar(Path::new("models.tar"))?;

// Create ZIP archive
ModelArchive::create_zip(models, Path::new("models.zip"))?;
```

### 🗜️ Compression Analysis

Analyze compression effectiveness and estimate ratios:

```rust
use ironvault::CompressionAnalyzer;

// Calculate compression ratio
let ratio = CompressionAnalyzer::compression_ratio(10000, 7500);
// ratio = 1.33

// Estimate compression for format
let estimated = CompressionAnalyzer::estimate_ratio(&ModelFormat::PyTorch);

// Full compression analysis
let report = CompressionAnalyzer::analyze_compression(
    10000, 
    7500, 
    &ModelFormat::PyTorch
);
println!("Saved {}% space", report.space_saved_percent);
```

### ⚡ Retrieval Optimization

LRU cache for fast model retrieval:

```rust
use ironvault::RetrievalOptimizer;

// Create cache with 100MB limit
let mut optimizer = RetrievalOptimizer::new(100 * 1024 * 1024);

// Cache a model
optimizer.cache_model("llama-7b".to_string(), model_data)?;

// Retrieve from cache
if let Some(data) = optimizer.get_cached("llama-7b") {
    // Fast retrieval from cache
}

// Get cache statistics
let stats = optimizer.cache_stats();
println!("Cache utilization: {:.2}%", stats.utilization);
```

### 🔢 Quantization Metadata

Work with quantization information:

```rust
use ironvault::QuantizationInfo;

// Get supported quantization schemes
let schemes = QuantizationInfo::schemes();
// ["FP32", "FP16", "INT8", "Q4_0", "Q8_0", ...]

// Estimate size after quantization
let new_size = QuantizationInfo::estimate_size(
    1000000,  // original size
    32,       // from bits (FP32)
    8         // to bits (INT8)
);
// new_size = 250000 (4x smaller)

// Calculate memory savings
let savings = QuantizationInfo::memory_savings(1000000, 250000);
println!("Saved {:.1}%", savings.saved_percent);  // 75%
```

### ✂️ Pruning Information

Track model pruning metadata:

```rust
use ironvault::{PruningInfo, PruningMethod};

let pruning = PruningInfo::new(
    PruningMethod::Magnitude,
    0.5,           // target sparsity
    7_000_000_000, // original params
    3_500_000_000  // remaining params
);

let actual_sparsity = pruning.calculate_sparsity();  // 0.5
let reduction = pruning.size_reduction();            // 50.0%
```

### 📊 Model Analysis

Analyze model files and metadata:

```rust
use ironvault::ModelAnalyzer;

let analysis = ModelAnalyzer::analyze(&model_data, &metadata);

println!("Format: {}", analysis.format);
println!("Size: {}", ModelAnalyzer::format_size(analysis.size_bytes));
println!("Parameters: {}", 
    ModelAnalyzer::format_parameters(
        analysis.estimated_parameters.unwrap()
    )
);

// Output:
// Format: Safetensors
// Size: 13.50 GB
// Parameters: 7.00B
```

### 📤 Model Export

Export models with metadata:

```rust
use ironvault::ModelExporter;

// Export single model with metadata
ModelExporter::export_with_metadata(
    model_data,
    &metadata,
    Path::new("exports/")
)?;

// Export multiple models
let models = vec![
    (data1, metadata1),
    (data2, metadata2),
];
let paths = ModelExporter::export_to_directory(models, Path::new("exports/"))?;
```

### 🔍 Deduplication

Find and remove duplicate models:

```rust
use ironvault::ModelDeduplicator;

let models = vec![
    ("model1".to_string(), data1),
    ("model2".to_string(), data2),
    ("model3".to_string(), data1),  // Duplicate of model1
];

// Find duplicates by hash
let duplicates = ModelDeduplicator::find_duplicates(models);
for (hash, model_names) in duplicates {
    println!("Duplicates: {:?}", model_names);
}

// Calculate similarity
let similarity = ModelDeduplicator::similarity_score(&data1, &data2);
println!("Models are {:.1}% similar", similarity);
```

## Utility Classes

### CompressionAnalyzer

- `compression_ratio(original, compressed) -> f64`
- `estimate_ratio(format) -> f64`
- `analyze_compression(original, compressed, format) -> CompressionReport`

### QuantizationInfo

- `schemes() -> Vec<&'static str>`
- `estimate_size(original, from_bits, to_bits) -> u64`
- `memory_savings(original, quantized) -> QuantizationSavings`
- `is_valid_scheme(scheme) -> bool`

### PruningInfo

- `new(method, sparsity, original_params, remaining_params) -> Self`
- `calculate_sparsity() -> f64`
- `size_reduction() -> f64`

### RetrievalOptimizer

- `new(max_cache_size) -> Self`
- `cache_model(key, data) -> Result<()>`
- `get_cached(key) -> Option<Vec<u8>>`
- `clear_cache()`
- `cache_stats() -> CacheStats`

### ModelAnalyzer

- `analyze(data, metadata) -> ModelAnalysis`
- `format_size(bytes) -> String`
- `format_parameters(params) -> String`

### ModelArchive

- `create_tar(models, output_path) -> Result<usize>`
- `extract_tar(archive_path) -> Result<Vec<(String, Vec<u8>)>>`
- `create_zip(models, output_path) -> Result<usize>`
- `extract_zip(archive_path) -> Result<Vec<(String, Vec<u8>)>>`

### ModelExporter

- `export_with_metadata(data, metadata, output_dir) -> Result<PathBuf>`
- `export_to_directory(models, output_dir) -> Result<Vec<PathBuf>>`

### ModelDeduplicator

- `calculate_hash(data) -> String`
- `find_duplicates(models) -> HashMap<String, Vec<String>>`
- `similarity_score(data1, data2) -> f64`

## Common Use Cases

### Backup Multiple Models

```rust
let models = vault.list_models();
let mut archive_data = Vec::new();

for model_name in models {
    let data = vault.get_model(&model_name, None)?;
    archive_data.push((format!("{}.safetensors", model_name), data));
}

ModelArchive::create_zip(archive_data, Path::new("backup.zip"))?;
```

### Optimize Storage with Caching

```rust
let mut cache = RetrievalOptimizer::new(1024 * 1024 * 1024); // 1GB cache

// First access (cache miss)
let data = vault.get_model("llama-7b", None)?;
cache.cache_model("llama-7b".to_string(), data.clone())?;

// Subsequent accesses (cache hit - much faster)
if let Some(cached_data) = cache.get_cached("llama-7b") {
    // Use cached data
}
```

### Find and Remove Duplicates

```rust
let mut all_models = Vec::new();

for name in vault.list_models() {
    let data = vault.get_model(&name, None)?;
    all_models.push((name.clone(), data));
}

let duplicates = ModelDeduplicator::find_duplicates(all_models);

// Keep first copy, delete duplicates
for (_, model_names) in duplicates {
    for name in model_names.iter().skip(1) {
        println!("Deleting duplicate: {}", name);
        // vault.delete_model(name)?;
    }
}
```

### Analyze Model Portfolio

```rust
for model_name in vault.list_models() {
    let data = vault.get_model(&model_name, None)?;
    let metadata = /* get metadata */;
    
    let analysis = ModelAnalyzer::analyze(&data, &metadata);
    
    println!("{}: {} with {} params", 
        model_name,
        ModelAnalyzer::format_size(analysis.size_bytes),
        analysis.estimated_parameters
            .map(|p| ModelAnalyzer::format_parameters(p))
            .unwrap_or("unknown".to_string())
    );
}
```

## Performance Tips

1. **Caching**: Use `RetrievalOptimizer` for frequently accessed models
2. **Compression**: Analyze compression effectiveness before applying to all models
3. **Deduplication**: Run periodically to save storage space
4. **Quantization**: Estimate size savings before quantizing large models
5. **Archiving**: Use TAR for simple archiving, ZIP for better compression

## See Also

- [Formats Guide](https://github.com/nervosys/IronVault/blob/master/FORMATS.md) - Supported model formats
- [CLI Documentation](CLI.md) - Command-line usage
