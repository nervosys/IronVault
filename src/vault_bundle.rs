//! Vault export/import — portable encrypted vault bundles.
//!
//! Exports selected models (with all versions and metadata) to a self-contained
//! `.ivault` archive. Imports merge models back into a vault.

use std::collections::HashMap;
use std::fs;

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Result, VaultError};

/// Current bundle format version.
///
/// Bumped from 1 to 2 when `data_checksum` became reproducible. Version 1
/// folded blobs in whatever order the exporter happened to enumerate models,
/// which an importer holding a `HashMap` cannot reconstruct — the field was
/// written but never verified. Version 2 fixes a canonical order so both sides
/// agree.
const BUNDLE_FORMAT_VERSION: u32 = 2;

/// Digest over a bundle's blob payload, folded in sorted-name order.
///
/// Each blob contributes its name, a separator, its length, and its bytes.
/// Framing the length keeps two adjacent blobs from hashing the same as one
/// concatenated blob, and including the name means renaming a blob changes the
/// digest.
///
/// Note this is an integrity check, not an authenticity one: the digest lives
/// in the same archive as the data it covers, so anyone who can rewrite the
/// blobs can rewrite the digest. It catches truncation and corruption, not a
/// deliberately crafted bundle. Use `iv sign` / `iv verify` for provenance.
struct BlobDigest(sha2::Sha256);

impl BlobDigest {
    fn new() -> Self {
        use sha2::Digest;
        Self(sha2::Sha256::new())
    }

    /// Fold in one blob. Callers feed blobs in sorted-name order.
    fn update(&mut self, name: &str, data: &[u8]) {
        use sha2::Digest;
        self.0.update(name.as_bytes());
        self.0.update([0u8]);
        self.0.update((data.len() as u64).to_le_bytes());
        self.0.update(data);
    }

    fn finish(self) -> String {
        use sha2::Digest;
        hex::encode(self.0.finalize())
    }
}

/// Reject any blob path that is not a single, ordinary file name.
///
/// `ModelVersion::file_path` arrives from `versions.json` *inside* the bundle,
/// which is entirely attacker-controlled for any archive the user did not
/// produce themselves. `Path::join` discards its base when handed an absolute
/// path and walks upward on `..`, so using that value unvalidated turns the
/// import's `fs::copy` into an arbitrary-write primitive — `"../versions.json"`
/// alone is enough to overwrite the target vault's own version index.
///
/// Exported bundles always store blobs as a flat name directly under `data/`
/// (see `export_vault`), so demanding exactly that rejects nothing legitimate.
fn validate_blob_name(file_path: &str) -> Result<()> {
    let mut components = Path::new(file_path).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => Err(VaultError::InvalidInput(format!(
            "Refusing to import blob path {file_path:?}: bundle blob paths must be \
             a single file name, with no directory separators, parent references, \
             or drive/root prefix"
        ))),
    }
}

// ── Manifest ─────────────────────────────────────────────────────────────────

/// Bundle manifest stored inside the archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    /// Bundle format version
    pub format_version: u32,
    /// Vault name that produced this bundle
    pub source_vault: String,
    /// When the bundle was created
    pub created_at: String,
    /// Models included (name → list of versions)
    pub models: HashMap<String, Vec<u32>>,
    /// SHA-256 of the data section for integrity
    pub data_checksum: String,
}

// ── Export ────────────────────────────────────────────────────────────────────

/// Export selected models from a vault into a portable archive.
///
/// The output is a tar file containing:
///   - `manifest.json` — the bundle manifest
///   - `versions.json` — version metadata for included models
///   - `data/<uuid>.vault` — encrypted blobs
///   - `tags.json` — tag data for included models (if any)
pub fn export_vault(
    vault_path: &Path,
    output: &Path,
    model_filter: Option<&[String]>,
) -> Result<ExportReport> {
    use crate::version::VersionControl;

    let vc = VersionControl::new(vault_path)?;
    let all_models = vc.list_models_owned();

    let models_to_export: Vec<String> = if let Some(filter) = model_filter {
        // Support glob-like patterns (* → match any)
        all_models
            .into_iter()
            .filter(|m| {
                filter.iter().any(|pat| {
                    if pat.contains('*') {
                        let pat_lower = pat.to_lowercase().replace('*', "");
                        m.to_lowercase().contains(&pat_lower)
                    } else {
                        m == pat
                    }
                })
            })
            .collect()
    } else {
        all_models
    };

    if models_to_export.is_empty() {
        return Err(VaultError::InvalidInput(
            "No models matched the export filter".to_string(),
        ));
    }

    // Collect version data for selected models
    let mut version_data: HashMap<String, Vec<crate::version::ModelVersion>> = HashMap::new();
    let mut blob_files: Vec<String> = Vec::new();

    for model in &models_to_export {
        let versions: Vec<crate::version::ModelVersion> =
            vc.list_versions(model).into_iter().cloned().collect();
        for v in &versions {
            blob_files.push(v.file_path.clone());
        }
        version_data.insert(model.clone(), versions);
    }

    // Build manifest
    let models_summary: HashMap<String, Vec<u32>> = version_data
        .iter()
        .map(|(name, vers)| (name.clone(), vers.iter().map(|v| v.version).collect()))
        .collect();

    // Write tar archive
    let out_file = fs::File::create(output)?;
    let mut tar_builder = tar::Builder::new(out_file);

    // Write versions.json
    let versions_json = serde_json::to_string_pretty(&version_data)?;
    let versions_bytes = versions_json.as_bytes();
    let mut header = tar::Header::new_gnu();
    header.set_size(versions_bytes.len() as u64);
    header.set_mode(0o600);
    header.set_cksum();
    tar_builder.append_data(&mut header, "versions.json", versions_bytes)?;

    // Copy blob files. Sorted and deduplicated so the digest below is a
    // function of the content alone, not of enumeration order, and so two
    // versions sharing a blob write it once.
    blob_files.sort();
    blob_files.dedup();

    let data_dir = vault_path.join("data");
    let mut data_hash = BlobDigest::new();

    for blob in &blob_files {
        let blob_path = data_dir.join(blob);
        if blob_path.exists() {
            let blob_data = fs::read(&blob_path)?;
            data_hash.update(blob, &blob_data);

            let mut header = tar::Header::new_gnu();
            header.set_size(blob_data.len() as u64);
            header.set_mode(0o600);
            header.set_cksum();
            let archive_path = format!("data/{}", blob);
            tar_builder.append_data(&mut header, &archive_path, blob_data.as_slice())?;
        }
    }

    // Write tags (if present)
    let tags_path = vault_path.join("tags.json");
    if tags_path.exists() {
        let tags_data = fs::read(&tags_path)?;
        let mut header = tar::Header::new_gnu();
        header.set_size(tags_data.len() as u64);
        header.set_mode(0o600);
        header.set_cksum();
        tar_builder.append_data(&mut header, "tags.json", tags_data.as_slice())?;
    }

    let checksum = data_hash.finish();
    let manifest = BundleManifest {
        format_version: BUNDLE_FORMAT_VERSION,
        source_vault: vault_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        models: models_summary,
        data_checksum: checksum,
    };

    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    let manifest_bytes = manifest_json.as_bytes();
    let mut header = tar::Header::new_gnu();
    header.set_size(manifest_bytes.len() as u64);
    header.set_mode(0o600);
    header.set_cksum();
    tar_builder.append_data(&mut header, "manifest.json", manifest_bytes)?;

    tar_builder.finish()?;

    Ok(ExportReport {
        output_path: output.to_path_buf(),
        models_exported: manifest.models.keys().cloned().collect(),
        total_versions: manifest.models.values().map(|v| v.len()).sum(),
        total_blobs: blob_files.len(),
    })
}

/// Import models from a vault bundle archive.
pub fn import_vault(
    vault_path: &Path,
    archive_path: &Path,
    overwrite: bool,
) -> Result<ImportReport> {
    use crate::version::VersionControl;

    let file = fs::File::open(archive_path)?;
    let mut archive = tar::Archive::new(file);

    let temp_dir = tempfile::tempdir().map_err(VaultError::IoError)?;

    // Extract everything to temp dir
    archive.unpack(temp_dir.path())?;

    // Read manifest
    let manifest_path = temp_dir.path().join("manifest.json");
    if !manifest_path.exists() {
        return Err(VaultError::InvalidInput(
            "Invalid vault bundle: missing manifest.json".to_string(),
        ));
    }
    let manifest: BundleManifest = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;

    // Read exported versions
    let versions_path = temp_dir.path().join("versions.json");
    let imported_versions: HashMap<String, Vec<crate::version::ModelVersion>> =
        serde_json::from_str(&fs::read_to_string(&versions_path)?)?;

    // Reject a hostile bundle before touching the target vault, so a bad path
    // late in the archive cannot leave a half-merged vault behind.
    for versions in imported_versions.values() {
        for version in versions {
            validate_blob_name(&version.file_path)?;
        }
    }

    // Verify the payload digest, likewise before any mutation.
    let checksum_verified = if manifest.format_version >= 2 {
        let mut names: Vec<&str> = imported_versions
            .values()
            .flatten()
            .map(|v| v.file_path.as_str())
            .collect();
        names.sort_unstable();
        names.dedup();

        let mut digest = BlobDigest::new();
        for name in names {
            let path = temp_dir.path().join("data").join(name);
            if path.exists() {
                digest.update(name, &fs::read(&path)?);
            }
        }

        let actual = digest.finish();
        if actual != manifest.data_checksum {
            return Err(VaultError::IntegrityError(format!(
                "Vault bundle failed its integrity check: manifest declares \
                 {} but the archived blobs hash to {}. The bundle is truncated \
                 or corrupt; it has not been imported.",
                manifest.data_checksum, actual
            )));
        }
        true
    } else {
        // Version 1 wrote a digest whose input order cannot be reconstructed
        // here, so there is nothing to check against. Say so rather than
        // implying the bundle was verified.
        tracing::warn!(
            format_version = manifest.format_version,
            "vault bundle predates reproducible checksums; importing without \
             an integrity check"
        );
        false
    };

    // Merge into target vault
    let mut vc = VersionControl::new(vault_path)?;
    let existing_models: Vec<String> = vc.list_models_owned();
    let data_dir = vault_path.join("data");
    fs::create_dir_all(&data_dir)?;

    let mut models_imported = 0usize;
    let mut versions_imported = 0usize;
    let mut skipped = 0usize;

    for (model_name, versions) in &imported_versions {
        if existing_models.contains(model_name) && !overwrite {
            skipped += versions.len();
            continue;
        }
        models_imported += 1;

        for version in versions {
            // Copy blob. The path comes from the bundle, so it is checked
            // before it reaches the filesystem.
            validate_blob_name(&version.file_path)?;
            let src_blob = temp_dir.path().join("data").join(&version.file_path);
            let dst_blob = data_dir.join(&version.file_path);
            if src_blob.exists() {
                fs::copy(&src_blob, &dst_blob)?;
                crate::permissions::restrict_file(&dst_blob)?;
            }

            // Add version to VC
            vc.import_version(model_name, version.clone())?;
            versions_imported += 1;
        }
    }

    // Merge tags if present
    let imported_tags = temp_dir.path().join("tags.json");
    if imported_tags.exists() {
        let mut target_tags = crate::tags::TagStore::new(vault_path)?;
        let src_data: crate::tags::TagData =
            serde_json::from_str(&fs::read_to_string(&imported_tags)?)?;

        for (model, tags) in &src_data.tags {
            if imported_versions.contains_key(model) {
                let tag_vec: Vec<String> = tags.iter().cloned().collect();
                target_tags.add_tags(model, &tag_vec)?;
            }
        }
        for (model, annots) in &src_data.annotations {
            if imported_versions.contains_key(model) {
                for (k, v) in annots {
                    target_tags.set_annotation(model, k, v)?;
                }
            }
        }
    }

    Ok(ImportReport {
        source_vault: manifest.source_vault,
        models_imported,
        versions_imported,
        versions_skipped: skipped,
        checksum_verified,
    })
}

// ── Reports ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ExportReport {
    pub output_path: PathBuf,
    pub models_exported: Vec<String>,
    pub total_versions: usize,
    pub total_blobs: usize,
}

#[derive(Debug, Serialize)]
pub struct ImportReport {
    pub source_vault: String,
    pub models_imported: usize,
    pub versions_imported: usize,
    pub versions_skipped: usize,
    /// Whether the bundle's `data_checksum` was checked against its blobs.
    /// False for format-version-1 bundles, whose digest is not reproducible.
    pub checksum_verified: bool,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Blob path validation ─────────────────────────────────────────────

    /// `version.file_path` is read from `versions.json` inside the bundle, so
    /// it is attacker-controlled. `Path::join` drops its base for an absolute
    /// path and walks upward on `..`, which turned the import's `fs::copy`
    /// into an arbitrary write — `"../versions.json"` alone would overwrite
    /// the target vault's own version index.
    #[test]
    fn test_blob_names_that_escape_the_data_directory_are_rejected() {
        let hostile = [
            "../versions.json",
            "../../manifest.json",
            "../../../../../../etc/passwd",
            "a/b",
            "./x",
            "",
            ".",
            "..",
            #[cfg(windows)]
            r"C:\Windows\System32\drivers\etc\hosts",
            #[cfg(windows)]
            r"..\..\evil",
            #[cfg(not(windows))]
            "/etc/passwd",
            #[cfg(not(windows))]
            "/tmp/evil",
        ];

        for name in hostile {
            assert!(
                validate_blob_name(name).is_err(),
                "{name:?} escapes data/ and must be refused"
            );
        }
    }

    #[test]
    fn test_ordinary_blob_names_are_accepted() {
        for name in [
            "a1b2c3d4.vault",
            "model.bin",
            "0f8e7d6c-1234-5678-9abc-def012345678.vault",
            "name with spaces.vault",
        ] {
            assert!(
                validate_blob_name(name).is_ok(),
                "{name:?} is a normal exported blob name"
            );
        }
    }

    // ── Payload digest ───────────────────────────────────────────────────

    #[test]
    fn test_blob_digest_is_sensitive_to_content_name_and_framing() {
        let digest = |entries: &[(&str, &[u8])]| {
            let mut d = BlobDigest::new();
            for (n, b) in entries {
                d.update(n, b);
            }
            d.finish()
        };

        let base = digest(&[("a.vault", b"one"), ("b.vault", b"two")]);

        // Content change
        assert_ne!(base, digest(&[("a.vault", b"onE"), ("b.vault", b"two")]));
        // Rename
        assert_ne!(base, digest(&[("a.vault", b"one"), ("c.vault", b"two")]));
        // Re-splitting the same bytes across blobs must not collide: this is
        // what the length framing buys.
        assert_ne!(base, digest(&[("a.vault", b"onetwo"), ("b.vault", b"")]));
        // Identical input reproduces.
        assert_eq!(base, digest(&[("a.vault", b"one"), ("b.vault", b"two")]));
    }

    // ── End-to-end import hardening ──────────────────────────────────────

    /// Build a bundle tar by hand so the test can put values in `versions.json`
    /// that `export_vault` would never produce.
    fn craft_bundle(
        path: &Path,
        blob_file_path: &str,
        blobs: &[(&str, &[u8])],
        declared_checksum: Option<&str>,
        format_version: u32,
    ) {
        fn add(builder: &mut tar::Builder<fs::File>, name: &str, bytes: &[u8]) {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o600);
            header.set_cksum();
            builder.append_data(&mut header, name, bytes).unwrap();
        }

        let versions_json = format!(
            r#"{{"victim":[{{"version":1,"checkpoint_id":"c1",
               "timestamp":"2026-01-01T00:00:00Z","parent_version":null,
               "format":"Safetensors","size_bytes":3,"compressed_size_bytes":3,
               "checksum_sha256":"00","metadata":{{}},"file_path":{}}}]}}"#,
            serde_json::to_string(blob_file_path).unwrap()
        );

        // Default to the digest the importer will compute, so tests that are
        // not about the checksum get past it.
        let checksum = declared_checksum.map_or_else(
            || {
                let mut names: Vec<&str> = blobs.iter().map(|(n, _)| *n).collect();
                names.sort_unstable();
                names.dedup();
                let mut d = BlobDigest::new();
                for n in names {
                    if let Some((_, data)) = blobs.iter().find(|(bn, _)| bn == &n) {
                        d.update(n, data);
                    }
                }
                d.finish()
            },
            std::string::ToString::to_string,
        );

        let manifest = format!(
            r#"{{"format_version":{format_version},"source_vault":"evil",
               "created_at":"2026-01-01T00:00:00Z",
               "models":{{"victim":[1]}},"data_checksum":"{checksum}"}}"#
        );

        let mut builder = tar::Builder::new(fs::File::create(path).unwrap());
        add(&mut builder, "manifest.json", manifest.as_bytes());
        add(&mut builder, "versions.json", versions_json.as_bytes());
        for (name, data) in blobs {
            add(&mut builder, &format!("data/{name}"), data);
        }
        builder.finish().unwrap();
    }

    #[test]
    fn test_import_refuses_a_bundle_that_escapes_the_vault() {
        let tmp = tempfile::tempdir().unwrap();
        let vault = tmp.path().join("vault");
        fs::create_dir_all(vault.join("data")).unwrap();

        // A canary the traversal would clobber.
        let canary = vault.join("versions.json");
        fs::write(&canary, b"ORIGINAL").unwrap();

        let bundle = tmp.path().join("evil.ivault");
        craft_bundle(&bundle, "../versions.json", &[], None, 2);

        let err = import_vault(&vault, &bundle, true).unwrap_err();
        assert!(
            err.to_string().contains("single file name"),
            "expected a path-validation refusal, got: {err}"
        );
        assert_eq!(
            fs::read(&canary).unwrap(),
            b"ORIGINAL",
            "the vault's own version index must be untouched"
        );
    }

    #[test]
    fn test_import_rejects_a_bundle_whose_blobs_do_not_match_its_checksum() {
        let tmp = tempfile::tempdir().unwrap();
        let vault = tmp.path().join("vault");
        fs::create_dir_all(vault.join("data")).unwrap();

        let bundle = tmp.path().join("corrupt.ivault");
        craft_bundle(
            &bundle,
            "blob.vault",
            &[("blob.vault", b"actual bytes")],
            Some(&"0".repeat(64)),
            2,
        );

        let err = import_vault(&vault, &bundle, true).unwrap_err();
        assert!(
            err.to_string().contains("integrity check"),
            "expected an integrity failure, got: {err}"
        );
        assert!(
            !vault.join("data").join("blob.vault").exists(),
            "nothing may be written from a bundle that failed verification"
        );
    }

    #[test]
    fn test_import_accepts_a_well_formed_bundle_and_reports_verification() {
        let tmp = tempfile::tempdir().unwrap();
        let vault = tmp.path().join("vault");
        fs::create_dir_all(vault.join("data")).unwrap();

        let bundle = tmp.path().join("good.ivault");
        craft_bundle(
            &bundle,
            "blob.vault",
            &[("blob.vault", b"actual bytes")],
            None,
            2,
        );

        let report = import_vault(&vault, &bundle, true).unwrap();
        assert!(report.checksum_verified);
        assert_eq!(report.versions_imported, 1);
        assert_eq!(
            fs::read(vault.join("data").join("blob.vault")).unwrap(),
            b"actual bytes"
        );
    }

    /// A version-1 bundle carries a digest that cannot be recomputed here. It
    /// still imports, but must not claim to have been verified.
    #[test]
    fn test_legacy_bundle_imports_without_claiming_verification() {
        let tmp = tempfile::tempdir().unwrap();
        let vault = tmp.path().join("vault");
        fs::create_dir_all(vault.join("data")).unwrap();

        let bundle = tmp.path().join("legacy.ivault");
        craft_bundle(
            &bundle,
            "blob.vault",
            &[("blob.vault", b"actual bytes")],
            Some("whatever-v1-wrote"),
            1,
        );

        let report = import_vault(&vault, &bundle, true).unwrap();
        assert!(!report.checksum_verified);
        assert_eq!(report.versions_imported, 1);
    }

    #[test]
    fn test_bundle_manifest_roundtrip() {
        let manifest = BundleManifest {
            format_version: 1,
            source_vault: "test".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            models: {
                let mut m = HashMap::new();
                m.insert("llama".to_string(), vec![1, 2, 3]);
                m
            },
            data_checksum: "abc123".to_string(),
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: BundleManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.format_version, 1);
        assert_eq!(parsed.models.get("llama").unwrap().len(), 3);
    }

    #[test]
    fn test_export_report_fields() {
        let report = ExportReport {
            output_path: PathBuf::from("/tmp/test.ivault"),
            models_exported: vec!["m1".into(), "m2".into()],
            total_versions: 5,
            total_blobs: 5,
        };
        assert_eq!(report.models_exported.len(), 2);
        assert_eq!(report.total_versions, 5);
    }
}
