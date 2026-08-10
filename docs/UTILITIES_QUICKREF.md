# Model Utilities Quick Reference

Quick reference guide for IronVault utilities. For complete documentation, see [UTILITIES.md](UTILITIES.md).

## 📦 Archiving

### Create Archives
```rust
use ironvault::ModelArchive;

// TAR archive
let models = vec![
    ("model1.pt".to_string(), data1),
    ("model2.onnx".to_string(), data2),
];
ModelArchive::create_tar(models, Path::new("backup.tar"))?;

// ZIP archive
ModelArchive::create_zip(models, Path::new("backup.zip"))?;
```

### Extract Archives
```rust
// Extract TAR
let models = ModelArchive::extract_tar(Path::new("backup.tar"))?;

// Extract ZIP
let models = ModelArchive::extract_zip(Path::new("backup.zip"))?;
```

## ⚡ Caching

### Setup and Use
```rust
use ironvault::RetrievalOptimizer;

// Create cache (100 MB limit)
let mut cache = RetrievalOptimizer::new(100 * 1024 * 1024);

// Cache model
cache.cache_model("llama-7b".to_string(), model_data)?;

// Retrieve from cache
if let Some(data) = cache.get_cached("llama-7b") {
    // Fast retrieval!
}

// Get statistics
let stats = cache.cache_stats();
println!("Utilization: {:.1}%", stats.utilization);
```

## 🔍 Deduplication

### Find Duplicates
```rust
use ironvault::ModelDeduplicator;

let models = vec![
    ("model1".to_string(), data1),
    ("model2".to_string(), data2),
    ("model1-copy".to_string(), data1), // Duplicate!
];

let duplicates = ModelDeduplicator::find_duplicates(models);
for (hash, names) in duplicates {
    println!("Duplicates: {:?}", names);
}
```

### Calculate Similarity
```rust
let similarity = ModelDeduplicator::similarity_score(&data1, &data2);
println!("Models are {:.1}% similar", similarity);
```

## 📊 Analysis

### Analyze Model
```rust
use ironvault::{ModelAnalyzer, ModelMetadata};

let analysis = ModelAnalyzer::analyze(&data, &metadata);
println!("Size: {}", ModelAnalyzer::format_size(analysis.size_bytes));
println!("Params: {}", 
    ModelAnalyzer::format_parameters(
        analysis.estimated_parameters.unwrap()
    )
);
```

### Format Utilities
```rust
// Human-readable sizes
ModelAnalyzer::format_size(1024 * 1024 * 1024);  // "1.00 GB"
ModelAnalyzer::format_size(5 * 1024 * 1024);      // "5.00 MB"

// Parameter counts
ModelAnalyzer::format_parameters(7_000_000_000);  // "7.00B"
ModelAnalyzer::format_parameters(1_500_000);      // "1.50M"
```

## 🗜️ Compression

### Analyze Compression
```rust
use ironvault::CompressionAnalyzer;

// Calculate ratio
let ratio = CompressionAnalyzer::compression_ratio(10000, 7500);
// ratio = 1.33

// Full analysis
let report = CompressionAnalyzer::analyze_compression(
    10000,
    7500,
    &ModelFormat::PyTorch
);
println!("Saved {:.1}%", report.space_saved_percent);
```

### Estimate by Format
```rust
let ratio = CompressionAnalyzer::estimate_ratio(&ModelFormat::PyTorch);
// ratio ≈ 1.3
```

## 🔢 Quantization

### Get Schemes
```rust
use ironvault::QuantizationInfo;

let schemes = QuantizationInfo::schemes();
// ["FP32", "FP16", "INT8", "Q4_0", ...]
```

### Estimate Size
```rust
// FP32 → INT8
let new_size = QuantizationInfo::estimate_size(
    4_000_000_000,  // 4 GB
    32,             // from bits
    8               // to bits
);
// new_size = 1_000_000_000 (1 GB)
```

### Calculate Savings
```rust
let savings = QuantizationInfo::memory_savings(4_000_000_000, 1_000_000_000);
println!("Saved {:.1}%", savings.saved_percent);  // 75%
println!("{}x smaller", savings.size_ratio);      // 4x
```

## ✂️ Pruning

### Track Pruning
```rust
use ironvault::{PruningInfo, PruningMethod};

let pruning = PruningInfo::new(
    PruningMethod::Magnitude,
    0.5,           // 50% sparsity
    7_000_000_000, // original params
    3_500_000_000  // remaining params
);

println!("Sparsity: {:.1}%", pruning.calculate_sparsity() * 100.0);
println!("Reduction: {:.1}%", pruning.size_reduction());
```

### Pruning Methods
```rust
PruningMethod::Magnitude
PruningMethod::Structured
PruningMethod::Unstructured
PruningMethod::GradientBased
PruningMethod::LayerWise
PruningMethod::Custom("method_name".to_string())
```

## 📤 Export

### Export with Metadata
```rust
use ironvault::ModelExporter;

// Export single model
ModelExporter::export_with_metadata(
    model_data,
    &metadata,
    Path::new("exports/")
)?;
// Creates: exports/model.onnx + exports/model.meta.json
```

### Batch Export
```rust
let models = vec![
    (data1, metadata1),
    (data2, metadata2),
];
let paths = ModelExporter::export_to_directory(
    models,
    Path::new("exports/")
)?;
```

## 📝 Common Patterns

### Backup All Models
```rust
let vault = Vault::new(None)?;
vault.unlock(passphrase)?;

let mut models = Vec::new();
for name in vault.list_models() {
    let data = vault.get_model(&name, None)?;
    models.push((format!("{}.pt", name), data));
}

ModelArchive::create_zip(models, Path::new("backup.zip"))?;
```

### Find and Remove Duplicates
```rust
let mut all_models = Vec::new();
for name in vault.list_models() {
    let data = vault.get_model(&name, None)?;
    all_models.push((name.clone(), data));
}

let duplicates = ModelDeduplicator::find_duplicates(all_models);
for (_, names) in duplicates {
    // Keep first, delete rest
    for name in names.iter().skip(1) {
        vault.delete_version(name, 1)?;
    }
}
```

### Optimize with Caching
```rust
let mut cache = RetrievalOptimizer::new(1024 * 1024 * 1024);

// First access (slow)
let data = vault.get_model("llama-7b", None)?;
cache.cache_model("llama-7b".to_string(), data.clone())?;

// Subsequent accesses (fast)
if let Some(cached) = cache.get_cached("llama-7b") {
    // Use cached data
}
```

### Analyze Model Portfolio
```rust
for name in vault.list_models() {
    let data = vault.get_model(&name, None)?;
    let metadata = /* get metadata */;
    
    let analysis = ModelAnalyzer::analyze(&data, &metadata);
    println!("{}: {} with {} params",
        name,
        ModelAnalyzer::format_size(analysis.size_bytes),
        analysis.estimated_parameters
            .map(|p| ModelAnalyzer::format_parameters(p))
            .unwrap_or("unknown".to_string())
    );
}
```

## 🎯 Performance Tips

1. **Cache frequently accessed models** - Use `RetrievalOptimizer` for 10-100x speedup
2. **Deduplicate before archiving** - Save storage space
3. **Use TAR for speed, ZIP for compression** - TAR is faster, ZIP compresses better
4. **Estimate before quantizing** - Use `QuantizationInfo::estimate_size()` first
5. **Batch operations** - Export/archive multiple models at once

## 📚 See Also

- [Complete Utilities Guide](UTILITIES.md) - Detailed documentation and examples
- [API Documentation](https://docs.rs/ironvault) - Full API reference
- [Examples](https://github.com/nervosys/IronVault/blob/master/examples/) - Working code examples
