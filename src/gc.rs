//! Garbage collection — find and remove orphaned blobs, temp files, and
//! unreferenced data inside a vault directory.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::Result;
use crate::version::VersionControl;

// ── Types ────────────────────────────────────────────────────────────────────

/// Report produced by a GC run.
#[derive(Debug, Clone, Serialize)]
pub struct GcReport {
    /// Blob files referenced by at least one version.
    pub referenced_blobs: usize,
    /// Blob files on disk that are NOT referenced.
    pub orphaned_blobs: Vec<PathBuf>,
    /// Temporary files discovered (e.g. `.tmp`, `.part`).
    pub temp_files: Vec<PathBuf>,
    /// Total bytes that would be freed.
    pub reclaimable_bytes: u64,
    /// Whether the orphans were actually deleted.
    pub deleted: bool,
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Scan a vault directory for orphaned blobs and temp files.
///
/// When `dry_run` is true the report is produced but nothing is deleted.
pub fn gc(vault_path: &Path, dry_run: bool, key: &crate::crypto::SecureKey) -> Result<GcReport> {
    let mut vc = VersionControl::new(vault_path)?;
    // The key is not optional here and must not become so. A locked index
    // reads as empty, every blob would then be unreferenced, and this function
    // deletes unreferenced blobs -- so running it without the key would delete
    // every model in the vault.
    vc.unlock(key)?;

    // 1. Collect every file_path referenced by any version.
    let models = vc.list_models_owned();
    let mut referenced: BTreeSet<String> = BTreeSet::new();
    for model in &models {
        for ver in vc.list_versions(model) {
            referenced.insert(ver.file_path.clone());
        }
    }

    // 2. Walk the `data/` directory.
    let data_dir = vault_path.join("data");
    let mut orphaned: Vec<PathBuf> = Vec::new();
    let mut temp_files: Vec<PathBuf> = Vec::new();
    let mut reclaimable: u64 = 0;

    if data_dir.is_dir() {
        for entry in fs::read_dir(&data_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();

            // Temp files
            if file_name.ends_with(".tmp") || file_name.ends_with(".part") {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                reclaimable += size;
                temp_files.push(path);
                continue;
            }

            // Check if referenced
            if !referenced.contains(&file_name) {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                reclaimable += size;
                orphaned.push(path);
            }
        }
    }

    let deleted = if !dry_run {
        for p in orphaned.iter().chain(temp_files.iter()) {
            let _ = fs::remove_file(p);
        }
        true
    } else {
        false
    };

    Ok(GcReport {
        referenced_blobs: referenced.len(),
        orphaned_blobs: orphaned,
        temp_files,
        reclaimable_bytes: reclaimable,
        deleted,
    })
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// A key for the sealed index. gc refuses to run without one, which is
    /// the point: a locked index reads empty and every blob looks orphaned.
    fn test_key() -> crate::crypto::SecureKey {
        crate::crypto::VaultCrypto::new()
            .unwrap()
            .derive_key(b"gc-test-passphrase".to_vec(), Some(vec![7u8; 16]))
            .unwrap()
            .0
    }

    #[test]
    fn test_gc_empty_vault() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("data")).unwrap();
        // Create a version control file so VersionControl::new() succeeds
        let report = gc(dir.path(), true, &test_key()).unwrap();
        assert_eq!(report.orphaned_blobs.len(), 0);
        assert_eq!(report.temp_files.len(), 0);
        assert!(!report.deleted);
    }

    #[test]
    fn test_gc_finds_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("abc.tmp"), b"temp").unwrap();
        fs::write(data.join("xyz.part"), b"partial").unwrap();

        let report = gc(dir.path(), true, &test_key()).unwrap();
        assert_eq!(report.temp_files.len(), 2);
        assert_eq!(report.reclaimable_bytes, 11); // 4 + 7
        assert!(!report.deleted);
    }

    #[test]
    fn test_gc_deletes_when_not_dry() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("orphan.vault"), b"stale").unwrap();

        let report = gc(dir.path(), false, &test_key()).unwrap();
        assert_eq!(report.orphaned_blobs.len(), 1);
        assert!(report.deleted);
        // File should be gone
        assert!(!data.join("orphan.vault").exists());
    }
}
