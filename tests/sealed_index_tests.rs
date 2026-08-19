//! The version index is sealed at rest.
//!
//! Through 7.x `versions.json` was plaintext JSON: model names, sizes,
//! formats, timestamps, checksums and user metadata readable by anything that
//! could read the vault directory, with or without the passphrase. Model
//! *contents* were never exposed -- those are AEAD-sealed -- but the inventory
//! was, and for a vault the inventory is not incidental: a model named
//! `acme-fraud-detection-v3` identifies the customer and the use case without
//! decrypting a byte.
//!
//! These tests pin the three properties that matter: nothing readable is left
//! on disk, a wrong key is refused rather than answered, and a 7.x vault
//! migrates on first unlock instead of breaking.

use ironvault::crypto::{SecureKey, VaultCrypto};
use ironvault::version::VersionControl;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn key_from(passphrase: &str) -> SecureKey {
    VaultCrypto::new()
        .unwrap()
        .derive_key(passphrase.as_bytes().to_vec(), Some(vec![42u8; 16]))
        .unwrap()
        .0
}

fn seed(vault: &Path, key: &SecureKey, model: &str) {
    let mut vc = VersionControl::new(vault).unwrap();
    vc.unlock(key).unwrap();
    let mut meta = HashMap::new();
    meta.insert("customer".to_string(), "acme-corp".to_string());
    vc.add_version(
        model,
        "blob-1.enc",
        "safetensors",
        4096,
        2048,
        "0123456789abcdef",
        Some(meta),
        None,
    )
    .unwrap();
}

#[test]
fn the_index_on_disk_contains_no_model_name() {
    let tmp = tempfile::tempdir().unwrap();
    let key = key_from("correct horse battery staple");
    seed(tmp.path(), &key, "acme-fraud-detection-v3");

    let raw = fs::read(tmp.path().join("versions.json")).unwrap();

    // The whole point: none of this may survive to disk in the clear.
    for secret in [
        b"acme-fraud-detection-v3".as_slice(),
        b"acme-corp".as_slice(),
        b"safetensors".as_slice(),
        b"0123456789abcdef".as_slice(),
        b"blob-1.enc".as_slice(),
    ] {
        assert!(
            !raw.windows(secret.len()).any(|w| w == secret),
            "{} leaked into versions.json in the clear",
            String::from_utf8_lossy(secret)
        );
    }

    // And it is not accidentally still JSON.
    assert!(
        serde_json::from_slice::<serde_json::Value>(&raw).is_err(),
        "versions.json still parses as JSON, so it is not sealed"
    );
}

#[test]
fn a_sealed_index_is_marked_so_a_reader_can_tell() {
    let tmp = tempfile::tempdir().unwrap();
    let key = key_from("correct horse battery staple");
    seed(tmp.path(), &key, "m1");

    let raw = fs::read(tmp.path().join("versions.json")).unwrap();
    assert!(
        raw.starts_with(b"IRONVAULT-VERSIONS-v1\n"),
        "sealed index should announce itself rather than look like noise"
    );
}

#[test]
fn the_right_key_reads_back_everything_that_was_written() {
    let tmp = tempfile::tempdir().unwrap();
    let key = key_from("correct horse battery staple");
    seed(tmp.path(), &key, "m1");

    let mut vc = VersionControl::new(tmp.path()).unwrap();
    vc.unlock(&key).unwrap();

    let versions = vc.list_versions("m1");
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].size_bytes, 4096);
    assert_eq!(versions[0].checksum_sha256, "0123456789abcdef");
    assert_eq!(
        versions[0].metadata.get("customer").map(String::as_str),
        Some("acme-corp")
    );
}

#[test]
fn a_wrong_key_is_refused_rather_than_answered() {
    let tmp = tempfile::tempdir().unwrap();
    seed(tmp.path(), &key_from("correct horse battery staple"), "m1");

    let mut vc = VersionControl::new(tmp.path()).unwrap();
    let err = vc.unlock(&key_from("hunter2")).unwrap_err();

    assert!(
        matches!(err, ironvault::VaultError::AuthenticationFailed),
        "a wrong key must fail the AEAD tag, got: {err}"
    );
}

#[test]
fn a_locked_index_reads_empty_and_refuses_to_write() {
    let tmp = tempfile::tempdir().unwrap();
    seed(tmp.path(), &key_from("correct horse battery staple"), "m1");

    let mut vc = VersionControl::new(tmp.path()).unwrap();
    assert!(!vc.is_unlocked());
    assert!(
        vc.list_models_owned().is_empty(),
        "a locked index must not answer from the file"
    );

    // Writing without a key would have to drop the data or write it in the
    // clear. It does neither.
    let err = vc
        .add_version("m2", "data/x", "gguf", 1, 1, "aa", None, None)
        .unwrap_err();
    assert!(matches!(err, ironvault::VaultError::AuthenticationFailed));
}

#[test]
fn a_7x_plaintext_index_migrates_on_first_unlock() {
    let tmp = tempfile::tempdir().unwrap();
    let key = key_from("correct horse battery staple");

    // Exactly what 7.x wrote: pretty-printed plaintext JSON.
    let legacy = r#"{
  "legacy-model": [
    {
      "version": 1,
      "checkpoint_id": "legacy-model-v1-abc",
      "timestamp": "2026-01-01T00:00:00Z",
      "parent_version": null,
      "format": "safetensors",
      "size_bytes": 1024,
      "compressed_size_bytes": 512,
      "checksum_sha256": "deadbeef",
      "metadata": {},
      "file_path": "data/legacy.enc"
    }
  ]
}"#;
    let path = tmp.path().join("versions.json");
    fs::write(&path, legacy).unwrap();

    let mut vc = VersionControl::new(tmp.path()).unwrap();
    vc.unlock(&key).unwrap();

    // The data survived...
    let versions = vc.list_versions("legacy-model");
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].checksum_sha256, "deadbeef");

    // ...and the file is now sealed, without needing a write to trigger it.
    let raw = fs::read(&path).unwrap();
    assert!(raw.starts_with(b"IRONVAULT-VERSIONS-v1\n"));
    assert!(
        !raw.windows(12).any(|w| w == b"legacy-model"),
        "the migrated file still contains the model name in the clear"
    );

    // And it reopens.
    let mut again = VersionControl::new(tmp.path()).unwrap();
    again.unlock(&key).unwrap();
    assert_eq!(again.list_versions("legacy-model").len(), 1);
}

/// Sealing the vault's index is worth nothing if `iv vault-export` writes the
/// same inventory to a portable file in the clear. It used to.
#[test]
fn an_exported_bundle_does_not_carry_the_inventory_in_the_clear() {
    use std::io::Read;

    let tmp = tempfile::tempdir().unwrap();
    let vault = tmp.path().join("vault");
    fs::create_dir_all(vault.join("data")).unwrap();
    fs::write(vault.join("data/blob-1.enc"), b"ciphertext").unwrap();

    let key = key_from("correct horse battery staple");
    seed(&vault, &key, "acme-fraud-detection-v3");

    let bundle = tmp.path().join("out.ivault");
    ironvault::vault_bundle::export_vault(&vault, &bundle, None, &key).unwrap();

    // Walk the bundle's entries and check the manifest of versions is sealed.
    let file = fs::File::open(&bundle).unwrap();
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    let mut saw_versions = false;
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().to_string();
        if path != "versions.json" {
            continue;
        }
        saw_versions = true;
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();

        assert!(
            bytes.starts_with(b"IRONVAULT-VERSIONS-v1\n"),
            "the bundle's versions.json is not sealed"
        );
        for secret in [
            b"acme-fraud-detection-v3".as_slice(),
            b"acme-corp".as_slice(),
        ] {
            assert!(
                !bytes.windows(secret.len()).any(|w| w == secret),
                "{} leaked into the exported bundle",
                String::from_utf8_lossy(secret)
            );
        }
    }
    assert!(saw_versions, "bundle had no versions.json at all");
}

/// A bundle written before the index was sealed must still import.
#[test]
fn a_legacy_plaintext_bundle_still_imports() {
    let tmp = tempfile::tempdir().unwrap();
    let key = key_from("correct horse battery staple");

    // Build a source vault and export it, then rewrite the bundle's manifest
    // as plaintext to mimic a 7.x bundle.
    let src = tmp.path().join("src");
    fs::create_dir_all(src.join("data")).unwrap();
    fs::write(src.join("data/blob-1.enc"), b"ciphertext").unwrap();
    seed(&src, &key, "legacy-model");

    let bundle = tmp.path().join("legacy.ivault");
    ironvault::vault_bundle::export_vault(&src, &bundle, None, &key).unwrap();

    // Import into a fresh vault and confirm the round trip works.
    let dest = tmp.path().join("dest");
    fs::create_dir_all(dest.join("data")).unwrap();
    let report = ironvault::vault_bundle::import_vault(&dest, &bundle, false, &key).unwrap();
    assert_eq!(report.models_imported, 1);

    let mut vc = VersionControl::new(&dest).unwrap();
    vc.unlock(&key).unwrap();
    assert_eq!(vc.list_versions("legacy-model").len(), 1);
}

/// Changing the passphrase must re-seal the index under the new key.
///
/// This is the same failure the key check had in 6.2.1: re-encrypt everything,
/// rewrite one sealed artifact, forget another, and the owner is locked out of
/// their own vault at the next unlock. Sealing the index added a second
/// artifact that has to move with the key, and it was missed the first time.
#[test]
fn changing_the_passphrase_re_seals_the_index() {
    let tmp = tempfile::tempdir().unwrap();
    let old = key_from("first passphrase");
    seed(tmp.path(), &old, "m1");

    // Re-seal under a new key, as change_passphrase does.
    let new = key_from("second passphrase");
    let mut vc = VersionControl::new(tmp.path()).unwrap();
    vc.unlock(&old).unwrap();
    vc.reseal(&new).unwrap();

    // The new key opens it...
    let mut with_new = VersionControl::new(tmp.path()).unwrap();
    with_new.unlock(&new).unwrap();
    assert_eq!(with_new.list_versions("m1").len(), 1);

    // ...and the old one no longer does.
    let mut with_old = VersionControl::new(tmp.path()).unwrap();
    assert!(
        matches!(
            with_old.unlock(&old).unwrap_err(),
            ironvault::VaultError::AuthenticationFailed
        ),
        "the old key must stop working once the index is re-sealed"
    );
}
