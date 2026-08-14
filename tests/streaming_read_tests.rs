//! `Vault::read_model_into` must be interchangeable with `Vault::get_model`.
//!
//! The streaming read exists so an inference engine can decrypt a model straight
//! into one page-locked buffer instead of holding ~3× the model in RAM. That is
//! only worth having if it produces *exactly* the same bytes: a divergence here
//! would not crash, it would produce weights that still decode into fluent text.
//! So every test below compares the two paths rather than asserting on the
//! streaming one alone.

use ironvault::config::{DirectoryPaths, VaultConfig};
use ironvault::error::VaultError;
use ironvault::formats::{ModelFormat, ModelMetadata};
use ironvault::vault::Vault;

const PASSPHRASE: &[u8] = b"streaming_read_integration_passphrase";

/// A vault in a temp dir, unlocked, with the given compression algorithm.
fn test_vault(dir: &tempfile::TempDir, compression: &str) -> Vault {
    let dirs = DirectoryPaths {
        config_dir: dir.path().join("config"),
        data_dir: dir.path().join("data"),
        cache_dir: dir.path().join("cache"),
        vault_dir: dir.path().join("data/vaults/default"),
        log_dir: dir.path().join("data/logs"),
        backends_dir: dir.path().join("config/backends"),
        utilities_dir: dir.path().join("config/utilities"),
        databases_dir: dir.path().join("config/databases"),
    };
    let mut config = VaultConfig::with_dirs(dirs).unwrap();
    config.compression.algorithm = compression.to_string();

    let mut vault = Vault::new(Some(config)).unwrap();
    vault.unlock(PASSPHRASE.to_vec()).unwrap();
    vault
}

fn metadata(name: &str) -> ModelMetadata {
    ModelMetadata {
        name: name.to_string(),
        format: ModelFormat::GGUF,
        description: None,
        framework: None,
        task: None,
        architecture: None,
        parameters: None,
        custom_fields: std::collections::HashMap::new(),
    }
}

/// Weights-like bytes: large enough to span several 4 MiB chunks, and not a
/// multiple of the chunk size, so the short final chunk is always exercised.
fn payload(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i.wrapping_mul(31) % 251) as u8).collect()
}

fn roundtrip_matches(compression: &str, len: usize) {
    let dir = tempfile::tempdir().unwrap();
    let mut vault = test_vault(&dir, compression);

    let data = payload(len);
    vault
        .store_model("m", data.clone(), metadata("m"), None)
        .unwrap();

    let buffered = vault.get_model("m", None).unwrap();
    assert_eq!(buffered, data, "{compression}: get_model must round-trip");

    let declared = vault.model_plaintext_len("m", None).unwrap();
    assert_eq!(
        declared as usize,
        data.len(),
        "{compression}: declared size"
    );

    let mut dst = vec![0u8; declared as usize];
    let written = vault.read_model_into("m", None, &mut dst).unwrap();

    assert_eq!(written, data.len(), "{compression}: bytes written");
    assert_eq!(
        dst, buffered,
        "{compression}: streaming and buffered reads must agree exactly"
    );
}

#[test]
fn streaming_matches_buffered_without_compression() {
    // The case that matters for inference: quantized weights do not compress,
    // so this is how models should actually be stored.
    roundtrip_matches("none", 9 * 1024 * 1024 + 12345);
}

#[test]
fn streaming_matches_buffered_with_gzip() {
    // The default. Gzip is applied to the whole model BEFORE chunked encryption,
    // so the streaming path has to decompress across chunk boundaries.
    roundtrip_matches("gzip", 9 * 1024 * 1024 + 12345);
}

#[test]
fn streaming_matches_buffered_with_lzma() {
    // Smaller: lzma_rs is slow enough that a 9 MiB payload dominates the suite.
    roundtrip_matches("lzma", 512 * 1024 + 77);
}

#[test]
fn streaming_matches_buffered_for_a_tiny_model() {
    roundtrip_matches("none", 3);
}

#[test]
fn an_empty_model_streams() {
    roundtrip_matches("none", 0);
}

#[test]
fn a_wrong_sized_destination_is_refused_naming_both_sizes() {
    let dir = tempfile::tempdir().unwrap();
    let mut vault = test_vault(&dir, "none");
    let data = payload(4096);
    vault.store_model("m", data, metadata("m"), None).unwrap();

    // Too small.
    let mut short = vec![0u8; 4095];
    let err = vault.read_model_into("m", None, &mut short).unwrap_err();
    assert!(matches!(err, VaultError::InvalidInput(_)), "got {err:?}");
    let msg = err.to_string();
    assert!(msg.contains("4095"), "must name what it GOT: {msg}");
    assert!(msg.contains("4096"), "must name what it needed: {msg}");

    // Too large is equally wrong: it would leave a tail of zeros that reads as
    // valid weights.
    let mut long = vec![0u8; 4097];
    assert!(matches!(
        vault.read_model_into("m", None, &mut long).unwrap_err(),
        VaultError::InvalidInput(_)
    ));
}

#[test]
fn a_locked_vault_refuses_to_stream() {
    let dir = tempfile::tempdir().unwrap();
    let mut vault = test_vault(&dir, "none");
    vault
        .store_model("m", payload(1024), metadata("m"), None)
        .unwrap();
    let len = vault.model_plaintext_len("m", None).unwrap() as usize;

    vault.lock();

    let mut dst = vec![0u8; len];
    let err = vault.read_model_into("m", None, &mut dst).unwrap_err();
    assert!(
        matches!(err, VaultError::SecurityViolation(_)),
        "got {err:?}"
    );
    assert!(dst.iter().all(|&b| b == 0), "nothing may be written");
}

#[test]
fn a_missing_model_is_reported_before_anything_is_allocated() {
    let dir = tempfile::tempdir().unwrap();
    let vault = test_vault(&dir, "none");
    assert!(matches!(
        vault.model_plaintext_len("nope", None).unwrap_err(),
        VaultError::ModelNotFound(_)
    ));
}

/// The biggest regular file anywhere under `root` — the stored ciphertext,
/// whatever the vault decided to call it or where it put it.
fn largest_file_under(root: &std::path::Path) -> Option<std::path::PathBuf> {
    let mut best: Option<(u64, std::path::PathBuf)> = None;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(meta) = entry.metadata() {
                if best.as_ref().is_none_or(|(len, _)| meta.len() > *len) {
                    best = Some((meta.len(), path));
                }
            }
        }
    }
    best.map(|(_, path)| path)
}

/// A model whose ciphertext has been tampered with must not leave readable
/// weights in the destination. A caller that ignores the error would otherwise
/// run on half-decrypted data — which, for an LLM, still produces fluent text.
#[test]
fn a_corrupted_model_leaves_the_destination_zeroed() {
    let dir = tempfile::tempdir().unwrap();
    let mut vault = test_vault(&dir, "none");

    let data = payload(5 * 1024 * 1024 + 3);
    vault
        .store_model("m", data.clone(), metadata("m"), None)
        .unwrap();

    // Corrupt a byte deep in the stored ciphertext, past the first chunk, so
    // the early chunks decrypt cleanly and only a later one fails. The vault
    // chooses its own filename and layout, so find the file by size rather than
    // by guessing a path.
    let stored = largest_file_under(dir.path()).expect("a stored ciphertext file");

    let mut bytes = std::fs::read(&stored).unwrap();
    let target = bytes.len() - 64;
    bytes[target] ^= 0xFF;
    std::fs::write(&stored, &bytes).unwrap();

    let mut dst = vec![0u8; data.len()];
    let err = vault
        .read_model_into("m", None, &mut dst)
        .expect_err("a corrupted model must not read cleanly");
    // A tampered byte inside a chunk fails that chunk's GCM tag, which surfaces
    // as AuthenticationFailed. Corruption of the framing instead trips the
    // stream MAC (IntegrityError). Either is correct; silently succeeding is not.
    assert!(
        matches!(
            err,
            VaultError::AuthenticationFailed
                | VaultError::IntegrityError(_)
                | VaultError::CryptoError(_)
        ),
        "got {err:?}"
    );
    assert!(
        dst.iter().all(|&b| b == 0),
        "a failed read must not leave partially decrypted weights behind"
    );
}
