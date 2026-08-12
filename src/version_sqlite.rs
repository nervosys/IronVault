//! SQLite-backed version repository.
//!
//! Provides ACID, indexed, concurrent-safe version storage via SQLite.
//! Auto-migrates from the legacy `versions.json` format on first access.
//!
//! Feature-gated behind `sqlite`.

#[cfg(feature = "sqlite")]
use std::collections::HashMap;
#[cfg(feature = "sqlite")]
use std::path::{Path, PathBuf};

#[cfg(feature = "sqlite")]
use chrono::{DateTime, Utc};
#[cfg(feature = "sqlite")]
use uuid::Uuid;

#[cfg(feature = "sqlite")]
use crate::crypto::FipsCrypto;
#[cfg(feature = "sqlite")]
use crate::error::{Result, VaultError};
#[cfg(feature = "sqlite")]
use crate::version::ModelVersion;

/// SQLite-backed version repository with ACID guarantees.
///
/// Maintains an in-memory cache synchronized with the database
/// so that the `VersionRepo` trait (which returns references) works.
#[cfg(feature = "sqlite")]
pub struct SqliteVersionRepo {
    conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    vault_path: PathBuf,
    /// In-memory cache for reference-returning trait methods.
    cache: HashMap<String, Vec<ModelVersion>>,
}

#[cfg(feature = "sqlite")]
impl SqliteVersionRepo {
    /// Open (or create) a SQLite version database in the vault directory.
    ///
    /// If a legacy `versions.json` file exists, it is auto-migrated
    /// into the database, then renamed to `versions.json.migrated`.
    pub fn new(vault_path: &Path) -> Result<Self> {
        let db_path = vault_path.join("versions.db");
        let conn = rusqlite::Connection::open(&db_path).map_err(|e| {
            VaultError::StorageError(format!("Failed to open versions database: {}", e))
        })?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| VaultError::StorageError(format!("Failed to set PRAGMA: {}", e)))?;

        // Create tables
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                model_name TEXT NOT NULL,
                version INTEGER NOT NULL,
                checkpoint_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                parent_version INTEGER,
                format TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                compressed_size_bytes INTEGER NOT NULL,
                checksum_sha256 TEXT NOT NULL,
                file_path TEXT NOT NULL,
                UNIQUE(model_name, version)
            );
            CREATE TABLE IF NOT EXISTS version_metadata (
                model_name TEXT NOT NULL,
                version INTEGER NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                PRIMARY KEY(model_name, version, key)
            );
            CREATE INDEX IF NOT EXISTS idx_versions_model ON versions(model_name);
            CREATE INDEX IF NOT EXISTS idx_versions_model_ver ON versions(model_name, version);",
        )
        .map_err(|e| VaultError::StorageError(format!("Failed to create version tables: {}", e)))?;

        let mut repo = Self {
            conn: std::sync::Arc::new(std::sync::Mutex::new(conn)),
            vault_path: vault_path.to_path_buf(),
            cache: HashMap::new(),
        };

        // Auto-migrate from versions.json if it exists
        repo.migrate_from_json()?;

        // Load cache from database
        repo.reload_cache()?;

        Ok(repo)
    }

    /// Create an in-memory SQLite version repository (for testing).
    pub fn in_memory() -> Result<Self> {
        let conn = rusqlite::Connection::open_in_memory().map_err(|e| {
            VaultError::StorageError(format!("Failed to create in-memory DB: {}", e))
        })?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                model_name TEXT NOT NULL,
                version INTEGER NOT NULL,
                checkpoint_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                parent_version INTEGER,
                format TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                compressed_size_bytes INTEGER NOT NULL,
                checksum_sha256 TEXT NOT NULL,
                file_path TEXT NOT NULL,
                UNIQUE(model_name, version)
            );
            CREATE TABLE IF NOT EXISTS version_metadata (
                model_name TEXT NOT NULL,
                version INTEGER NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                PRIMARY KEY(model_name, version, key)
            );",
        )
        .map_err(|e| VaultError::StorageError(format!("Failed to create tables: {}", e)))?;

        Ok(Self {
            conn: std::sync::Arc::new(std::sync::Mutex::new(conn)),
            vault_path: PathBuf::new(),
            cache: HashMap::new(),
        })
    }

    /// Migrate legacy `versions.json` into the database.
    fn migrate_from_json(&self) -> Result<()> {
        let json_path = self.vault_path.join("versions.json");
        if !json_path.exists() {
            return Ok(());
        }

        let contents = std::fs::read_to_string(&json_path)?;
        let versions: HashMap<String, Vec<ModelVersion>> = serde_json::from_str(&contents)?;

        let conn = self
            .conn
            .lock()
            .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| VaultError::StorageError(format!("Transaction failed: {}", e)))?;

        for (model_name, model_versions) in &versions {
            for mv in model_versions {
                tx.execute(
                    "INSERT OR IGNORE INTO versions
                     (model_name, version, checkpoint_id, timestamp, parent_version,
                      format, size_bytes, compressed_size_bytes, checksum_sha256, file_path)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        model_name,
                        mv.version,
                        mv.checkpoint_id,
                        mv.timestamp.to_rfc3339(),
                        mv.parent_version,
                        mv.format,
                        mv.size_bytes,
                        mv.compressed_size_bytes,
                        mv.checksum_sha256,
                        mv.file_path,
                    ],
                )
                .map_err(|e| {
                    VaultError::StorageError(format!(
                        "Failed to migrate version {}/{}: {}",
                        model_name, mv.version, e
                    ))
                })?;

                // Migrate metadata
                for (key, value) in &mv.metadata {
                    tx.execute(
                        "INSERT OR IGNORE INTO version_metadata
                         (model_name, version, key, value)
                         VALUES (?1, ?2, ?3, ?4)",
                        rusqlite::params![model_name, mv.version, key, value],
                    )
                    .map_err(|e| {
                        VaultError::StorageError(format!("Failed to migrate metadata: {}", e))
                    })?;
                }
            }
        }

        tx.commit()
            .map_err(|e| VaultError::StorageError(format!("Migration commit failed: {}", e)))?;

        // Rename the old file to mark migration as complete
        let migrated_path = self.vault_path.join("versions.json.migrated");
        std::fs::rename(&json_path, &migrated_path)?;

        Ok(())
    }

    /// Reload the in-memory cache from the database.
    fn reload_cache(&mut self) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;

        let mut cache: HashMap<String, Vec<ModelVersion>> = HashMap::new();

        // Load all versions
        let mut stmt = conn
            .prepare(
                "SELECT model_name, version, checkpoint_id, timestamp, parent_version,
                        format, size_bytes, compressed_size_bytes, checksum_sha256, file_path
                 FROM versions ORDER BY model_name, version",
            )
            .map_err(|e| VaultError::StorageError(format!("Prepare failed: {}", e)))?;

        let rows = stmt
            .query_map([], |row| {
                let model_name: String = row.get(0)?;
                let timestamp_str: String = row.get(3)?;
                let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok((
                    model_name,
                    ModelVersion {
                        version: row.get(1)?,
                        checkpoint_id: row.get(2)?,
                        timestamp,
                        parent_version: row.get(4)?,
                        format: row.get(5)?,
                        size_bytes: row.get(6)?,
                        compressed_size_bytes: row.get(7)?,
                        checksum_sha256: row.get(8)?,
                        file_path: row.get(9)?,
                        metadata: HashMap::new(), // loaded below
                    },
                ))
            })
            .map_err(|e| VaultError::StorageError(format!("Query failed: {}", e)))?;

        for row in rows {
            let (model_name, mv) =
                row.map_err(|e| VaultError::StorageError(format!("Row read failed: {}", e)))?;
            cache.entry(model_name).or_default().push(mv);
        }

        // Load all metadata
        let mut meta_stmt = conn
            .prepare("SELECT model_name, version, key, value FROM version_metadata")
            .map_err(|e| VaultError::StorageError(format!("Prepare failed: {}", e)))?;

        let meta_rows = meta_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| VaultError::StorageError(format!("Meta query failed: {}", e)))?;

        for row in meta_rows {
            let (model_name, version, key, value) =
                row.map_err(|e| VaultError::StorageError(format!("Meta row read failed: {}", e)))?;

            if let Some(versions) = cache.get_mut(&model_name) {
                if let Some(mv) = versions.iter_mut().find(|v| v.version == version) {
                    mv.metadata.insert(key, value);
                }
            }
        }

        drop(stmt);
        drop(meta_stmt);
        drop(conn);

        self.cache = cache;
        Ok(())
    }

    /// Generate unique checkpoint identifier.
    fn generate_checkpoint_id(model_name: &str, version: u32) -> String {
        let uuid = Uuid::new_v4();
        format!("{}-v{}-{}", model_name, version, uuid)
    }

    /// Return the vault directory path.
    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }
}

// ── VersionRepo trait implementation ─────────────────────────

#[cfg(feature = "sqlite")]
impl crate::traits::VersionRepo for SqliteVersionRepo {
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
        // Determine next version number
        let version = {
            let conn = self
                .conn
                .lock()
                .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;
            let max_ver: Option<u32> = conn
                .query_row(
                    "SELECT MAX(version) FROM versions WHERE model_name = ?1",
                    rusqlite::params![model],
                    |row| row.get(0),
                )
                .unwrap_or(None);
            max_ver.unwrap_or(0) + 1
        };

        let timestamp = Utc::now();
        let checkpoint_id = Self::generate_checkpoint_id(model, version);
        let meta = metadata.unwrap_or_default();

        {
            let conn = self
                .conn
                .lock()
                .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;

            let tx = conn
                .unchecked_transaction()
                .map_err(|e| VaultError::StorageError(format!("Transaction failed: {}", e)))?;

            tx.execute(
                "INSERT INTO versions
                 (model_name, version, checkpoint_id, timestamp, parent_version,
                  format, size_bytes, compressed_size_bytes, checksum_sha256, file_path)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    model,
                    version,
                    checkpoint_id,
                    timestamp.to_rfc3339(),
                    parent_version,
                    format,
                    size_bytes,
                    compressed_size_bytes,
                    checksum,
                    file_path,
                ],
            )
            .map_err(|e| VaultError::StorageError(format!("Insert version failed: {}", e)))?;

            for (key, value) in &meta {
                tx.execute(
                    "INSERT INTO version_metadata (model_name, version, key, value)
                     VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![model, version, key, value],
                )
                .map_err(|e| VaultError::StorageError(format!("Insert metadata failed: {}", e)))?;
            }

            tx.commit()
                .map_err(|e| VaultError::StorageError(format!("Commit failed: {}", e)))?;
        }

        let mv = ModelVersion {
            version,
            checkpoint_id,
            timestamp,
            parent_version,
            format: format.to_string(),
            size_bytes,
            compressed_size_bytes,
            checksum_sha256: checksum.to_string(),
            metadata: meta,
            file_path: file_path.to_string(),
        };

        // Update cache
        self.cache
            .entry(model.to_string())
            .or_default()
            .push(mv.clone());

        Ok(mv)
    }

    fn get_version(&self, model: &str, version: Option<u32>) -> Option<&ModelVersion> {
        let versions = self.cache.get(model)?;
        if versions.is_empty() {
            return None;
        }
        if let Some(v) = version {
            versions.iter().find(|mv| mv.version == v)
        } else {
            versions.iter().max_by_key(|mv| mv.version)
        }
    }

    fn list_versions(&self, model: &str) -> Vec<&ModelVersion> {
        self.cache
            .get(model)
            .map(|v| {
                let mut sorted: Vec<&ModelVersion> = v.iter().collect();
                sorted.sort_by_key(|mv| mv.version);
                sorted
            })
            .unwrap_or_default()
    }

    fn get_lineage(&self, model: &str, version: u32) -> Vec<&ModelVersion> {
        let mut lineage = Vec::new();
        if let Some(mut current) = self.get_version(model, Some(version)) {
            lineage.push(current);
            while let Some(parent_ver) = current.parent_version {
                if let Some(parent) = self.get_version(model, Some(parent_ver)) {
                    lineage.insert(0, parent);
                    current = parent;
                } else {
                    break;
                }
            }
        }
        lineage
    }

    fn delete_version(&mut self, model: &str, version: u32) -> Result<bool> {
        let deleted = {
            let conn = self
                .conn
                .lock()
                .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;

            let count = conn
                .execute(
                    "DELETE FROM versions WHERE model_name = ?1 AND version = ?2",
                    rusqlite::params![model, version],
                )
                .map_err(|e| VaultError::StorageError(format!("Delete failed: {}", e)))?;

            conn.execute(
                "DELETE FROM version_metadata WHERE model_name = ?1 AND version = ?2",
                rusqlite::params![model, version],
            )
            .map_err(|e| VaultError::StorageError(format!("Delete metadata failed: {}", e)))?;

            count > 0
        };

        // Update the cache, dropping the model when its last version goes.
        //
        // Not cosmetic: `list_models` answers from the cache, and the rows are
        // already gone from the database. A retained empty entry meant this repo
        // listed a model that the same repo would not list after a restart, when
        // the cache is rebuilt from those rows. It also matches the JSON
        // backend, which the same trait is meant to make interchangeable.
        if let Some(versions) = self.cache.get_mut(model) {
            versions.retain(|v| v.version != version);
            if versions.is_empty() {
                self.cache.remove(model);
            }
        }

        Ok(deleted)
    }

    fn cleanup_old_versions(&mut self, model: &str, keep_count: usize) -> Result<Vec<u32>> {
        let to_delete: Vec<u32> = {
            let conn = self
                .conn
                .lock()
                .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;

            // Get versions to delete (all except the N most recent)
            let mut stmt = conn
                .prepare(
                    "SELECT version FROM versions WHERE model_name = ?1
                     ORDER BY version DESC",
                )
                .map_err(|e| VaultError::StorageError(format!("Prepare failed: {}", e)))?;

            let all_versions: Vec<u32> = stmt
                .query_map(rusqlite::params![model], |row| row.get(0))
                .map_err(|e| VaultError::StorageError(format!("Query failed: {}", e)))?
                .filter_map(|r| r.ok())
                .collect();

            if all_versions.len() <= keep_count {
                return Ok(Vec::new());
            }

            all_versions[keep_count..].to_vec()
        };

        for version in &to_delete {
            self.delete_version(model, *version)?;
        }

        Ok(to_delete)
    }

    fn verify_checksum(&self, model: &str, version: u32, data: &[u8]) -> bool {
        if let Some(mv) = self.get_version(model, Some(version)) {
            let checksum = hex::encode(FipsCrypto::hash_sha256(data));
            return checksum == mv.checksum_sha256;
        }
        false
    }

    fn update_metadata(
        &mut self,
        model: &str,
        version: u32,
        key: &str,
        value: String,
    ) -> Result<()> {
        {
            let conn = self
                .conn
                .lock()
                .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;

            conn.execute(
                "INSERT OR REPLACE INTO version_metadata (model_name, version, key, value)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![model, version, key, value],
            )
            .map_err(|e| VaultError::StorageError(format!("Upsert metadata failed: {}", e)))?;
        }

        // Update cache
        if let Some(versions) = self.cache.get_mut(model) {
            if let Some(mv) = versions.iter_mut().find(|v| v.version == version) {
                mv.metadata.insert(key.to_string(), value);
                return Ok(());
            }
        }

        Err(VaultError::VersionNotFound(version, model.to_string()))
    }

    fn get_metadata(&self, model: &str, version: u32, key: &str) -> Option<String> {
        self.get_version(model, Some(version))
            .and_then(|v| v.metadata.get(key).cloned())
    }

    fn list_models(&self) -> Vec<String> {
        self.cache.keys().cloned().collect()
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::traits::VersionRepo;

    #[test]
    fn test_sqlite_version_repo_basic() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();

        let v1 = repo
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

        let v2 = repo
            .add_version(
                "test_model",
                "test_file2.enc",
                "safetensors",
                2000,
                1000,
                "def456",
                None,
                Some(1),
            )
            .unwrap();

        assert_eq!(v2.version, 2);
        assert_eq!(v2.parent_version, Some(1));

        // Test get_version
        let latest = repo.get_version("test_model", None).unwrap();
        assert_eq!(latest.version, 2);

        let v1_ref = repo.get_version("test_model", Some(1)).unwrap();
        assert_eq!(v1_ref.format, "pytorch");

        // Test list_versions
        let all = repo.list_versions("test_model");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].version, 1);
        assert_eq!(all[1].version, 2);

        // Test lineage
        let lineage = repo.get_lineage("test_model", 2);
        assert_eq!(lineage.len(), 2);
    }

    #[test]
    fn test_sqlite_version_repo_delete() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();

        repo.add_version("m", "f.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        repo.add_version("m", "f2.enc", "pt", 200, 100, "c2", None, Some(1))
            .unwrap();
        repo.add_version("m", "f3.enc", "pt", 300, 150, "c3", None, Some(2))
            .unwrap();

        assert!(repo.delete_version("m", 2).unwrap());
        assert_eq!(repo.list_versions("m").len(), 2);
        assert!(!repo.delete_version("m", 99).unwrap());
    }

    /// Deleting every version deletes the model here too, and for a reason
    /// specific to this backend: `list_models` answers from the cache while the
    /// rows are already gone, so a retained empty entry made this repo list a
    /// model that the same repo would stop listing after a reopen.
    #[test]
    fn deleting_the_last_version_removes_the_model_from_the_cache_and_the_database() {
        let temp = tempfile::tempdir().unwrap();
        let vault_path = temp.path();

        {
            let mut repo = SqliteVersionRepo::new(vault_path).unwrap();
            repo.add_version("gone", "f.enc", "pt", 100, 50, "c1", None, None)
                .unwrap();
            repo.add_version("keep", "k.enc", "pt", 10, 5, "k1", None, None)
                .unwrap();

            assert!(repo.delete_version("gone", 1).unwrap());
            assert!(
                !repo.list_models().contains(&"gone".to_string()),
                "the cache still lists a model with no rows: {:?}",
                repo.list_models()
            );
            assert!(repo.list_versions("gone").is_empty());
            assert!(repo.list_models().contains(&"keep".to_string()));
        }

        // What a reopen rebuilds from the rows must be what the cache said.
        let reopened = SqliteVersionRepo::new(vault_path).unwrap();
        assert_eq!(reopened.list_models(), vec!["keep".to_string()]);
    }

    #[test]
    fn test_sqlite_version_repo_cleanup() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();

        for i in 0..5 {
            repo.add_version(
                "m",
                &format!("f{}.enc", i),
                "pt",
                100,
                50,
                &format!("c{}", i),
                None,
                if i > 0 { Some(i) } else { None },
            )
            .unwrap();
        }

        let deleted = repo.cleanup_old_versions("m", 2).unwrap();
        assert_eq!(deleted.len(), 3);
        assert_eq!(repo.list_versions("m").len(), 2);
    }

    #[test]
    fn test_sqlite_version_repo_metadata() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();

        repo.add_version("m", "f.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();

        repo.update_metadata("m", 1, "author", "test_user".to_string())
            .unwrap();

        let val = repo.get_metadata("m", 1, "author");
        assert_eq!(val, Some("test_user".to_string()));

        // Update existing key
        repo.update_metadata("m", 1, "author", "new_user".to_string())
            .unwrap();
        assert_eq!(
            repo.get_metadata("m", 1, "author"),
            Some("new_user".to_string())
        );
    }

    #[test]
    fn test_sqlite_version_repo_list_models() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();

        repo.add_version("alpha", "f1.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        repo.add_version("beta", "f2.enc", "onnx", 200, 100, "c2", None, None)
            .unwrap();

        let mut models = repo.list_models();
        models.sort();
        assert_eq!(models, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_sqlite_migration_from_json() {
        let temp_dir = tempfile::tempdir().unwrap();
        let vault_path = temp_dir.path();

        // Create a legacy versions.json
        let mut versions: HashMap<String, Vec<ModelVersion>> = HashMap::new();
        versions.insert(
            "legacy_model".to_string(),
            vec![ModelVersion {
                version: 1,
                checkpoint_id: "legacy-v1-test".to_string(),
                timestamp: Utc::now(),
                parent_version: None,
                format: "pytorch".to_string(),
                size_bytes: 999,
                compressed_size_bytes: 500,
                checksum_sha256: "legacy_checksum".to_string(),
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("key1".to_string(), "value1".to_string());
                    m
                },
                file_path: "legacy.enc".to_string(),
            }],
        );

        let json_content = serde_json::to_string_pretty(&versions).unwrap();
        std::fs::write(vault_path.join("versions.json"), json_content).unwrap();

        // Open SQLite repo — should auto-migrate
        let repo = SqliteVersionRepo::new(vault_path).unwrap();

        // Verify migration
        let models = repo.list_models();
        assert_eq!(models, vec!["legacy_model"]);

        let v1 = repo.get_version("legacy_model", Some(1)).unwrap();
        assert_eq!(v1.format, "pytorch");
        assert_eq!(v1.size_bytes, 999);
        assert_eq!(v1.metadata.get("key1"), Some(&"value1".to_string()));

        // Verify the old file was renamed
        assert!(!vault_path.join("versions.json").exists());
        assert!(vault_path.join("versions.json.migrated").exists());
    }

    #[test]
    fn test_sqlite_new_on_disk_and_reopen() {
        // Covers lines 46-103 — new() on disk with WAL, table creation, reload_cache
        let temp_dir = tempfile::tempdir().unwrap();
        let vault_path = temp_dir.path();

        // Create on disk + add a version
        {
            let mut repo = SqliteVersionRepo::new(vault_path).unwrap();
            repo.add_version("disk_model", "f.enc", "onnx", 500, 250, "chk1", None, None)
                .unwrap();
            repo.update_metadata("disk_model", 1, "source", "test".to_string())
                .unwrap();
        }

        // Reopen — exercises reload_cache (lines 129-300+)
        let repo2 = SqliteVersionRepo::new(vault_path).unwrap();
        let v = repo2.get_version("disk_model", Some(1)).unwrap();
        assert_eq!(v.format, "onnx");
        assert_eq!(v.metadata.get("source"), Some(&"test".to_string()));
    }

    #[test]
    fn test_sqlite_verify_checksum() {
        // Covers lines 497, 501, 509
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        let data = b"model data";
        let checksum = hex::encode(FipsCrypto::hash_sha256(data));
        repo.add_version("m", "f.enc", "pt", 100, 50, &checksum, None, None)
            .unwrap();

        assert!(repo.verify_checksum("m", 1, data));
        assert!(!repo.verify_checksum("m", 1, b"tampered"));
        assert!(!repo.verify_checksum("m", 99, data)); // nonexistent version
    }

    #[test]
    fn test_sqlite_get_version_none() {
        let repo = SqliteVersionRepo::in_memory().unwrap();
        assert!(repo.get_version("nonexistent", None).is_none());
        assert!(repo.get_version("nonexistent", Some(1)).is_none());
    }

    #[test]
    fn test_sqlite_add_version_with_metadata() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        let mut meta = HashMap::new();
        meta.insert("author".to_string(), "tester".to_string());
        meta.insert("framework".to_string(), "pytorch".to_string());
        let v = repo
            .add_version("m", "f.enc", "pt", 100, 50, "c1", Some(meta), None)
            .unwrap();
        assert_eq!(v.version, 1);
        assert_eq!(v.metadata.get("author"), Some(&"tester".to_string()));
    }

    #[test]
    fn test_sqlite_update_metadata_nonexistent_version() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        repo.add_version("m", "f.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        let result = repo.update_metadata("m", 99, "key", "val".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_sqlite_get_metadata_none() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        repo.add_version("m", "f.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        assert!(repo.get_metadata("m", 1, "nonexistent").is_none());
        assert!(repo.get_metadata("m", 99, "any").is_none());
    }

    #[test]
    fn test_sqlite_lineage_chain() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        repo.add_version("m", "f1.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        repo.add_version("m", "f2.enc", "pt", 100, 50, "c2", None, Some(1))
            .unwrap();
        repo.add_version("m", "f3.enc", "pt", 100, 50, "c3", None, Some(2))
            .unwrap();

        let lineage = repo.get_lineage("m", 3);
        assert_eq!(lineage.len(), 3);
        assert_eq!(lineage[0].version, 1);
        assert_eq!(lineage[2].version, 3);
    }

    #[test]
    fn test_sqlite_cleanup_noop() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        repo.add_version("m", "f.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        let deleted = repo.cleanup_old_versions("m", 10).unwrap();
        assert!(deleted.is_empty());
    }

    #[test]
    fn test_sqlite_vault_path() {
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteVersionRepo::new(dir.path()).unwrap();
        assert_eq!(repo.vault_path(), dir.path());
    }

    #[test]
    fn test_sqlite_delete_nonexistent_model() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        // Delete from a model that doesn't exist — should return Ok(false)
        let result = repo.delete_version("no_model", 1).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_sqlite_cleanup_empty_model() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        // Cleanup on a model with no versions — should return empty vec
        let deleted = repo.cleanup_old_versions("no_model", 5).unwrap();
        assert!(deleted.is_empty());
    }

    #[test]
    fn test_sqlite_list_versions_empty() {
        let repo = SqliteVersionRepo::in_memory().unwrap();
        let versions = repo.list_versions("nonexistent");
        assert!(versions.is_empty());
    }

    #[test]
    fn test_sqlite_list_models_empty() {
        let repo = SqliteVersionRepo::in_memory().unwrap();
        assert!(repo.list_models().is_empty());
    }

    #[test]
    fn test_sqlite_lineage_nonexistent() {
        let repo = SqliteVersionRepo::in_memory().unwrap();
        let lineage = repo.get_lineage("no_model", 1);
        assert!(lineage.is_empty());
    }

    #[test]
    fn test_sqlite_lineage_no_parent() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        repo.add_version("m", "f.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        let lineage = repo.get_lineage("m", 1);
        assert_eq!(lineage.len(), 1);
        assert_eq!(lineage[0].version, 1);
    }

    #[test]
    fn test_sqlite_lineage_broken_chain() {
        // Lineage with a missing parent version
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        repo.add_version("m", "f1.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        repo.add_version("m", "f2.enc", "pt", 100, 50, "c2", None, Some(1))
            .unwrap();
        repo.add_version("m", "f3.enc", "pt", 100, 50, "c3", None, Some(2))
            .unwrap();
        // Delete version 1, breaking the chain
        repo.delete_version("m", 1).unwrap();
        // Lineage from v3 should stop at v2 (parent v1 missing)
        let lineage = repo.get_lineage("m", 3);
        assert_eq!(lineage.len(), 2);
        assert_eq!(lineage[0].version, 2);
        assert_eq!(lineage[1].version, 3);
    }

    #[test]
    fn test_sqlite_get_version_specific_and_latest() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        repo.add_version("m", "f1.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        repo.add_version("m", "f2.enc", "onnx", 200, 100, "c2", None, Some(1))
            .unwrap();
        repo.add_version("m", "f3.enc", "st", 300, 150, "c3", None, Some(2))
            .unwrap();

        // Get specific versions
        assert_eq!(repo.get_version("m", Some(1)).unwrap().format, "pt");
        assert_eq!(repo.get_version("m", Some(2)).unwrap().format, "onnx");

        // Get latest (None) should return v3
        assert_eq!(repo.get_version("m", None).unwrap().version, 3);

        // Get nonexistent version number
        assert!(repo.get_version("m", Some(99)).is_none());
    }

    #[test]
    fn test_sqlite_verify_checksum_nonexistent_model() {
        let repo = SqliteVersionRepo::in_memory().unwrap();
        assert!(!repo.verify_checksum("no_model", 1, b"data"));
    }

    #[test]
    fn test_sqlite_get_metadata_nonexistent_model() {
        let repo = SqliteVersionRepo::in_memory().unwrap();
        assert!(repo.get_metadata("no_model", 1, "key").is_none());
    }

    #[test]
    fn test_sqlite_multiple_models_independent() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        repo.add_version("alpha", "a.enc", "pt", 100, 50, "ca", None, None)
            .unwrap();
        repo.add_version("beta", "b.enc", "onnx", 200, 100, "cb", None, None)
            .unwrap();
        repo.add_version("alpha", "a2.enc", "st", 300, 150, "ca2", None, Some(1))
            .unwrap();

        // Alpha has 2 versions, beta has 1
        assert_eq!(repo.list_versions("alpha").len(), 2);
        assert_eq!(repo.list_versions("beta").len(), 1);

        // Delete alpha v1 — beta should be unaffected
        repo.delete_version("alpha", 1).unwrap();
        assert_eq!(repo.list_versions("alpha").len(), 1);
        assert_eq!(repo.list_versions("beta").len(), 1);

        let mut models = repo.list_models();
        models.sort();
        assert_eq!(models, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_sqlite_add_version_auto_increment() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        let v1 = repo
            .add_version("m", "f.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        let v2 = repo
            .add_version("m", "f2.enc", "pt", 100, 50, "c2", None, None)
            .unwrap();
        let v3 = repo
            .add_version("m", "f3.enc", "pt", 100, 50, "c3", None, None)
            .unwrap();
        assert_eq!(v1.version, 1);
        assert_eq!(v2.version, 2);
        assert_eq!(v3.version, 3);
    }

    #[test]
    fn test_sqlite_update_metadata_multiple_keys() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        repo.add_version("m", "f.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();

        repo.update_metadata("m", 1, "key1", "val1".to_string())
            .unwrap();
        repo.update_metadata("m", 1, "key2", "val2".to_string())
            .unwrap();
        repo.update_metadata("m", 1, "key3", "val3".to_string())
            .unwrap();

        assert_eq!(repo.get_metadata("m", 1, "key1"), Some("val1".to_string()));
        assert_eq!(repo.get_metadata("m", 1, "key2"), Some("val2".to_string()));
        assert_eq!(repo.get_metadata("m", 1, "key3"), Some("val3".to_string()));
    }

    #[test]
    fn test_sqlite_cleanup_keeps_latest() {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        for i in 0..10 {
            repo.add_version(
                "m",
                &format!("f{}.enc", i),
                "pt",
                100,
                50,
                &format!("c{}", i),
                None,
                if i > 0 { Some(i as u32) } else { None },
            )
            .unwrap();
        }

        let deleted = repo.cleanup_old_versions("m", 3).unwrap();
        assert_eq!(deleted.len(), 7);
        let remaining = repo.list_versions("m");
        assert_eq!(remaining.len(), 3);
        // Kept should be the 3 most recent (v8, v9, v10)
        let versions: Vec<u32> = remaining.iter().map(|v| v.version).collect();
        assert!(versions.contains(&8));
        assert!(versions.contains(&9));
        assert!(versions.contains(&10));
    }

    #[test]
    fn test_sqlite_migration_no_json_file() {
        // When no versions.json exists, migration should be a no-op
        let dir = tempfile::tempdir().unwrap();
        let repo = SqliteVersionRepo::new(dir.path()).unwrap();
        assert!(repo.list_models().is_empty());
    }

    #[test]
    fn test_sqlite_migration_with_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let vault_path = dir.path();

        let mut versions: HashMap<String, Vec<ModelVersion>> = HashMap::new();
        let mut meta = HashMap::new();
        meta.insert("author".to_string(), "migrator".to_string());
        meta.insert("task".to_string(), "classification".to_string());

        versions.insert(
            "migrated_model".to_string(),
            vec![ModelVersion {
                version: 1,
                checkpoint_id: "mig-v1".to_string(),
                timestamp: Utc::now(),
                parent_version: None,
                format: "safetensors".to_string(),
                size_bytes: 5000,
                compressed_size_bytes: 2500,
                checksum_sha256: "mig_chk".to_string(),
                metadata: meta,
                file_path: "migrated.enc".to_string(),
            }],
        );

        std::fs::write(
            vault_path.join("versions.json"),
            serde_json::to_string(&versions).unwrap(),
        )
        .unwrap();

        let repo = SqliteVersionRepo::new(vault_path).unwrap();
        let v = repo.get_version("migrated_model", Some(1)).unwrap();
        assert_eq!(v.format, "safetensors");
        assert_eq!(v.metadata.get("author"), Some(&"migrator".to_string()));
        assert_eq!(v.metadata.get("task"), Some(&"classification".to_string()));
    }

    #[test]
    fn test_sqlite_disk_persistence_metadata() {
        // Verify metadata survives close + reopen
        let dir = tempfile::tempdir().unwrap();
        {
            let mut repo = SqliteVersionRepo::new(dir.path()).unwrap();
            let mut meta = HashMap::new();
            meta.insert("framework".to_string(), "pytorch".to_string());
            repo.add_version("persist", "f.enc", "pt", 100, 50, "c1", Some(meta), None)
                .unwrap();
            repo.update_metadata("persist", 1, "extra", "value".to_string())
                .unwrap();
        }
        // Reopen
        let repo2 = SqliteVersionRepo::new(dir.path()).unwrap();
        let v = repo2.get_version("persist", Some(1)).unwrap();
        assert_eq!(v.metadata.get("framework"), Some(&"pytorch".to_string()));
        assert_eq!(v.metadata.get("extra"), Some(&"value".to_string()));
    }

    #[test]
    fn test_sqlite_generate_checkpoint_id() {
        let id1 = SqliteVersionRepo::generate_checkpoint_id("model", 1);
        let id2 = SqliteVersionRepo::generate_checkpoint_id("model", 1);
        assert!(id1.starts_with("model-v1-"));
        assert!(id2.starts_with("model-v1-"));
        // UUIDs should differ
        assert_ne!(id1, id2);
    }
}
