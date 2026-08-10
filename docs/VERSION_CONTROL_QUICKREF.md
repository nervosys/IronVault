# Version Control - Quick Reference

## Common Commands

### Initialize Vault
```rust
let config = VaultConfig::new()?;
let mut vault = config.build()?;
```

### Store Version
```rust
// Base version (no parent)
let v1 = vault.store_model(name, data, metadata, None)?;

// Child version (with parent)
let v2 = vault.store_model(name, data, metadata, Some(1))?;
```

### Retrieve Version
```rust
// Latest version
let data = vault.get_model(name, None)?;

// Specific version
let data = vault.get_model(name, Some(3))?;
```

### List Versions
```rust
let versions = vault.list_versions(name);
for v in versions {
    println!("v{}: {}", v.version, v.checkpoint_id);
}
```

### Get Lineage
```rust
let lineage = vault.get_lineage(name, 5);
println!("Generation depth: {}", lineage.len());
```

### Delete Version
```rust
let deleted = vault.delete_version(name, 7)?;
```

### Cleanup Old Versions
```rust
// Keep last 5 versions
let deleted = vault.cleanup_old_versions(name, 5)?;
println!("Deleted {} versions", deleted.len());
```

### Verify Checksum
```rust
let is_valid = vault.verify_checksum(name, version, &data);
```

---

## Version Structure

```rust
pub struct ModelVersion {
    version: u32,                     // Sequential: 1, 2, 3...
    checkpoint_id: String,            // Unique UUID
    timestamp: DateTime<Utc>,         // Creation time
    parent_version: Option<u32>,      // Parent for lineage
    format: String,                   // Model format
    size_bytes: u64,                  // Original size
    compressed_size_bytes: u64,       // After compression
    checksum_sha256: String,          // SHA-256 integrity
    metadata: HashMap<String, String>, // Custom fields
    file_path: String,                // Encrypted file path
}
```

---

## Metadata Examples

### Basic Metadata
```rust
let metadata = ModelMetadata::new(name, format)
    .with_framework("PyTorch".to_string())
    .with_task("text-generation".to_string())
    .with_parameters(7_200_000_000);
```

### Extended Metadata
```rust
let metadata = ModelMetadata::new(name, format)
    .with_framework("PyTorch".to_string())
    .with_task("text-generation".to_string())
    .with_parameters(7_200_000_000)
    .add_custom_field("stage".to_string(), "fine-tuning".to_string())
    .add_custom_field("epochs".to_string(), "48".to_string())
    .add_custom_field("learning_rate".to_string(), "2e-5".to_string())
    .add_custom_field("batch_size".to_string(), "128".to_string())
    .add_custom_field("dataset".to_string(), "custom-corpus".to_string())
    .add_custom_field("gpu_hours".to_string(), "240".to_string());
```

---

## Branching Pattern

```rust
// Create base (v1)
let v1 = vault.store_model(name, base_data, metadata_base, None)?;

// Create main line (v2 from v1)
let v2 = vault.store_model(name, ft_data, metadata_ft, Some(1))?;

// Create branches (v3, v4 from v2)
let v3 = vault.store_model(name, exp_a_data, metadata_a, Some(2))?;
let v4 = vault.store_model(name, exp_b_data, metadata_b, Some(2))?;

// Continue from winner (v5 from v4)
let v5 = vault.store_model(name, winner_data, metadata_final, Some(4))?;
```

**Version Tree:**
```
v1 (base)
│
v2 (fine-tuned)
├─ v3 (experiment-a)
└─ v4 (experiment-b)
   └─ v5 (production)
```

---

## Rollback Pattern

```rust
// Production issue detected at v5
let current = 5;
let target = 3;

// 1. Get version info
let v5 = vault.get_version(name, Some(current)).unwrap();
let v3 = vault.get_version(name, Some(target)).unwrap();

// 2. Load target version
let data = vault.get_model(name, Some(target))?;

// 3. Verify integrity
assert!(vault.verify_checksum(name, target, &data));

// 4. Deploy
deploy(&data)?;
```

---

## Comparison Pattern

```rust
let v3 = vault.get_version(name, Some(3)).unwrap();
let v5 = vault.get_version(name, Some(5)).unwrap();

// Size comparison
let size_diff = v5.size_bytes as i64 - v3.size_bytes as i64;
println!("Size difference: {} bytes", size_diff);

// Compression comparison
let v3_ratio = (1.0 - v3.compressed_size_bytes as f64 / v3.size_bytes as f64) * 100.0;
let v5_ratio = (1.0 - v5.compressed_size_bytes as f64 / v5.size_bytes as f64) * 100.0;
println!("Compression: {:.1}% → {:.1}%", v3_ratio, v5_ratio);

// Metadata diff
for (key, v5_value) in &v5.metadata {
    if let Some(v3_value) = v3.metadata.get(key) {
        if v3_value != v5_value {
            println!("{}: {} → {}", key, v3_value, v5_value);
        }
    } else {
        println!("{}: {} (new)", key, v5_value);
    }
}
```

---

## Cleanup Strategies

### Keep Last N
```rust
// Keep 5 most recent
vault.cleanup_old_versions(name, 5)?;
```

### Time-based
```rust
let cutoff = Utc::now() - Duration::days(30);
let versions = vault.list_versions(name);

for v in versions {
    if v.timestamp < cutoff {
        vault.delete_version(name, v.version)?;
    }
}
```

### Tag-based
```rust
// Keep only production versions
let versions = vault.list_versions(name);

for v in versions {
    if !v.metadata.contains_key("production") {
        vault.delete_version(name, v.version)?;
    }
}
```

### Generation-based
```rust
// Keep every 5th generation
let versions = vault.list_versions(name);

for v in versions {
    let lineage = vault.get_lineage(name, v.version);
    if lineage.len() % 5 != 0 {
        vault.delete_version(name, v.version)?;
    }
}
```

---

## Use Case Cheat Sheet

| Use Case                | Pattern           | Code                                               |
| ----------------------- | ----------------- | -------------------------------------------------- |
| **Training Checkpoint** | Store with parent | `store_model(name, data, metadata, Some(parent))?` |
| **A/B Testing**         | Create branches   | `store_model(name, data_a, meta_a, Some(base))?`   |
| **Rollback**            | Load previous     | `get_model(name, Some(old_version))?`              |
| **Compare Versions**    | Get multiple      | `get_version(name, Some(v))` for each              |
| **Audit Trail**         | Get lineage       | `get_lineage(name, version)`                       |
| **Storage Cleanup**     | Delete old        | `cleanup_old_versions(name, keep_count)?`          |
| **Integrity Check**     | Verify checksum   | `verify_checksum(name, version, &data)`            |

---

## Workflow Templates

### Training Pipeline
```rust
// 1. Base model
let v1 = vault.store_model(name, base, meta_base, None)?;

// 2. Fine-tune
let v2 = vault.store_model(name, ft, meta_ft, Some(1))?;

// 3. Experiment (branch A)
let v3 = vault.store_model(name, exp_a, meta_a, Some(2))?;

// 4. Experiment (branch B)
let v4 = vault.store_model(name, exp_b, meta_b, Some(2))?;

// 5. Evaluate and pick winner
let winner = if eval(v3) > eval(v4) { 3 } else { 4 };

// 6. Continue from winner
let v5 = vault.store_model(name, final, meta_final, Some(winner))?;
```

### Production Deployment
```rust
// 1. Test latest version
let latest = vault.get_version(name, None).unwrap();
let data = vault.get_model(name, None)?;

// 2. Verify integrity
assert!(vault.verify_checksum(name, latest.version, &data));

// 3. Deploy
deploy(&data)?;

// 4. If issues, rollback
if production_issues() {
    let previous = latest.parent_version.unwrap();
    let rollback_data = vault.get_model(name, Some(previous))?;
    deploy(&rollback_data)?;
}
```

### Compliance Audit
```rust
// 1. Get all versions
let versions = vault.list_versions(name);

// 2. For each version, show lineage
for v in versions {
    println!("Version {}:", v.version);
    let lineage = vault.get_lineage(name, v.version);
    
    for ancestor in lineage {
        println!("  ← v{}: {}", 
            ancestor.version, 
            ancestor.timestamp.format("%Y-%m-%d"));
    }
}

// 3. Verify all checksums
for v in versions {
    let data = vault.get_model(name, Some(v.version))?;
    assert!(vault.verify_checksum(name, v.version, &data));
}
```

---

## Error Handling

```rust
// Safe version retrieval
match vault.get_model(name, Some(version)) {
    Ok(data) => {
        // Verify integrity
        if vault.verify_checksum(name, version, &data) {
            deploy(&data)?;
        } else {
            eprintln!("Checksum verification failed!");
        }
    }
    Err(e) => {
        eprintln!("Failed to retrieve version {}: {}", version, e);
        // Fallback to previous version
        let fallback = version - 1;
        let data = vault.get_model(name, Some(fallback))?;
        deploy(&data)?;
    }
}
```

---

## Performance Tips

1. **Batch Operations**: Load multiple versions in parallel
```rust
use rayon::prelude::*;

let versions: Vec<_> = (1..=5)
    .into_par_iter()
    .map(|v| vault.get_model(name, Some(v)))
    .collect();
```

2. **Metadata Queries**: Filter before loading data
```rust
let versions = vault.list_versions(name);
let rlhf = versions.into_iter()
    .filter(|v| v.metadata.contains_key("rlhf"))
    .collect::<Vec<_>>();
```

3. **Regular Cleanup**: Avoid version bloat
```rust
// Weekly maintenance
vault.cleanup_old_versions(name, 10)?;
```

4. **Checksum Caching**: Store validation results
```rust
let mut validated = HashMap::new();

for v in versions {
    if !validated.contains_key(&v.version) {
        let data = vault.get_model(name, Some(v.version))?;
        let valid = vault.verify_checksum(name, v.version, &data);
        validated.insert(v.version, valid);
    }
}
```

---

## CLI Integration

```bash
# List versions
iv list llama-2-7b-chat

# Get version info
iv info llama-2-7b-chat --version 3

# Get lineage
iv lineage llama-2-7b-chat --version 5

# Compare versions
iv compare llama-2-7b-chat --versions 3,5

# Cleanup
iv cleanup llama-2-7b-chat --keep 5

# Verify
iv verify llama-2-7b-chat --version 3
```

---

## Security Notes

- ✅ All versions encrypted at rest (ChaCha20-Poly1305)
- ✅ SHA-256 checksums for integrity
- ✅ Secure permission handling (0600 for files)
- ✅ FIPS-compliant cryptography
- ✅ Audit trail via version history
- ✅ Tamper detection via checksums

---

## See Also

- **Full Guide**: `docs/VERSION_CONTROL.md`
- **Examples**: `examples/version_control_demo.rs`
- **API Docs**: `src/version.rs`

---

**IronVault (AIMV)** - Git-like version control for AI models.
