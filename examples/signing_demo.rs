//! Example: Model signing and verification with HMAC-SHA256
//!
//! Run with: cargo run --example signing_demo

use ironvault::signing::ModelSigner;
use std::collections::HashMap;
use std::path::Path;

fn main() -> ironvault::Result<()> {
    println!("=== IronVault Signing Example ===\n");

    // 1. Generate a signing keypair
    println!("1. Generating signing keypair...");
    let keypair = ModelSigner::generate_keypair(Some("ML Team <ml@example.com>"))?;
    println!("   ✓ Identity: {:?}", keypair.identity);
    println!("   ✓ Public key: {}...", &keypair.public_key[..16]);
    println!("   ✓ Created at: {}\n", keypair.created_at);

    // 2. Save and load keypair
    println!("2. Key persistence...");
    let key_path = Path::new("example_signing_key.json");
    ModelSigner::save_keypair(&keypair, key_path)?;
    println!("   ✓ Saved to: {}", key_path.display());

    let loaded = ModelSigner::load_keypair(key_path)?;
    println!("   ✓ Loaded back, identity: {:?}\n", loaded.identity);

    // 3. Create a dummy file and sign it
    println!("3. Signing a model file...");
    let test_file = Path::new("example_model.bin");
    std::fs::write(test_file, b"example model data for signing demo")?;

    let mut metadata = HashMap::new();
    metadata.insert("purpose".to_string(), "demo".to_string());

    let signature = ModelSigner::sign(&keypair, test_file, metadata)?;
    println!("   ✓ Signature: {}...", &signature.signature[..32]);
    println!("   ✓ File SHA-256: {}...", &signature.file_sha256[..32]);
    println!("   ✓ Signer: {:?}\n", signature.signer);

    // 4. Save signature
    println!("4. Saving detached signature...");
    let sig_path = Path::new("example_model.sig");
    ModelSigner::save_signature(&signature, sig_path)?;
    println!("   ✓ Saved to: {}\n", sig_path.display());

    // 5. Verify
    println!("5. Verifying signature...");
    let loaded_sig = ModelSigner::load_signature(sig_path)?;
    // The key is required. Verifying without it can only re-hash the file and
    // compare against a hash the signature file itself supplies, which proves
    // nothing about who produced it.
    let verification = ModelSigner::verify(&loaded_sig, test_file, Some(&keypair.secret_seed))?;
    println!("   ✓ Valid: {}", verification.valid);
    println!("   ✓ File hash match: {}", verification.file_hash_match);
    println!("   ✓ Signature match: {}", verification.signature_match);

    // Without the key, authenticity is explicitly reported as unchecked.
    let unkeyed = ModelSigner::verify(&loaded_sig, test_file, None)?;
    println!("   ✓ Unkeyed verify: valid={}", unkeyed.valid);
    println!(
        "   ✓ Unkeyed verify: signature_checked={}\n",
        unkeyed.signature_checked
    );

    // Cleanup
    let _ = std::fs::remove_file(key_path);
    let _ = std::fs::remove_file(test_file);
    let _ = std::fs::remove_file(sig_path);

    println!("=== Signing example complete ===");
    Ok(())
}
