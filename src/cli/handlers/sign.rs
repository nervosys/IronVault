//! CLI handlers for model signing and verification (iv sign, iv verify).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ironvault::{kms, ModelSigner, Result, SigningKeyPair, VaultConfig, VaultError};

use crate::cli::helpers::{build_vault, prompt_passphrase};

/// Load a signing key pair from a KMS URI.
///
/// The stored secret may be either a full keypair JSON document or a bare
/// hex-encoded 32-byte seed.
fn keypair_from_kms(uri: &str, identity: Option<&str>) -> Result<SigningKeyPair> {
    println!("Loading signing key from KMS: {uri}");
    let secret = kms::resolve(uri)?;
    ModelSigner::parse_keypair(&secret, identity)
}

#[allow(clippy::too_many_arguments)]
pub fn handle_sign(
    name: String,
    version: Option<u32>,
    key: Option<String>,
    identity: Option<String>,
    file: Option<PathBuf>,
    config: VaultConfig,
    use_sqlite: bool,
) -> Result<()> {
    let id = identity.as_deref();

    // A KMS-backed key is fetched, never generated or written to disk.
    let keypair = match key.as_deref() {
        Some(k) if kms::is_kms_uri(k) => keypair_from_kms(k, id)?,
        other => {
            let key_path = other.map_or_else(
                || config.dirs.config_dir.join("signing_key.json"),
                PathBuf::from,
            );

            if key_path.exists() {
                println!("Loading signing key from: {}", key_path.display());
                ModelSigner::load_keypair(&key_path)?
            } else {
                println!("Generating new signing key pair...");
                let kp = ModelSigner::generate_keypair(id)?;
                ModelSigner::save_keypair(&kp, &key_path)?;
                println!("Key pair saved to: {}", key_path.display());
                kp
            }
        }
    };

    let metadata = HashMap::new();

    if let Some(file_path) = file {
        // Sign a file on disk
        let sig = ModelSigner::sign(&keypair, &file_path, metadata)?;
        let sig_path = file_path.with_extension("sig");
        ModelSigner::save_signature(&sig, &sig_path)?;
        println!("Signed: {}", file_path.display());
        println!("Signature: {}", sig_path.display());
        println!("SHA-256: {}", sig.file_sha256);
    } else {
        // Sign a model from the vault — we need to export it first
        let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
        let mut vault = build_vault(config, use_sqlite)?;
        vault.unlock(passphrase)?;

        let data = vault.get_model(&name, version)?;

        let temp_dir = tempfile::tempdir().map_err(VaultError::IoError)?;
        let temp_path = temp_dir.path().join(&name);
        std::fs::write(&temp_path, &data)?;

        let sig = ModelSigner::sign(&keypair, &temp_path, metadata)?;
        let sig_dir = vault.get_config().dirs.data_dir.clone();
        let sig_path = sig_dir.join(format!("{}_v{}.sig", name, version.unwrap_or(0)));
        ModelSigner::save_signature(&sig, &sig_path)?;

        println!("Signed model '{}' (v{})", name, version.unwrap_or(0));
        println!("Signature: {}", sig_path.display());
        println!("SHA-256: {}", sig.file_sha256);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn handle_verify(
    name: String,
    _version: Option<u32>,
    signature: PathBuf,
    key: Option<String>,
    file: Option<PathBuf>,
    config: VaultConfig,
    use_sqlite: bool,
) -> Result<()> {
    let sig = ModelSigner::load_signature(&signature)?;

    let _temp_dir;
    let file_path = if let Some(fp) = file {
        fp
    } else {
        // Export model from vault to temp
        let passphrase = prompt_passphrase("Enter vault passphrase: ")?;
        let mut vault = build_vault(config, use_sqlite)?;
        vault.unlock(passphrase)?;

        let data = vault.get_model(&name, None)?;
        _temp_dir = tempfile::tempdir().map_err(VaultError::IoError)?;
        let temp_path = _temp_dir.path().join(&name);
        std::fs::write(&temp_path, &data)?;
        temp_path
    };

    // Load secret key if provided (for HMAC verification)
    let secret_seed = match key.as_deref() {
        Some(k) if kms::is_kms_uri(k) => Some(keypair_from_kms(k, None)?.secret_seed),
        Some(k) => Some(ModelSigner::load_keypair(Path::new(k))?.secret_seed),
        None => None,
    };

    let result = ModelSigner::verify(&sig, &file_path, secret_seed.as_deref())?;

    if result.file_hash_match {
        println!("✓ File hash matches signature");
    } else {
        println!("✗ File hash does NOT match signature");
    }

    if !result.signature_checked {
        println!("? Cryptographic signature NOT CHECKED (no --key supplied)");
    } else if result.signature_match {
        println!("✓ Cryptographic signature valid");
    } else {
        println!("✗ Cryptographic signature INVALID");
    }

    if let Some(signer) = &sig.signer {
        println!("Signer: {}", signer);
    }
    println!("Signed at: {}", sig.signed_at);

    if result.valid {
        println!("\n✓ Verification PASSED");
        return Ok(());
    }

    println!("\n✗ Verification FAILED");
    // Fail the process too. `iv verify` is what a pipeline gates on; exiting 0
    // after printing FAILED means every non-interactive caller treats a
    // tampered or unverifiable model as good.
    Err(VaultError::IntegrityError(result.reason.unwrap_or_else(
        || "Signature verification failed".to_string(),
    )))
}
