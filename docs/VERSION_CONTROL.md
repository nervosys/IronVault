# Version Control System - Complete Guide

## Overview

IronVault (AIMV) provides a **Git-like version control system** specifically designed for AI models. Track every checkpoint, branch experiments, trace lineage, and rollback instantly - all with military-grade encryption and data integrity verification.

## Table of Contents

- [Core Concepts](#core-concepts)
- [Version Management](#version-management)
- [Branching](#branching)
- [Lineage Tracking](#lineage-tracking)
- [Time Travel](#time-travel)
- [Version Comparison](#version-comparison)
- [Cleanup Policies](#cleanup-policies)
- [Checksum Verification](#checksum-verification)
- [Metadata Tracking](#metadata-tracking)
- [Complete Workflows](#complete-workflows)
- [API Reference](#api-reference)
- [Best Practices](#best-practices)

---

## Core Concepts

### Version Structure

Every model version in AIMV includes:

```rust
pub struct ModelVersion {
    pub version: u32,                    // Sequential: 1, 2, 3, ...
    pub checkpoint_id: String,           // Unique: "model-v2-uuid"
    pub timestamp: DateTime<Utc>,        // When created
    pub parent_version: Option<u32>,     // Lineage tracking
    pub format: String,                  // safetensors, gguf, etc.
    pub size_bytes: u64,                 // Original size
    pub compressed_size_bytes: u64,      // After compression
    pub checksum_sha256: String,         // Integrity verification
    pub metadata: HashMap<String, String>, // Custom fields
    pub file_path: String,               // Encrypted file location
}
```

### Key Features

1. **Sequential Versioning**: Automatic v1, v2, v3, ... numbering
2. **Unique Identifiers**: UUID-based checkpoint IDs
3. **Timestamp Tracking**: Precise creation time for each version
4. **Parent Relationships**: Track model lineage and evolution
5. **Format Agnostic**: Works with any model format
6. **Size Tracking**: Both original and compressed sizes
7. **Integrity Verification**: SHA-256 checksums
8. **Rich Metadata**: Extensible key-value metadata

---

## Version Management

### Creating Versions

```rust
use ironvault::{VaultConfig, formats::ModelMetadata};

// Initialize vault
let config = VaultConfig::new()?;
let mut vault = config.build()?;

// Create metadata
let metadata = ModelMetadata::new(
    "llama-2-7b-chat".to_string(),
    ModelFormat::Safetensors,
)
.with_framework("PyTorch".to_string())
.with_task("text-generation".to_string())
.with_parameters(7_200_000_000);

// Store version 1 (base model)
let v1 = vault.store_model(
    "llama-2-7b-chat",
    &model_data,
    &metadata,
    None  // No parent
)?;

println!("Created v{}: {}", v1.version, v1.checkpoint_id);
```

### Listing Versions

```rust
// Get all versions
let versions = vault.list_versions("llama-2-7b-chat");

for version in versions {
    println!("v{}: {} ({})", 
        version.version,
        version.timestamp.format("%Y-%m-%d"),
        version.format
    );
}

// Output:
// v1: 2024-10-01 (safetensors)
// v2: 2024-10-15 (safetensors)
// v3: 2024-10-22 (safetensors)
```

### Getting Specific Version

```rust
// Get latest version
let latest = vault.get_version("llama-2-7b-chat", None);

// Get specific version
let v3 = vault.get_version("llama-2-7b-chat", Some(3));

if let Some(version) = v3 {
    println!("Version {}: {} bytes", 
        version.version, 
        version.size_bytes
    );
}
```

---

## Branching

### Creating Branches

Branches allow parallel development from the same parent:

```rust
// Base model (v1)
let v1 = vault.store_model(
    "llama-2-7b",
    &base_data,
    &metadata_base,
    None
)?;

// General fine-tuning (v2 from v1)
let v2 = vault.store_model(
    "llama-2-7b",
    &finetune_data,
    &metadata_finetune,
    Some(1)  // Parent: v1
)?;

// Create two experimental branches from v2
let v3 = vault.store_model(
    "llama-2-7b",
    &experiment_a_data,
    &metadata_exp_a,
    Some(2)  // Branch A from v2
)?;

let v4 = vault.store_model(
    "llama-2-7b",
    &experiment_b_data,
    &metadata_exp_b,
    Some(2)  // Branch B from v2
)?;
```

### Version Tree

```
v1 (base)
│
v2 (fine-tuned)
├─ v3 (experiment-a)
│  └─ v5 (improved-a)
└─ v4 (experiment-b)
   └─ v6 (improved-b)
```

### Branching Use Cases

| Use Case            | Description                      | Example                   |
| ------------------- | -------------------------------- | ------------------------- |
| **A/B Testing**     | Compare different approaches     | Try 2 learning rates      |
| **Multi-task**      | Specialize for different tasks   | Chat vs Code vs Math      |
| **Quantization**    | Create multiple quant levels     | Q4, Q5, Q8 from same base |
| **Experimentation** | Try new techniques safely        | RLHF, DPO, etc.           |
| **Feature Dev**     | Develop capabilities in parallel | Add RAG, tool use, etc.   |

---

## Lineage Tracking

### Getting Complete Lineage

```rust
// Get full ancestry of v5
let lineage = vault.get_lineage("llama-2-7b-chat", 5);

for (i, version) in lineage.iter().enumerate() {
    let indent = "  ".repeat(i);
    println!("{}v{}: {} ({})", 
        indent,
        version.version,
        version.timestamp.format("%Y-%m-%d"),
        version.metadata.get("stage").unwrap_or(&"unknown".to_string())
    );
}

// Output:
// v1: 2024-10-01 (pre-training)
//   v2: 2024-10-15 (fine-tuning)
//     v3: 2024-10-22 (chat-tuning)
//       v5: 2024-11-05 (rlhf)
```

### Lineage Information

```rust
let lineage = vault.get_lineage("llama-2-7b-chat", 5);

println!("Generation depth: {}", lineage.len());
println!("Root version: v{}", lineage[0].version);
println!("Current version: v{}", lineage.last().unwrap().version);

// Calculate training duration
let start = lineage[0].timestamp;
let end = lineage.last().unwrap().timestamp;
let duration = end - start;
println!("Training duration: {} days", duration.num_days());
```

### Lineage Use Cases

- **Reproducibility**: Trace exact training path
- **Audit Trail**: Document model evolution
- **Compliance**: Show regulatory checkpoint history
- **Research**: Track experimental variations
- **Debugging**: Identify where issues were introduced

---

## Time Travel

### Rollback to Previous Version

```rust
// Current production: v5
// Problem detected: model too cautious

// 1. List available versions
let versions = vault.list_versions("llama-2-7b-chat");
for v in versions {
    println!("v{}: {}", v.version, 
        v.metadata.get("description").unwrap_or(&"".to_string()));
}

// 2. Load previous version
let v3_data = vault.get_model("llama-2-7b-chat", Some(3))?;

// 3. Test v3
run_evaluation_suite(&v3_data)?;

// 4. Deploy v3 or create new branch
deploy_to_production(&v3_data)?;

// Or continue development from v3
let v6 = vault.store_model(
    "llama-2-7b-chat",
    &improved_data,
    &metadata_v6,
    Some(3)  // Continue from v3
)?;
```

### Instant Rollback Scenarios

| Scenario                | Action                       | Benefit                      |
| ----------------------- | ---------------------------- | ---------------------------- |
| **Production Issue**    | Load previous stable version | Zero downtime recovery       |
| **Quality Degradation** | Rollback to baseline         | Maintain service quality     |
| **Failed Experiment**   | Return to known good state   | Continue from safe point     |
| **A/B Test Lost**       | Revert to control            | Restore original behavior    |
| **Compliance Need**     | Load audited checkpoint      | Meet regulatory requirements |

### Time Travel Workflow

```rust
// Complete rollback workflow
fn rollback_workflow() -> Result<()> {
    let mut vault = VaultConfig::new()?.build()?;
    
    // Current problematic version
    let current = 5;
    let target = 3;
    
    // 1. Get version metadata
    let v5 = vault.get_version("model", Some(current)).unwrap();
    let v3 = vault.get_version("model", Some(target)).unwrap();
    
    // 2. Compare versions
    println!("Current v{}: {}", current, v5.timestamp);
    println!("Target v{}: {}", target, v3.timestamp);
    println!("Rolling back {} days", 
        (v5.timestamp - v3.timestamp).num_days());
    
    // 3. Load target version
    let model_data = vault.get_model("model", Some(target))?;
    
    // 4. Verify integrity
    assert!(vault.verify_checksum("model", target, &model_data));
    
    // 5. Deploy
    deploy(&model_data)?;
    
    println!("✓ Rolled back to v{}", target);
    Ok(())
}
```

---

## Version Comparison

### Side-by-Side Comparison

```rust
let v3 = vault.get_version("llama-2-7b-chat", Some(3)).unwrap();
let v5 = vault.get_version("llama-2-7b-chat", Some(5)).unwrap();

println!("Comparison: v{} vs v{}", v3.version, v5.version);
println!();

// Size comparison
println!("Original Size:");
println!("  v{}: {} bytes", v3.version, v3.size_bytes);
println!("  v{}: {} bytes", v5.version, v5.size_bytes);
println!("  Difference: {} bytes", 
    v5.size_bytes as i64 - v3.size_bytes as i64);
println!();

// Compression comparison
let v3_ratio = (1.0 - v3.compressed_size_bytes as f64 / v3.size_bytes as f64) * 100.0;
let v5_ratio = (1.0 - v5.compressed_size_bytes as f64 / v5.size_bytes as f64) * 100.0;
println!("Compression Ratio:");
println!("  v{}: {:.1}%", v3.version, v3_ratio);
println!("  v{}: {:.1}%", v5.version, v5_ratio);
println!();

// Metadata comparison
println!("Metadata Diff:");
for (key, v5_value) in &v5.metadata {
    if let Some(v3_value) = v3.metadata.get(key) {
        if v3_value != v5_value {
            println!("  ~ {}: {} → {}", key, v3_value, v5_value);
        }
    } else {
        println!("  + {}: {}", key, v5_value);
    }
}
```

### Comparison Table

```
┌─────────────────────┬──────────────────┬──────────────────┐
│ Metric              │ v3 (Baseline)    │ v5 (Chat-pro)    │
├─────────────────────┼──────────────────┼──────────────────┤
│ Version             │ 3                │ 5                │
│ Date                │ 2024-10-22       │ 2024-11-05       │
│ Parent              │ v2               │ v3               │
│ Generation          │ 3                │ 4                │
├─────────────────────┼──────────────────┼──────────────────┤
│ Original Size       │ 13.2 GB          │ 13.2 GB          │
│ Compressed Size     │ 7.8 GB           │ 7.9 GB           │
│ Compression Ratio   │ 41%              │ 40%              │
├─────────────────────┼──────────────────┼──────────────────┤
│ Training Epochs     │ 40               │ 48               │
│ Learning Rate       │ 2e-5             │ 1e-5             │
│ Batch Size          │ 128              │ 256              │
└─────────────────────┴──────────────────┴──────────────────┘
```

---

## Cleanup Policies

### Keep Last N Versions

```rust
// Keep only the 5 most recent versions
let deleted = vault.cleanup_old_versions("llama-2-7b-chat", 5)?;

println!("Deleted {} versions: {:?}", deleted.len(), deleted);
// Output: Deleted 10 versions: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
```

### Delete Specific Version

```rust
let deleted = vault.delete_version("llama-2-7b-chat", 7)?;

if deleted {
    println!("✓ Version 7 deleted");
} else {
    println!("✗ Version 7 not found");
}
```

### Automated Cleanup

```rust
// Weekly cleanup script
fn weekly_cleanup(vault: &mut Vault) -> Result<()> {
    let models = vault.list_all_models()?;
    
    for model in models {
        // Keep last 5 versions of each model
        let deleted = vault.cleanup_old_versions(&model, 5)?;
        
        if !deleted.is_empty() {
            println!("{}: Deleted {} versions", model, deleted.len());
        }
    }
    
    Ok(())
}
```

### Retention Policies

| Policy               | Description               | Use Case             | Implementation            |
| -------------------- | ------------------------- | -------------------- | ------------------------- |
| **Keep Last N**      | Keep N most recent        | Production models    | `cleanup_old_versions(n)` |
| **Time-based**       | Delete older than X days  | Long-term storage    | Filter by timestamp       |
| **Generation-based** | Keep every Nth generation | Milestone tracking   | Filter by lineage depth   |
| **Tag-based**        | Keep tagged versions only | Critical checkpoints | Custom metadata filter    |
| **Hybrid**           | Combination of above      | Balanced approach    | Custom logic              |

### Storage Optimization Example

```rust
// Before cleanup
let versions = vault.list_versions("llama-2-7b-chat");
let total_size: u64 = versions.iter()
    .map(|v| v.compressed_size_bytes)
    .sum();
println!("Storage used: {} GB", total_size / 1_000_000_000);
// Output: Storage used: 118 GB (15 versions)

// Cleanup
vault.cleanup_old_versions("llama-2-7b-chat", 5)?;

// After cleanup
let versions = vault.list_versions("llama-2-7b-chat");
let total_size: u64 = versions.iter()
    .map(|v| v.compressed_size_bytes)
    .sum();
println!("Storage used: {} GB", total_size / 1_000_000_000);
// Output: Storage used: 39 GB (5 versions)
// Saved: 79 GB (67%)
```

---

## Checksum Verification

### Automatic Verification

Every time you retrieve a model, AIMV automatically verifies integrity:

```rust
// Automatic verification on retrieval
let data = vault.get_model("llama-2-7b-chat", Some(3))?;
// ✓ Checksum verified: a1b2c3d4e5f6...
// ✓ Data integrity confirmed

// If checksum fails, get_model returns an error
```

### Manual Verification

```rust
// Verify without loading
let is_valid = vault.verify_checksum(
    "llama-2-7b-chat",
    3,
    &model_data
);

if is_valid {
    println!("✓ Data integrity verified");
} else {
    eprintln!("✗ Data corruption detected!");
    // Handle corruption
}
```

### Checksum Details

- **Algorithm**: SHA-256 (FIPS 180-4 compliant)
- **Length**: 64 hexadecimal characters (256 bits)
- **Storage**: Included in version metadata
- **Computation**: Automatic on store/retrieve

### Protection Against

- **Bit Rot**: Storage device degradation over time
- **Transmission Errors**: Network transfer corruption
- **Unauthorized Modification**: Tampering detection
- **Data Corruption**: Filesystem or hardware issues
- **Malicious Attacks**: Integrity compromise attempts

---

## Metadata Tracking

### Custom Metadata

```rust
let metadata = ModelMetadata::new(name, format)
    .with_framework("PyTorch".to_string())
    .with_task("text-generation".to_string())
    .with_parameters(7_200_000_000)
    .add_custom_field("epochs".to_string(), "48".to_string())
    .add_custom_field("learning_rate".to_string(), "2e-5".to_string())
    .add_custom_field("batch_size".to_string(), "128".to_string())
    .add_custom_field("dataset".to_string(), "custom-corpus".to_string())
    .add_custom_field("gpu_hours".to_string(), "240".to_string());
```

### Metadata Evolution

Track how training parameters change across versions:

```
v1: stage=pre-training, tokens=1.5T, precision=fp32
v2: stage=fine-tuning, tokens=10B, precision=fp32, epochs=40
v3: stage=chat-tuning, tokens=5B, precision=fp32, epochs=40, specialization=chat
v5: stage=rlhf, tokens=5B, precision=fp16, epochs=48, rlhf_iterations=3
```

### Querying Metadata

```rust
// Find versions with specific metadata
let versions = vault.list_versions("llama-2-7b-chat");

let rlhf_versions: Vec<_> = versions
    .into_iter()
    .filter(|v| v.metadata.contains_key("rlhf_iterations"))
    .collect();

println!("Found {} RLHF versions", rlhf_versions.len());

// Find versions with high epoch count
let high_epoch_versions: Vec<_> = versions
    .into_iter()
    .filter(|v| {
        v.metadata.get("epochs")
            .and_then(|e| e.parse::<u32>().ok())
            .map(|e| e >= 40)
            .unwrap_or(false)
    })
    .collect();
```

---

## Complete Workflows

### Training Pipeline

```rust
// Day 1: Initialize with base model
let config = VaultConfig::new()?;
let mut vault = config.build()?;

let base_metadata = ModelMetadata::new(
    "customer-support-bot".to_string(),
    ModelFormat::Safetensors,
).add_custom_field("stage".to_string(), "base".to_string());

let v1 = vault.store_model(
    "customer-support-bot",
    &base_model,
    &base_metadata,
    None
)?;

// Day 3: General fine-tuning
let ft_metadata = base_metadata.clone()
    .add_custom_field("stage".to_string(), "fine-tuning".to_string())
    .add_custom_field("epochs".to_string(), "40".to_string());

let v2 = vault.store_model(
    "customer-support-bot",
    &finetuned_model,
    &ft_metadata,
    Some(1)
)?;

// Day 7: A/B testing
let exp_a_metadata = ft_metadata.clone()
    .add_custom_field("experiment".to_string(), "high-lr".to_string())
    .add_custom_field("learning_rate".to_string(), "5e-5".to_string());

let v3 = vault.store_model(
    "customer-support-bot",
    &experiment_a,
    &exp_a_metadata,
    Some(2)
)?;

let exp_b_metadata = ft_metadata.clone()
    .add_custom_field("experiment".to_string(), "low-lr".to_string())
    .add_custom_field("learning_rate".to_string(), "1e-5".to_string());

let v4 = vault.store_model(
    "customer-support-bot",
    &experiment_b,
    &exp_b_metadata,
    Some(2)
)?;

// Day 10: Evaluate and continue with winner
let v3_results = evaluate(&vault.get_model("customer-support-bot", Some(3))?)?;
let v4_results = evaluate(&vault.get_model("customer-support-bot", Some(4))?)?;

let winner = if v4_results.score > v3_results.score { 4 } else { 3 };
println!("Winner: v{}", winner);

// Continue from winner
let final_metadata = /* ... */;
let v5 = vault.store_model(
    "customer-support-bot",
    &final_model,
    &final_metadata,
    Some(winner)
)?;
```

---

## API Reference

### Core Functions

```rust
// Version Control
pub fn add_version(
    &mut self,
    model_name: &str,
    file_path: &str,
    format: &str,
    size_bytes: u64,
    compressed_size_bytes: u64,
    checksum: &str,
    metadata: Option<HashMap<String, String>>,
    parent_version: Option<u32>,
) -> Result<ModelVersion>

pub fn get_version(&self, model_name: &str, version: Option<u32>) 
    -> Option<&ModelVersion>

pub fn list_versions(&self, model_name: &str) -> Vec<&ModelVersion>

pub fn get_lineage(&self, model_name: &str, version: u32) 
    -> Vec<&ModelVersion>

pub fn delete_version(&mut self, model_name: &str, version: u32) 
    -> Result<bool>

pub fn cleanup_old_versions(&mut self, model_name: &str, keep_count: usize) 
    -> Result<Vec<u32>>

pub fn verify_checksum(&self, model_name: &str, version: u32, data: &[u8]) 
    -> bool
```

---

## Best Practices

### 1. Version Naming Convention

```rust
// Use descriptive model names
"llama-2-7b-chat"         // Good
"model1"                   // Bad

// Add stage info to metadata
.add_custom_field("stage".to_string(), "fine-tuning".to_string())
```

### 2. Parent Tracking

```rust
// Always specify parent for non-base versions
let v2 = vault.store_model(name, data, metadata, Some(1))?;  // Good
let v2 = vault.store_model(name, data, metadata, None)?;     // Bad (loses lineage)
```

### 3. Metadata Documentation

```rust
// Include comprehensive metadata
let metadata = ModelMetadata::new(name, format)
    .with_framework("PyTorch".to_string())
    .with_task("text-generation".to_string())
    .add_custom_field("stage".to_string(), "rlhf".to_string())
    .add_custom_field("dataset".to_string(), "custom-corpus".to_string())
    .add_custom_field("epochs".to_string(), "48".to_string())
    .add_custom_field("learning_rate".to_string(), "2e-5".to_string())
    .add_custom_field("gpu_hours".to_string(), "240".to_string())
    .add_custom_field("notes".to_string(), "Best RLHF run".to_string());
```

### 4. Regular Cleanup

```rust
// Implement retention policy
// Weekly: keep last 5 versions
vault.cleanup_old_versions(model_name, 5)?;

// Monthly: archive old versions externally
```

### 5. Verification

```rust
// Always verify after retrieval for critical operations
let data = vault.get_model(name, version)?;
assert!(vault.verify_checksum(name, version.unwrap(), &data));
```

### 6. Branching Strategy

```rust
// Use branches for experiments
// Main line: v1 → v2 → v3 → ... (production)
// Experiments: branch from stable versions

// Good branching
let stable = vault.store_model(name, data, metadata, Some(parent))?;
let exp_a = vault.store_model(name, exp_data, exp_metadata, Some(stable.version))?;
let exp_b = vault.store_model(name, exp_data2, exp_metadata2, Some(stable.version))?;
```

### 7. Documentation

```rust
// Document major changes in metadata
.add_custom_field("changelog".to_string(), 
    "Improved RLHF with PPO, 3 iterations".to_string())
```

---

## Performance Considerations

### Storage

- Each version stored as separate encrypted file
- Compression reduces storage by 40-60%
- Deduplication possible with content-addressed storage
- Regular cleanup recommended

### Retrieval Speed

- Version lookup: O(1) via HashMap
- Model loading: Limited by disk I/O
- Checksum verification: ~1-2 seconds for 7B model
- Lineage traversal: O(depth)

### Scalability

- Tested with 1000+ versions per model
- Version file (~1 MB per 100 versions)
- Recommend cleanup policy for long-running projects

---

## Compliance

Version control features support:

- **CMMC AU.3.046**: Audit logging of version operations
- **CMMC AU.3.049**: Audit information protection (encrypted)
- **CMMC AU.3.051**: Version history = audit trail
- **FDA 21 CFR Part 11**: Electronic records (checksum = signature)
- **GDPR**: Data lineage and provenance tracking

---

## Examples

See `examples/version_control_demo.rs` for comprehensive demonstrations:

```bash
cargo run --example version_control_demo --release
```

---

**IronVault (AIMV)** - Git-like version control for AI models with military-grade security.
