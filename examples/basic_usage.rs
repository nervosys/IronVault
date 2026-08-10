//! Example: Basic usage of IronVault

use ironvault::formats::{ModelFormat, ModelMetadata};
use ironvault::{VaultBuilder, VaultConfig};

fn main() -> ironvault::Result<()> {
    println!("=== IronVault Basic Example ===\n");

    // 1. Create and configure vault (using VaultBuilder)
    println!("1. Creating vault with VaultBuilder...");
    let config = VaultConfig::new()?;
    let mut vault = VaultBuilder::new()
        .config(config)
        // .sqlite_versions()        // uncomment with `sqlite` feature
        // .no_default_subscribers()  // opt out of built-in audit + metrics
        .build()?;
    println!(
        "   ✓ Vault created at: {:?}",
        vault.get_config().dirs.vault_dir
    );
    println!("   ✓ Version backend: {}", vault.version_backend_name());
    println!(
        "   ✓ Event subscribers: {}\n",
        vault.event_bus().subscriber_count()
    );

    // 2. Unlock vault with passphrase
    println!("2. Unlocking vault...");
    let passphrase = b"my_super_secure_passphrase_2024";
    vault.unlock(passphrase.to_vec())?;
    println!("   ✓ Vault unlocked\n");

    // 3. Create sample model data
    println!("3. Creating sample model data...");
    let model_data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
    println!("   Model size: {} bytes\n", model_data.len());

    // 4. Create metadata
    println!("4. Creating model metadata...");
    let metadata = ModelMetadata::new("my-first-model".to_string(), ModelFormat::PyTorch)
        .with_description("Example PyTorch model for demonstration".to_string())
        .with_framework("PyTorch 2.0".to_string())
        .with_task("image-classification".to_string())
        .with_architecture("ResNet-50".to_string())
        .with_parameters(25_500_000)
        .add_custom_field("dataset".to_string(), "ImageNet".to_string())
        .add_custom_field("accuracy".to_string(), "94.5%".to_string());

    println!("   ✓ Metadata created\n");

    // 5. Store model in vault
    println!("5. Storing model in vault...");
    let version = vault.store_model("my-first-model", model_data.clone(), metadata, None)?;

    println!("   ✓ Model stored successfully!");
    println!("     Version: {}", version.version);
    println!("     Checkpoint ID: {}", version.checkpoint_id);
    println!("     Original size: {} bytes", version.size_bytes);
    println!(
        "     Compressed size: {} bytes",
        version.compressed_size_bytes
    );
    println!(
        "     Compression ratio: {:.1}%",
        (1.0 - version.compressed_size_bytes as f64 / version.size_bytes as f64) * 100.0
    );
    println!("     Format: {}", version.format);
    println!("     Checksum: {}\n", version.checksum_sha256);

    // 6. Store another version
    println!("6. Storing updated version...");
    let updated_data: Vec<u8> = (0..1200).map(|i| ((i * 2) % 256) as u8).collect();
    let metadata_v2 = ModelMetadata::new("my-first-model".to_string(), ModelFormat::PyTorch)
        .with_description("Updated model with better accuracy".to_string())
        .with_framework("PyTorch 2.1".to_string())
        .add_custom_field("accuracy".to_string(), "95.2%".to_string());

    let version2 = vault.store_model(
        "my-first-model",
        updated_data,
        metadata_v2,
        Some(1), // Parent version
    )?;

    println!(
        "   ✓ Version 2 stored (parent: v{})\n",
        version2.parent_version.unwrap()
    );

    // 7. List all models
    println!("7. Listing all models in vault...");
    let models = vault.list_models();
    println!("   Found {} model(s):", models.len());
    for model_name in &models {
        println!("     - {}", model_name);
    }
    println!();

    // 8. List versions of a model
    println!("8. Listing versions of 'my-first-model'...");
    let versions = vault.list_versions("my-first-model");
    println!("   Found {} version(s):", versions.len());
    for v in &versions {
        println!(
            "     - v{}: {} bytes ({})",
            v.version,
            v.size_bytes,
            v.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );
    }
    println!();

    // 9. Get lineage
    println!("9. Getting lineage for version 2...");
    let lineage = vault.get_lineage("my-first-model", 2);
    println!("   Lineage:");
    for (i, v) in lineage.iter().enumerate() {
        println!(
            "     {}v{} -> checkpoint {}",
            "  ".repeat(i),
            v.version,
            v.checkpoint_id
        );
    }
    println!();

    // 10. Retrieve specific version
    println!("10. Retrieving version 1...");
    let retrieved_data = vault.get_model("my-first-model", Some(1))?;
    println!("    ✓ Retrieved {} bytes", retrieved_data.len());
    println!(
        "    Data matches original: {}\n",
        retrieved_data == model_data
    );

    // 11. Retrieve latest version
    println!("11. Retrieving latest version...");
    let latest_data = vault.get_model("my-first-model", None)?;
    println!(
        "    ✓ Retrieved {} bytes (latest version)\n",
        latest_data.len()
    );

    // 12. Get vault statistics
    println!("12. Vault statistics:");
    let stats = vault.get_stats()?;
    println!("    Models: {}", stats.model_count);
    println!("    Total versions: {}", stats.total_versions);
    println!(
        "    Total size: {} bytes ({:.2} KB)",
        stats.total_size_bytes,
        stats.total_size_bytes as f64 / 1024.0
    );
    println!("    Files: {}\n", stats.file_count);

    // 13. Delete a version
    println!("13. Deleting version 1...");
    let deleted = vault.delete_version("my-first-model", 1)?;
    if deleted {
        println!("    ✓ Version 1 deleted successfully\n");
    }

    // 14. Verify deletion
    println!("14. Verifying deletion...");
    let remaining_versions = vault.list_versions("my-first-model");
    println!("    Remaining versions: {}", remaining_versions.len());
    for v in &remaining_versions {
        println!("      - v{}", v.version);
    }

    println!("\n=== Example completed successfully! ===");

    Ok(())
}
