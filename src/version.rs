//! Version control system for model checkpoints
//!
//! Maintains complete history of model versions with metadata and generations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::crypto::FipsCrypto;
use crate::error::Result;

/// Represents a single model version/checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersion {
    /// Version number (sequential)
    pub version: u32,

    /// Unique checkpoint identifier
    pub checkpoint_id: String,

    /// Timestamp when version was created
    pub timestamp: DateTime<Utc>,

    /// Parent version for branching/lineage
    pub parent_version: Option<u32>,

    /// Model format
    pub format: String,

    /// Original size in bytes
    pub size_bytes: u64,

    /// Compressed size in bytes
    pub compressed_size_bytes: u64,

    /// SHA-256 checksum of original data
    pub checksum_sha256: String,

    /// User-provided metadata
    pub metadata: HashMap<String, String>,

    /// Relative path to encrypted file
    pub file_path: String,
}

/// Version control system for model checkpoints
///
/// Features:
/// - Complete version history
/// - Parent-child relationships (branching)
/// - Metadata tracking
/// - Checksum verification
/// - Generation/lineage tracking
///
/// Compliance:
/// - CMMC AU.3.046: Alert in the event of an audit logging process failure
/// - CMMC AU.3.049: Protect audit information from unauthorized access
pub struct VersionControl {
    vault_path: PathBuf,
    version_file: PathBuf,
    pub(crate) versions: HashMap<String, Vec<ModelVersion>>,
}

impl VersionControl {
    const VERSION_FILE: &'static str = "versions.json";

    /// Create new version control instance
    pub fn new(vault_path: &Path) -> Result<Self> {
        let version_file = vault_path.join(Self::VERSION_FILE);
        let mut vc = Self {
            vault_path: vault_path.to_path_buf(),
            version_file,
            versions: HashMap::new(),
        };
        vc.load_versions()?;
        Ok(vc)
    }

    /// Return the vault directory path
    pub fn vault_path(&self) -> &std::path::Path {
        &self.vault_path
    }

    /// Load version history from file
    fn load_versions(&mut self) -> Result<()> {
        if self.version_file.exists() {
            let contents = fs::read_to_string(&self.version_file)?;
            self.versions = serde_json::from_str(&contents)?;
        }
        Ok(())
    }

    /// Save version history to file
    fn save_versions(&self) -> Result<()> {
        let contents = serde_json::to_string_pretty(&self.versions)?;
        fs::write(&self.version_file, contents)?;
        crate::permissions::restrict_file(&self.version_file)?;

        Ok(())
    }

    /// Add new model version
    #[allow(clippy::too_many_arguments)]
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
    ) -> Result<ModelVersion> {
        let versions = self.versions.entry(model_name.to_string()).or_default();

        // Determine next version number
        let version = if versions.is_empty() {
            1
        } else {
            versions.iter().map(|v| v.version).max().unwrap_or(0) + 1
        };

        let timestamp = Utc::now();
        let checkpoint_id = Self::generate_checkpoint_id(model_name, version, &timestamp);

        let model_version = ModelVersion {
            version,
            checkpoint_id,
            timestamp,
            parent_version,
            format: format.to_string(),
            size_bytes,
            compressed_size_bytes,
            checksum_sha256: checksum.to_string(),
            metadata: metadata.unwrap_or_default(),
            file_path: file_path.to_string(),
        };

        versions.push(model_version.clone());
        self.save_versions()?;

        Ok(model_version)
    }

    /// Get specific model version
    pub fn get_version(&self, model_name: &str, version: Option<u32>) -> Option<&ModelVersion> {
        let versions = self.versions.get(model_name)?;

        if versions.is_empty() {
            return None;
        }

        if let Some(v) = version {
            versions.iter().find(|mv| mv.version == v)
        } else {
            // Return latest version
            versions.iter().max_by_key(|mv| mv.version)
        }
    }

    /// List all versions of a model
    pub fn list_versions(&self, model_name: &str) -> Vec<&ModelVersion> {
        self.versions
            .get(model_name)
            .map(|v| {
                let mut sorted: Vec<&ModelVersion> = v.iter().collect();
                sorted.sort_by_key(|mv| mv.version);
                sorted
            })
            .unwrap_or_default()
    }

    /// Get complete lineage/generation history for a version
    pub fn get_lineage(&self, model_name: &str, version: u32) -> Vec<&ModelVersion> {
        let mut lineage = Vec::new();

        if let Some(mut current) = self.get_version(model_name, Some(version)) {
            lineage.push(current);

            while let Some(parent_ver) = current.parent_version {
                if let Some(parent) = self.get_version(model_name, Some(parent_ver)) {
                    lineage.insert(0, parent);
                    current = parent;
                } else {
                    break;
                }
            }
        }

        lineage
    }

    /// Delete a specific version.
    ///
    /// Deleting a model's last version removes the model, rather than leaving
    /// the name behind with an empty version list. There is no "delete a model"
    /// operation — deleting every version *is* how a caller deletes one — so a
    /// name that outlived its versions showed up in `list_models` and counted
    /// toward `model_count` while having nothing in it, and a CLI built on this
    /// had to explain a model that was not there.
    pub fn delete_version(&mut self, model_name: &str, version: u32) -> Result<bool> {
        if let Some(versions) = self.versions.get_mut(model_name) {
            let original_len = versions.len();
            versions.retain(|v| v.version != version);

            if versions.len() < original_len {
                if versions.is_empty() {
                    self.versions.remove(model_name);
                }
                self.save_versions()?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Clean up old versions, keeping only the most recent
    pub fn cleanup_old_versions(
        &mut self,
        model_name: &str,
        keep_count: usize,
    ) -> Result<Vec<u32>> {
        let Some(versions) = self.versions.get_mut(model_name) else {
            return Ok(Vec::new());
        };

        if versions.len() <= keep_count {
            return Ok(Vec::new());
        }

        // Sort by version number descending
        versions.sort_by_key(|v| std::cmp::Reverse(v.version));

        // Keep the most recent
        let to_delete: Vec<u32> = versions
            .iter()
            .skip(keep_count)
            .map(|v| v.version)
            .collect();

        versions.truncate(keep_count);
        // `keep_count == 0` empties the model, and a name with no versions is
        // not a model — the same rule `delete_version` follows. The SQLite
        // backend gets this for free by delegating to its own `delete_version`;
        // this branch is what keeps the two agreeing.
        if versions.is_empty() {
            self.versions.remove(model_name);
        }
        self.save_versions()?;

        Ok(to_delete)
    }

    /// Verify data integrity using stored checksum
    pub fn verify_checksum(&self, model_name: &str, version: u32, data: &[u8]) -> bool {
        if let Some(model_version) = self.get_version(model_name, Some(version)) {
            let checksum = hex::encode(FipsCrypto::hash_sha256(data));
            return checksum == model_version.checksum_sha256;
        }
        false
    }

    /// Update metadata for a specific model version
    pub fn update_metadata(
        &mut self,
        model_name: &str,
        version: u32,
        key: &str,
        value: String,
    ) -> Result<()> {
        if let Some(versions) = self.versions.get_mut(model_name) {
            if let Some(model_version) = versions.iter_mut().find(|v| v.version == version) {
                model_version.metadata.insert(key.to_string(), value);
                self.save_versions()?;
                return Ok(());
            }
        }
        Err(crate::error::VaultError::VersionNotFound(
            version,
            model_name.to_string(),
        ))
    }

    /// Get metadata for a specific model version
    pub fn get_metadata(&self, model_name: &str, version: u32, key: &str) -> Option<String> {
        self.get_version(model_name, Some(version))
            .and_then(|v| v.metadata.get(key).cloned())
    }

    /// Return an owned list of model names (useful when the caller needs
    /// ownership, e.g. for export/import operations).
    pub fn list_models_owned(&self) -> Vec<String> {
        self.versions.keys().cloned().collect()
    }

    /// Import a pre-built `ModelVersion` for a model — used during vault
    /// bundle import to transplant versions from another vault.
    pub fn import_version(&mut self, model_name: &str, version: ModelVersion) -> Result<()> {
        self.versions
            .entry(model_name.to_string())
            .or_default()
            .push(version);
        self.save_versions()
    }

    /// Generate unique checkpoint identifier
    fn generate_checkpoint_id(
        model_name: &str,
        version: u32,
        _timestamp: &DateTime<Utc>,
    ) -> String {
        let uuid = Uuid::new_v4();
        format!("{}-v{}-{}", model_name, version, uuid)
    }
}

// ── Trait implementation ─────────────────────────────────────

impl crate::traits::VersionRepo for VersionControl {
    fn add_version(
        &mut self,
        model: &str,
        file_path: &str,
        format: &str,
        size_bytes: u64,
        compressed_size_bytes: u64,
        checksum: &str,
        metadata: Option<HashMap<String, String>>,
        parent_version: Option<u32>,
    ) -> Result<ModelVersion> {
        VersionControl::add_version(
            self,
            model,
            file_path,
            format,
            size_bytes,
            compressed_size_bytes,
            checksum,
            metadata,
            parent_version,
        )
    }

    fn get_version(&self, model: &str, version: Option<u32>) -> Option<&ModelVersion> {
        VersionControl::get_version(self, model, version)
    }

    fn list_versions(&self, model: &str) -> Vec<&ModelVersion> {
        VersionControl::list_versions(self, model)
    }

    fn get_lineage(&self, model: &str, version: u32) -> Vec<&ModelVersion> {
        VersionControl::get_lineage(self, model, version)
    }

    fn delete_version(&mut self, model: &str, version: u32) -> Result<bool> {
        VersionControl::delete_version(self, model, version)
    }

    fn cleanup_old_versions(&mut self, model: &str, keep_count: usize) -> Result<Vec<u32>> {
        VersionControl::cleanup_old_versions(self, model, keep_count)
    }

    fn verify_checksum(&self, model: &str, version: u32, data: &[u8]) -> bool {
        VersionControl::verify_checksum(self, model, version, data)
    }

    fn update_metadata(
        &mut self,
        model: &str,
        version: u32,
        key: &str,
        value: String,
    ) -> Result<()> {
        VersionControl::update_metadata(self, model, version, key, value)
    }

    fn get_metadata(&self, model: &str, version: u32, key: &str) -> Option<String> {
        VersionControl::get_metadata(self, model, version, key)
    }

    fn list_models(&self) -> Vec<String> {
        self.versions.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_version_control() {
        let temp_dir = tempdir().unwrap();
        let mut vc = VersionControl::new(temp_dir.path()).unwrap();

        let v1 = vc
            .add_version(
                "test_model",
                "test_file.enc",
                "pytorch",
                1000,
                500,
                "abc123",
                None,
                None,
            )
            .unwrap();

        assert_eq!(v1.version, 1);
        assert_eq!(v1.format, "pytorch");

        let v2 = vc
            .add_version(
                "test_model",
                "test_file2.enc",
                "pytorch",
                2000,
                1000,
                "def456",
                None,
                Some(1),
            )
            .unwrap();

        assert_eq!(v2.version, 2);
        assert_eq!(v2.parent_version, Some(1));

        let lineage = vc.get_lineage("test_model", 2);
        assert_eq!(lineage.len(), 2);
    }

    #[test]
    fn test_version_control_list_versions() {
        // Covers line 175 — list_versions sorted path
        let temp_dir = tempdir().unwrap();
        let mut vc = VersionControl::new(temp_dir.path()).unwrap();

        vc.add_version("m", "f1.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        vc.add_version("m", "f2.enc", "pt", 200, 100, "c2", None, Some(1))
            .unwrap();
        vc.add_version("m", "f3.enc", "pt", 300, 150, "c3", None, Some(2))
            .unwrap();

        let versions = vc.list_versions("m");
        assert_eq!(versions.len(), 3);
        assert_eq!(versions[0].version, 1);
        assert_eq!(versions[2].version, 3);

        // Nonexistent model
        let empty = vc.list_versions("nonexistent");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_version_control_reopen() {
        // Covers line 72 — re-load existing versions.json
        let temp_dir = tempdir().unwrap();
        {
            let mut vc = VersionControl::new(temp_dir.path()).unwrap();
            vc.add_version("m", "f1.enc", "pt", 100, 50, "c1", None, None)
                .unwrap();
        }
        let vc2 = VersionControl::new(temp_dir.path()).unwrap();
        assert_eq!(vc2.list_versions("m").len(), 1);
    }

    #[test]
    fn test_delete_version() {
        let temp_dir = tempdir().unwrap();
        let mut vc = VersionControl::new(temp_dir.path()).unwrap();
        vc.add_version("m", "f1.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        vc.add_version("m", "f2.enc", "pt", 200, 100, "c2", None, Some(1))
            .unwrap();

        let deleted = vc.delete_version("m", 1).unwrap();
        assert!(deleted);
        assert_eq!(vc.list_versions("m").len(), 1);

        // Delete nonexistent version
        let not_deleted = vc.delete_version("m", 99).unwrap();
        assert!(!not_deleted);

        // Delete from nonexistent model
        let not_deleted2 = vc.delete_version("nonexistent", 1).unwrap();
        assert!(!not_deleted2);
    }

    /// Deleting every version of a model deletes the model. There is no other
    /// way to delete one, so a name left behind with an empty version list is a
    /// model that `list_models` reports and `get_model` cannot serve.
    #[test]
    fn deleting_the_last_version_removes_the_model() {
        let temp_dir = tempdir().unwrap();
        let mut vc = VersionControl::new(temp_dir.path()).unwrap();
        vc.add_version("m", "f1.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        vc.add_version("keep", "k1.enc", "pt", 10, 5, "k", None, None)
            .unwrap();

        assert!(vc.delete_version("m", 1).unwrap());

        assert!(
            !vc.list_models_owned().contains(&"m".to_string()),
            "the model outlived its last version: {:?}",
            vc.list_models_owned()
        );
        assert!(vc.list_versions("m").is_empty());
        // Its neighbour is untouched.
        assert_eq!(vc.list_versions("keep").len(), 1);

        // And the removal is persisted, not only in memory.
        let reloaded = VersionControl::new(temp_dir.path()).unwrap();
        assert_eq!(reloaded.list_models_owned(), vec!["keep".to_string()]);

        // Storing under the name again starts a fresh history.
        assert_eq!(
            vc.add_version("m", "f2.enc", "pt", 100, 50, "c2", None, None)
                .unwrap()
                .version,
            1
        );
    }

    #[test]
    fn test_cleanup_old_versions() {
        let temp_dir = tempdir().unwrap();
        let mut vc = VersionControl::new(temp_dir.path()).unwrap();
        for i in 0..5 {
            vc.add_version(
                "m",
                &format!("f{}.enc", i),
                "pt",
                100,
                50,
                &format!("c{}", i),
                None,
                if i == 0 { None } else { Some(i as u32) },
            )
            .unwrap();
        }
        assert_eq!(vc.list_versions("m").len(), 5);

        // Keep only 2
        let removed = vc.cleanup_old_versions("m", 2).unwrap();
        assert_eq!(removed.len(), 3);
        assert_eq!(vc.list_versions("m").len(), 2);

        // Cleanup with keep_count >= current count
        let removed2 = vc.cleanup_old_versions("m", 10).unwrap();
        assert!(removed2.is_empty());

        // Cleanup nonexistent model
        let removed3 = vc.cleanup_old_versions("nonexistent", 1).unwrap();
        assert!(removed3.is_empty());
    }

    #[test]
    fn test_verify_checksum() {
        let temp_dir = tempdir().unwrap();
        let mut vc = VersionControl::new(temp_dir.path()).unwrap();
        let data = b"hello world";
        let checksum = hex::encode(crate::crypto::FipsCrypto::hash_sha256(data));
        vc.add_version(
            "m",
            "f.enc",
            "pt",
            data.len() as u64,
            data.len() as u64,
            &checksum,
            None,
            None,
        )
        .unwrap();

        assert!(vc.verify_checksum("m", 1, data));
        assert!(!vc.verify_checksum("m", 1, b"wrong data"));
        assert!(!vc.verify_checksum("m", 99, data));
        assert!(!vc.verify_checksum("nonexistent", 1, data));
    }

    #[test]
    fn test_update_and_get_metadata() {
        let temp_dir = tempdir().unwrap();
        let mut vc = VersionControl::new(temp_dir.path()).unwrap();
        vc.add_version("m", "f.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();

        vc.update_metadata("m", 1, "author", "Alice".to_string())
            .unwrap();
        assert_eq!(vc.get_metadata("m", 1, "author"), Some("Alice".to_string()));

        // Nonexistent key
        assert_eq!(vc.get_metadata("m", 1, "missing_key"), None);

        // Nonexistent version
        let err = vc.update_metadata("m", 99, "key", "val".to_string());
        assert!(err.is_err());

        // Nonexistent model
        let err2 = vc.update_metadata("nonexistent", 1, "key", "val".to_string());
        assert!(err2.is_err());
    }

    #[test]
    fn test_get_lineage_chain() {
        let temp_dir = tempdir().unwrap();
        let mut vc = VersionControl::new(temp_dir.path()).unwrap();
        vc.add_version("m", "f1.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        vc.add_version("m", "f2.enc", "pt", 200, 100, "c2", None, Some(1))
            .unwrap();
        vc.add_version("m", "f3.enc", "pt", 300, 150, "c3", None, Some(2))
            .unwrap();

        let lineage = vc.get_lineage("m", 3);
        assert_eq!(lineage.len(), 3);
        assert_eq!(lineage[0].version, 1); // oldest first
        assert_eq!(lineage[2].version, 3); // newest last

        // Lineage with nonexistent version
        let empty = vc.get_lineage("m", 99);
        assert!(empty.is_empty());

        // Lineage for nonexistent model
        let empty2 = vc.get_lineage("nonexistent", 1);
        assert!(empty2.is_empty());
    }

    #[test]
    fn test_add_version_with_metadata() {
        let temp_dir = tempdir().unwrap();
        let mut vc = VersionControl::new(temp_dir.path()).unwrap();
        let mut meta = std::collections::HashMap::new();
        meta.insert("framework".to_string(), "pytorch".to_string());
        let v = vc
            .add_version("m", "f.enc", "pt", 100, 50, "c1", Some(meta), None)
            .unwrap();
        assert_eq!(v.metadata.get("framework"), Some(&"pytorch".to_string()));
    }

    #[test]
    fn test_version_repo_trait_forwarding() {
        // Covers L297, L321-342, L345, L352, L355-356 — VersionRepo trait impl
        use crate::traits::VersionRepo;
        let temp_dir = tempdir().unwrap();
        let mut vc = VersionControl::new(temp_dir.path()).unwrap();

        // add_version via trait
        let v = VersionRepo::add_version(
            &mut vc,
            "m",
            "f.enc",
            "pt",
            100,
            50,
            "checksum1",
            None,
            None,
        )
        .unwrap();
        assert_eq!(v.version, 1);

        // get_version via trait
        let got = VersionRepo::get_version(&vc, "m", Some(1));
        assert!(got.is_some());
        assert_eq!(got.unwrap().version, 1);

        // list_versions via trait
        let list = VersionRepo::list_versions(&vc, "m");
        assert_eq!(list.len(), 1);

        // get_lineage via trait
        let lineage = VersionRepo::get_lineage(&vc, "m", 1);
        assert_eq!(lineage.len(), 1);

        // verify_checksum via trait
        let valid = VersionRepo::verify_checksum(&vc, "m", 1, b"dummy");
        assert!(!valid); // sha256 of "dummy" != "checksum1"

        // update_metadata via trait
        VersionRepo::update_metadata(&mut vc, "m", 1, "key1", "val1".into()).unwrap();

        // get_metadata via trait
        let val = VersionRepo::get_metadata(&vc, "m", 1, "key1");
        assert_eq!(val, Some("val1".into()));

        // delete_version via trait
        let deleted = VersionRepo::delete_version(&mut vc, "m", 1).unwrap();
        assert!(deleted);

        // cleanup_old_versions via trait — add several versions first
        for i in 0..5 {
            VersionRepo::add_version(
                &mut vc,
                "m2",
                &format!("f{}.enc", i),
                "pt",
                100,
                50,
                &format!("c{}", i),
                None,
                None,
            )
            .unwrap();
        }
        let cleaned = VersionRepo::cleanup_old_versions(&mut vc, "m2", 2).unwrap();
        assert_eq!(cleaned.len(), 3);

        // list_models via trait
        let models = VersionRepo::list_models(&vc);
        assert!(models.contains(&"m2".to_string()));
    }

    #[test]
    fn test_vault_path_method() {
        // Covers L83-84 — vault_path() getter
        let temp_dir = tempdir().unwrap();
        let vc = VersionControl::new(temp_dir.path()).unwrap();
        assert_eq!(vc.vault_path(), temp_dir.path());
    }

    #[test]
    fn test_load_existing_versions() {
        // Covers L67-68 — loading versions from existing file
        let temp_dir = tempdir().unwrap();
        {
            let mut vc = VersionControl::new(temp_dir.path()).unwrap();
            vc.add_version("m", "f.enc", "pt", 100, 50, "c1", None, None)
                .unwrap();
        }
        // Re-open: should load the saved versions
        let vc2 = VersionControl::new(temp_dir.path()).unwrap();
        let versions = vc2.list_versions("m");
        assert_eq!(versions.len(), 1);
    }
}
