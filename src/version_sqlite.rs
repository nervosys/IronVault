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
use crate::crypto::VaultCrypto;
#[cfg(feature = "sqlite")]
use crate::error::{Result, VaultError};
#[cfg(feature = "sqlite")]
use crate::version::ModelVersion;

#[cfg(feature = "sqlite")]
use hmac::{Hmac, Mac};
#[cfg(feature = "sqlite")]
use sha2::Sha256;

#[cfg(feature = "sqlite")]
type HmacSha256 = Hmac<Sha256>;

/// One row of the sealed index, before it is sealed.
///
/// The model name travels inside the payload rather than in a column: the
/// `model_tag` column is a blind index, which can be matched but not read, so
/// the name has to come back from somewhere when the cache is rebuilt.
#[cfg(feature = "sqlite")]
#[derive(serde::Serialize, serde::Deserialize)]
struct SealedRow {
    model_name: String,
    version: ModelVersion,
}

/// The keys the sealed index needs, derived from the vault key at `unlock`.
///
/// See `docs/SQLITE_INDEX_SEALING.md`. Two derivations:
/// `tag_subkey = HMAC(vault_key, TAG_CONTEXT)`, and then
/// `model_tag = HMAC(tag_subkey, model_name)`. The vault key itself seals the
/// payload; it is never used directly as the blind-index key, so a tag leaks
/// nothing about the key that decrypts the row beside it.
#[cfg(feature = "sqlite")]
struct IndexSeal {
    key: crate::crypto::SecureKey,
    tag_subkey: [u8; 32],
}

#[cfg(feature = "sqlite")]
impl IndexSeal {
    const TAG_CONTEXT: &'static [u8] = b"IRONVAULT-INDEX-TAG-v1";

    fn new(key: &crate::crypto::SecureKey) -> Result<Self> {
        let mut mac = HmacSha256::new_from_slice(key.as_bytes())
            .map_err(|e| VaultError::CryptoError(format!("Blind index key: {}", e)))?;
        mac.update(Self::TAG_CONTEXT);
        let mut tag_subkey = [0u8; 32];
        tag_subkey.copy_from_slice(&mac.finalize().into_bytes());

        Ok(Self {
            key: crate::crypto::SecureKey::from_bytes(key.as_bytes())?,
            tag_subkey,
        })
    }

    /// Deterministic blind index over a model name.
    ///
    /// Deterministic on purpose — equality matching is the whole point, and it
    /// is also the documented leak: two rows sharing a tag are the same model.
    fn model_tag(&self, model: &str) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(&self.tag_subkey)
            .expect("HMAC-SHA256 accepts a 32-byte key");
        mac.update(model.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }

    fn seal_row(&self, model: &str, mv: &ModelVersion) -> Result<Vec<u8>> {
        let row = SealedRow {
            model_name: model.to_string(),
            version: mv.clone(),
        };
        let plain = serde_json::to_vec(&row)?;
        VaultCrypto::new()?.encrypt(&plain, &self.key)
    }

    fn open_row(&self, payload: &[u8]) -> Result<SealedRow> {
        let plain = VaultCrypto::new()?
            .decrypt(payload, &self.key)
            .map_err(|_| VaultError::AuthenticationFailed)?;
        Ok(serde_json::from_slice(&plain)?)
    }
}

/// Schema for the sealed index. `version` stays plaintext so `ORDER BY` and
/// the uniqueness constraint still work; everything else is in `payload`.
#[cfg(feature = "sqlite")]
const SEALED_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS versions_sealed (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        model_tag BLOB NOT NULL,
        version INTEGER NOT NULL,
        payload BLOB NOT NULL,
        UNIQUE(model_tag, version)
    );
    CREATE INDEX IF NOT EXISTS idx_sealed_tag ON versions_sealed(model_tag);";

/// SQLite-backed version repository with ACID guarantees.
///
/// Maintains an in-memory cache synchronized with the database
/// so that the `VersionRepo` trait (which returns references) works.
#[cfg(feature = "sqlite")]
pub struct SqliteVersionRepo {
    conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    vault_path: PathBuf,
    /// In-memory cache for reference-returning trait methods.
    ///
    /// Empty until `unlock`: rebuilding it means decrypting payloads, and the
    /// key does not exist at construction.
    cache: HashMap<String, Vec<ModelVersion>>,
    /// Set by `unlock`. Reads see an empty cache and writes are refused until
    /// it is — the index cannot be sealed without the key.
    seal: Option<IndexSeal>,
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

        // Enable WAL mode for better concurrent read performance.
        //
        // `secure_delete` and `temp_store` are here for the sealed index in
        // docs/SQLITE_INDEX_SEALING.md: once rows carry AEAD payloads instead
        // of plaintext names, a deleted row left on the freelist, or a spill
        // to a temp file, is the residue that sealing is supposed to remove.
        // Setting them now means an existing vault is already configured when
        // its rows are sealed, rather than sealing rows into a database that
        // still leaks the pages behind them. They cost nothing today: the
        // index is plaintext, so there is nothing yet to protect.
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             PRAGMA secure_delete=ON;
             PRAGMA temp_store=MEMORY;",
        )
        .map_err(|e| VaultError::StorageError(format!("Failed to set PRAGMA: {}", e)))?;

        // Create the sealed schema. The legacy plaintext `versions` /
        // `version_metadata` tables are deliberately not created here: an
        // existing vault still has them and `unlock` migrates them away, but a
        // new vault must never grow them.
        conn.execute_batch(SEALED_SCHEMA).map_err(|e| {
            VaultError::StorageError(format!("Failed to create sealed index: {}", e))
        })?;

        // No `reload_cache` here, and no migration either. Both need the vault
        // key to read a sealed payload, and the constructor runs before the
        // vault is unlocked. Until `unlock`, this repo reports an empty index.
        Ok(Self {
            conn: std::sync::Arc::new(std::sync::Mutex::new(conn)),
            vault_path: vault_path.to_path_buf(),
            cache: HashMap::new(),
            seal: None,
        })
    }

    /// Create an in-memory SQLite version repository (for testing).
    pub fn in_memory() -> Result<Self> {
        let conn = rusqlite::Connection::open_in_memory().map_err(|e| {
            VaultError::StorageError(format!("Failed to create in-memory DB: {}", e))
        })?;

        conn.execute_batch(SEALED_SCHEMA)
            .map_err(|e| VaultError::StorageError(format!("Failed to create tables: {}", e)))?;

        Ok(Self {
            conn: std::sync::Arc::new(std::sync::Mutex::new(conn)),
            vault_path: PathBuf::new(),
            cache: HashMap::new(),
            seal: None,
        })
    }

    /// The seal, or an error naming what the caller forgot.
    fn seal(&self) -> Result<&IndexSeal> {
        self.seal.as_ref().ok_or_else(|| {
            VaultError::SecurityViolation(
                "the version index is sealed; unlock the vault first".to_string(),
            )
        })
    }

    /// Migrate legacy `versions.json` into the database.
    fn migrate_from_json(&self, seal: &IndexSeal) -> Result<Vec<(String, ModelVersion)>> {
        let json_path = self.vault_path.join("versions.json");
        if !json_path.exists() {
            return Ok(Vec::new());
        }

        // Two shapes: sealed from 8.0, plaintext JSON before it. Told apart by
        // the marker rather than by trying to parse and seeing what happens.
        let raw = std::fs::read(&json_path)?;
        let plain = if let Some(ciphertext) = raw.strip_prefix(crate::version::SEALED_INDEX_MAGIC) {
            VaultCrypto::new()?
                .decrypt(ciphertext, &seal.key)
                .map_err(|_| VaultError::AuthenticationFailed)?
        } else {
            raw
        };
        let versions: HashMap<String, Vec<ModelVersion>> = serde_json::from_slice(&plain)?;

        let mut rows = Vec::new();
        for (model_name, model_versions) in versions {
            for mv in model_versions {
                rows.push((model_name.clone(), mv));
            }
        }

        // The rename happens in `unlock`, once the rows are durably sealed.
        Ok(rows)
    }

    /// Read a legacy plaintext `versions` table, if one is still there.
    ///
    /// Returns the rows so that `unlock` can write them into a fresh file.
    /// Nothing is deleted here: an in-place rewrite would leave every
    /// plaintext row on the freelist, which is the residue the seal exists to
    /// remove. See `docs/SQLITE_INDEX_SEALING.md`.
    fn read_legacy_rows(&self) -> Result<Vec<(String, ModelVersion)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;

        let has_legacy = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='versions'",
                [],
                |r| r.get::<_, String>(0),
            )
            .is_ok();
        if !has_legacy {
            return Ok(Vec::new());
        }

        let mut stmt = conn
            .prepare(
                "SELECT model_name, version, checkpoint_id, timestamp, parent_version,
                        format, size_bytes, compressed_size_bytes, checksum_sha256, file_path
                 FROM versions ORDER BY model_name, version",
            )
            .map_err(|e| VaultError::StorageError(format!("Prepare failed: {}", e)))?;

        let mut rows: Vec<(String, ModelVersion)> = stmt
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
                        metadata: HashMap::new(),
                    },
                ))
            })
            .map_err(|e| VaultError::StorageError(format!("Legacy query failed: {}", e)))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| VaultError::StorageError(format!("Legacy row read failed: {}", e)))?;

        // Fold the metadata table back into the rows it belongs to.
        let mut meta_stmt = conn
            .prepare("SELECT model_name, version, key, value FROM version_metadata")
            .map_err(|e| VaultError::StorageError(format!("Prepare failed: {}", e)))?;
        let metas = meta_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| VaultError::StorageError(format!("Meta query failed: {}", e)))?;

        for meta in metas {
            let (model_name, version, key, value) =
                meta.map_err(|e| VaultError::StorageError(format!("Meta read failed: {}", e)))?;
            if let Some((_, mv)) = rows
                .iter_mut()
                .find(|(m, v)| m == &model_name && v.version == version)
            {
                mv.metadata.insert(key, value);
            }
        }

        Ok(rows)
    }

    /// Reload the in-memory cache by opening every sealed row.
    ///
    /// An unfiltered scan, as before — the backend has always loaded the whole
    /// table and answered reads from memory, which is what makes a blind index
    /// sufficient here.
    fn reload_cache(&mut self) -> Result<()> {
        let seal = self.seal()?;
        let mut cache: HashMap<String, Vec<ModelVersion>> = HashMap::new();

        {
            let conn = self
                .conn
                .lock()
                .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;

            let mut stmt = conn
                .prepare("SELECT payload FROM versions_sealed ORDER BY model_tag, version")
                .map_err(|e| VaultError::StorageError(format!("Prepare failed: {}", e)))?;

            let payloads = stmt
                .query_map([], |row| row.get::<_, Vec<u8>>(0))
                .map_err(|e| VaultError::StorageError(format!("Query failed: {}", e)))?;

            for payload in payloads {
                let payload =
                    payload.map_err(|e| VaultError::StorageError(format!("Row read: {}", e)))?;
                let row = seal.open_row(&payload)?;
                cache.entry(row.model_name).or_default().push(row.version);
            }
        }

        for versions in cache.values_mut() {
            versions.sort_by_key(|mv| mv.version);
        }

        self.cache = cache;
        Ok(())
    }

    /// Write `rows` into a brand-new database file and swap it in.
    ///
    /// Deliberately not an in-place `UPDATE`: rewriting rows in the existing
    /// file leaves every plaintext row on the freelist and in the WAL, where
    /// it stays recoverable for the life of the file. Building a new file and
    /// deleting the old one takes those pages with it.
    fn rewrite_sealed(&mut self, rows: &[(String, ModelVersion)]) -> Result<()> {
        let seal = IndexSeal::new(&self.seal()?.key)?;
        let db_path = self.vault_path.join("versions.db");
        let new_path = self.vault_path.join("versions.db.new");
        let _ = std::fs::remove_file(&new_path);

        {
            let fresh = rusqlite::Connection::open(&new_path).map_err(|e| {
                VaultError::StorageError(format!("Failed to open new index: {}", e))
            })?;
            fresh
                .execute_batch(
                    "PRAGMA journal_mode=WAL;
                     PRAGMA foreign_keys=ON;
                     PRAGMA secure_delete=ON;
                     PRAGMA temp_store=MEMORY;",
                )
                .map_err(|e| VaultError::StorageError(format!("Failed to set PRAGMA: {}", e)))?;
            fresh.execute_batch(SEALED_SCHEMA).map_err(|e| {
                VaultError::StorageError(format!("Failed to create sealed index: {}", e))
            })?;

            let tx = fresh
                .unchecked_transaction()
                .map_err(|e| VaultError::StorageError(format!("Transaction failed: {}", e)))?;
            for (model_name, mv) in rows {
                tx.execute(
                    "INSERT OR IGNORE INTO versions_sealed (model_tag, version, payload)
                     VALUES (?1, ?2, ?3)",
                    rusqlite::params![
                        seal.model_tag(model_name),
                        mv.version,
                        seal.seal_row(model_name, mv)?
                    ],
                )
                .map_err(|e| VaultError::StorageError(format!("Seal insert failed: {}", e)))?;
            }
            tx.commit()
                .map_err(|e| VaultError::StorageError(format!("Seal commit failed: {}", e)))?;

            // Fold the WAL back in so the new file is self-contained before it
            // is renamed over the old one.
            fresh
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|e| VaultError::StorageError(format!("Checkpoint failed: {}", e)))?;
        }

        // Drop the handle on the old file before touching it — Windows will
        // not rename over an open database.
        {
            let placeholder = rusqlite::Connection::open_in_memory().map_err(|e| {
                VaultError::StorageError(format!("Failed to detach old index: {}", e))
            })?;
            let mut guard = self
                .conn
                .lock()
                .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;
            *guard = placeholder;
        }

        // The old file's WAL and shm hold plaintext pages too.
        let _ = std::fs::remove_file(self.vault_path.join("versions.db-wal"));
        let _ = std::fs::remove_file(self.vault_path.join("versions.db-shm"));
        let _ = std::fs::remove_file(self.vault_path.join("versions.db.new-wal"));
        let _ = std::fs::remove_file(self.vault_path.join("versions.db.new-shm"));
        std::fs::rename(&new_path, &db_path)?;

        let conn = rusqlite::Connection::open(&db_path).map_err(|e| {
            VaultError::StorageError(format!("Failed to reopen sealed index: {}", e))
        })?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             PRAGMA secure_delete=ON;
             PRAGMA temp_store=MEMORY;",
        )
        .map_err(|e| VaultError::StorageError(format!("Failed to set PRAGMA: {}", e)))?;
        {
            let mut guard = self
                .conn
                .lock()
                .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;
            *guard = conn;
        }

        Ok(())
    }

    /// Generate unique checkpoint identifier.
    fn generate_checkpoint_id(model_name: &str, version: u32) -> String {
        let uuid = Uuid::new_v4();
        format!("{}-v{}-{}", model_name, version, uuid)
    }

    /// Return the vault directory path.
    /// Hand the vault key to the backend.
    ///
    /// Migrates a `versions.json` left by the JSON backend, sealed or not, and
    /// renames it aside. This is where migration belongs rather than in the
    /// constructor: an 8.0 index is sealed, so reading it needs the key, and
    /// the constructor runs before the vault is unlocked.
    ///
    /// # Errors
    ///
    /// [`VaultError::AuthenticationFailed`] if `key` does not open a sealed
    /// index.
    pub fn unlock(&mut self, key: &crate::crypto::SecureKey) -> Result<()> {
        self.seal = Some(IndexSeal::new(key)?);

        // Two legacy shapes reach this point, and a vault can carry both: a
        // plaintext `versions` table from a pre-sealing SQLite backend, and a
        // `versions.json` left by the JSON backend. Collect both, then write
        // them into one fresh file.
        let mut pending = self.read_legacy_rows()?;
        let from_json = self.migrate_from_json(self.seal()?)?;
        let had_json = !from_json.is_empty();
        pending.extend(from_json);

        if !pending.is_empty() && !self.vault_path.as_os_str().is_empty() {
            self.rewrite_sealed(&pending)?;

            if had_json {
                let json_path = self.vault_path.join("versions.json");
                if json_path.exists() {
                    std::fs::rename(&json_path, self.vault_path.join("versions.json.migrated"))?;
                }
            }
        }

        self.reload_cache()
    }

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
        let seal = self.seal()?;
        let tag = seal.model_tag(model);

        // Next version number, matched on the blind index rather than the
        // name. This is one of the three predicates the seal has to preserve.
        let version = {
            let conn = self
                .conn
                .lock()
                .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;
            let max_ver: Option<u32> = conn
                .query_row(
                    "SELECT MAX(version) FROM versions_sealed WHERE model_tag = ?1",
                    rusqlite::params![tag],
                    |row| row.get(0),
                )
                .unwrap_or(None);
            max_ver.unwrap_or(0) + 1
        };

        let mv = ModelVersion {
            version,
            checkpoint_id: Self::generate_checkpoint_id(model, version),
            timestamp: Utc::now(),
            parent_version,
            format: format.to_string(),
            size_bytes,
            compressed_size_bytes,
            checksum_sha256: checksum.to_string(),
            metadata: metadata.unwrap_or_default(),
            file_path: file_path.to_string(),
        };

        let payload = seal.seal_row(model, &mv)?;
        {
            let conn = self
                .conn
                .lock()
                .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;
            conn.execute(
                "INSERT INTO versions_sealed (model_tag, version, payload) VALUES (?1, ?2, ?3)",
                rusqlite::params![tag, version, payload],
            )
            .map_err(|e| VaultError::StorageError(format!("Insert version failed: {}", e)))?;
        }

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
        let tag = self.seal()?.model_tag(model);
        let deleted = {
            let conn = self
                .conn
                .lock()
                .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;

            let count = conn
                .execute(
                    "DELETE FROM versions_sealed WHERE model_tag = ?1 AND version = ?2",
                    rusqlite::params![tag, version],
                )
                .map_err(|e| VaultError::StorageError(format!("Delete failed: {}", e)))?;

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
        let tag = self.seal()?.model_tag(model);
        let to_delete: Vec<u32> = {
            let conn = self
                .conn
                .lock()
                .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;

            // Get versions to delete (all except the N most recent)
            let mut stmt = conn
                .prepare(
                    "SELECT version FROM versions_sealed WHERE model_tag = ?1
                     ORDER BY version DESC",
                )
                .map_err(|e| VaultError::StorageError(format!("Prepare failed: {}", e)))?;

            let all_versions: Vec<u32> = stmt
                .query_map(rusqlite::params![tag], |row| row.get(0))
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
            let checksum = hex::encode(VaultCrypto::hash_sha256(data));
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
        // Metadata is no longer a table of its own — it lives inside the
        // sealed payload, so an update means re-sealing the whole row. The
        // cache is mutated first and is the source for the new payload.
        let updated = {
            let versions = self
                .cache
                .get_mut(model)
                .ok_or_else(|| VaultError::VersionNotFound(version, model.to_string()))?;
            let mv = versions
                .iter_mut()
                .find(|v| v.version == version)
                .ok_or_else(|| VaultError::VersionNotFound(version, model.to_string()))?;
            mv.metadata.insert(key.to_string(), value);
            mv.clone()
        };

        let seal = self.seal()?;
        let tag = seal.model_tag(model);
        let payload = seal.seal_row(model, &updated)?;

        let conn = self
            .conn
            .lock()
            .map_err(|e| VaultError::StorageError(format!("Lock poisoned: {}", e)))?;
        conn.execute(
            "UPDATE versions_sealed SET payload = ?1 WHERE model_tag = ?2 AND version = ?3",
            rusqlite::params![payload, tag, version],
        )
        .map_err(|e| VaultError::StorageError(format!("Upsert metadata failed: {}", e)))?;

        Ok(())
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
    /// Migration moved out of the constructor into `unlock`, because from 8.0
    /// the JSON index is sealed and reading it needs the vault key.
    fn migration_key() -> crate::crypto::SecureKey {
        crate::crypto::VaultCrypto::new()
            .unwrap()
            .derive_key(b"sqlite-migration-tests".to_vec(), Some(vec![13u8; 16]))
            .unwrap()
            .0
    }

    /// An unlocked in-memory repo.
    ///
    /// Every write now needs the vault key, so a test repo has to be unlocked
    /// the same way a real one is. `in_memory()` alone is a locked repo.
    fn sealed_in_memory() -> SqliteVersionRepo {
        let mut repo = SqliteVersionRepo::in_memory().unwrap();
        repo.unlock(&migration_key()).unwrap();
        repo
    }

    /// An unlocked on-disk repo.
    fn sealed_on_disk(path: &std::path::Path) -> SqliteVersionRepo {
        let mut repo = SqliteVersionRepo::new(path).unwrap();
        repo.unlock(&migration_key()).unwrap();
        repo
    }

    use super::*;
    use crate::traits::VersionRepo;

    #[test]
    fn test_sqlite_version_repo_basic() {
        let mut repo = sealed_in_memory();

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
        let mut repo = sealed_in_memory();

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
            let mut repo = sealed_on_disk(vault_path);
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
        let reopened = sealed_on_disk(vault_path);
        assert_eq!(reopened.list_models(), vec!["keep".to_string()]);
    }

    #[test]
    fn test_sqlite_version_repo_cleanup() {
        let mut repo = sealed_in_memory();

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
        let mut repo = sealed_in_memory();

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
        let mut repo = sealed_in_memory();

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
        let mut repo = sealed_on_disk(vault_path);
        repo.unlock(&migration_key()).unwrap();

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
            let mut repo = sealed_on_disk(vault_path);
            repo.add_version("disk_model", "f.enc", "onnx", 500, 250, "chk1", None, None)
                .unwrap();
            repo.update_metadata("disk_model", 1, "source", "test".to_string())
                .unwrap();
        }

        // Reopen — exercises reload_cache (lines 129-300+)
        let repo2 = sealed_on_disk(vault_path);
        let v = repo2.get_version("disk_model", Some(1)).unwrap();
        assert_eq!(v.format, "onnx");
        assert_eq!(v.metadata.get("source"), Some(&"test".to_string()));
    }

    #[test]
    fn the_database_file_holds_no_plaintext_model_name() {
        // The property the whole exercise exists for. A model name identifies
        // a customer and a use case without decrypting a byte, and on this
        // backend it used to sit in a TEXT column.
        let temp_dir = tempfile::tempdir().unwrap();
        let vault_path = temp_dir.path();
        const NAME: &str = "acme-fraud-detection-v3";

        {
            let mut repo = sealed_on_disk(vault_path);
            repo.add_version(NAME, "f.enc", "onnx", 10, 5, "chk", None, None)
                .unwrap();
            repo.update_metadata(NAME, 1, "customer", "acme".to_string())
                .unwrap();
        }

        // Every file the database spans, not just the main one — the WAL is
        // where a row lands first.
        for suffix in ["", "-wal", "-shm"] {
            let path = vault_path.join(format!("versions.db{}", suffix));
            if !path.exists() {
                continue;
            }
            let bytes = std::fs::read(&path).unwrap();
            assert!(
                !bytes.windows(NAME.len()).any(|w| w == NAME.as_bytes()),
                "model name found in plaintext in versions.db{}",
                suffix
            );
            assert!(
                !bytes.windows(4).any(|w| w == b"acme"),
                "metadata value found in plaintext in versions.db{}",
                suffix
            );
        }
    }

    #[test]
    fn a_locked_repo_refuses_writes_and_reports_an_empty_index() {
        // Sealing needs the key, so the trait's writes cannot be honoured
        // before unlock. They used to succeed.
        let mut repo = SqliteVersionRepo::in_memory().unwrap();

        let err = repo
            .add_version("m", "f.enc", "onnx", 10, 5, "chk", None, None)
            .unwrap_err();
        assert!(
            matches!(err, VaultError::SecurityViolation(_)),
            "expected a locked-index error, got {:?}",
            err
        );

        assert!(repo.list_models().is_empty());
        assert!(repo.get_version("m", None).is_none());
    }

    #[test]
    fn a_legacy_plaintext_database_migrates_into_sealed_rows() {
        let temp_dir = tempfile::tempdir().unwrap();
        let vault_path = temp_dir.path();

        // Hand-build the pre-sealing schema and put a row in it.
        {
            let conn = rusqlite::Connection::open(vault_path.join("versions.db")).unwrap();
            conn.execute_batch(
                "CREATE TABLE versions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    model_name TEXT NOT NULL, version INTEGER NOT NULL,
                    checkpoint_id TEXT NOT NULL, timestamp TEXT NOT NULL,
                    parent_version INTEGER, format TEXT NOT NULL,
                    size_bytes INTEGER NOT NULL, compressed_size_bytes INTEGER NOT NULL,
                    checksum_sha256 TEXT NOT NULL, file_path TEXT NOT NULL,
                    UNIQUE(model_name, version));
                 CREATE TABLE version_metadata (
                    model_name TEXT NOT NULL, version INTEGER NOT NULL,
                    key TEXT NOT NULL, value TEXT NOT NULL,
                    PRIMARY KEY(model_name, version, key));",
            )
            .unwrap();
            conn.execute(
                "INSERT INTO versions (model_name, version, checkpoint_id, timestamp,
                    parent_version, format, size_bytes, compressed_size_bytes,
                    checksum_sha256, file_path)
                 VALUES ('old_model', 1, 'old-v1', ?1, NULL, 'pytorch', 42, 21, 'sum', 'old.enc')",
                rusqlite::params![Utc::now().to_rfc3339()],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO version_metadata VALUES ('old_model', 1, 'note', 'kept')",
                [],
            )
            .unwrap();
        }

        let repo = sealed_on_disk(vault_path);

        // The row survived, metadata included.
        assert_eq!(repo.list_models(), vec!["old_model"]);
        let v = repo.get_version("old_model", Some(1)).unwrap();
        assert_eq!(v.format, "pytorch");
        assert_eq!(v.size_bytes, 42);
        assert_eq!(v.metadata.get("note"), Some(&"kept".to_string()));

        // The plaintext table is gone, not merely emptied — the migration
        // builds a new file rather than rewriting rows in place.
        let conn = repo.conn.lock().unwrap();
        let legacy_exists = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='versions'",
                [],
                |r| r.get::<_, String>(0),
            )
            .is_ok();
        assert!(!legacy_exists, "legacy plaintext table survived migration");

        let sealed_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM versions_sealed", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sealed_rows, 1);
        drop(conn);

        // And the old name is not recoverable from the file.
        let bytes = std::fs::read(vault_path.join("versions.db")).unwrap();
        assert!(
            !bytes.windows(9).any(|w| w == b"old_model"),
            "migrated-away name still present in the database file"
        );
    }

    #[test]
    fn a_wrong_key_does_not_open_a_sealed_index() {
        let temp_dir = tempfile::tempdir().unwrap();
        let vault_path = temp_dir.path();
        {
            let mut repo = sealed_on_disk(vault_path);
            repo.add_version("m", "f.enc", "onnx", 10, 5, "chk", None, None)
                .unwrap();
        }

        let other = crate::crypto::VaultCrypto::new()
            .unwrap()
            .derive_key(b"a-different-passphrase".to_vec(), Some(vec![7u8; 16]))
            .unwrap()
            .0;

        let mut repo = SqliteVersionRepo::new(vault_path).unwrap();
        let err = repo.unlock(&other).unwrap_err();
        assert!(
            matches!(err, VaultError::AuthenticationFailed),
            "expected AuthenticationFailed, got {:?}",
            err
        );
    }

    #[test]
    fn test_sqlite_residue_pragmas_are_set() {
        // The sealed index in docs/SQLITE_INDEX_SEALING.md depends on these:
        // a deleted row must not stay readable on the freelist, and nothing
        // may spill to a temp file. Both are per-connection, so this asserts
        // the state of a connection new() actually handed back, not that the
        // batch was merely executed.
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = sealed_on_disk(temp_dir.path());
        let conn = repo.conn.lock().unwrap();

        let secure_delete: i64 = conn
            .query_row("PRAGMA secure_delete;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(secure_delete, 1, "secure_delete must be on");

        // 2 == MEMORY
        let temp_store: i64 = conn
            .query_row("PRAGMA temp_store;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(temp_store, 2, "temp_store must be MEMORY");

        // The pragmas above must not have displaced the two that were
        // already load-bearing.
        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(journal_mode.to_lowercase(), "wal");

        let foreign_keys: i64 = conn
            .query_row("PRAGMA foreign_keys;", [], |r| r.get(0))
            .unwrap();
        assert_eq!(foreign_keys, 1);
    }

    #[test]
    fn test_sqlite_verify_checksum() {
        // Covers lines 497, 501, 509
        let mut repo = sealed_in_memory();
        let data = b"model data";
        let checksum = hex::encode(VaultCrypto::hash_sha256(data));
        repo.add_version("m", "f.enc", "pt", 100, 50, &checksum, None, None)
            .unwrap();

        assert!(repo.verify_checksum("m", 1, data));
        assert!(!repo.verify_checksum("m", 1, b"tampered"));
        assert!(!repo.verify_checksum("m", 99, data)); // nonexistent version
    }

    #[test]
    fn test_sqlite_get_version_none() {
        let repo = sealed_in_memory();
        assert!(repo.get_version("nonexistent", None).is_none());
        assert!(repo.get_version("nonexistent", Some(1)).is_none());
    }

    #[test]
    fn test_sqlite_add_version_with_metadata() {
        let mut repo = sealed_in_memory();
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
        let mut repo = sealed_in_memory();
        repo.add_version("m", "f.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        let result = repo.update_metadata("m", 99, "key", "val".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_sqlite_get_metadata_none() {
        let mut repo = sealed_in_memory();
        repo.add_version("m", "f.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        assert!(repo.get_metadata("m", 1, "nonexistent").is_none());
        assert!(repo.get_metadata("m", 99, "any").is_none());
    }

    #[test]
    fn test_sqlite_lineage_chain() {
        let mut repo = sealed_in_memory();
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
        let mut repo = sealed_in_memory();
        repo.add_version("m", "f.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        let deleted = repo.cleanup_old_versions("m", 10).unwrap();
        assert!(deleted.is_empty());
    }

    #[test]
    fn test_sqlite_vault_path() {
        let dir = tempfile::tempdir().unwrap();
        let repo = sealed_on_disk(dir.path());
        assert_eq!(repo.vault_path(), dir.path());
    }

    #[test]
    fn test_sqlite_delete_nonexistent_model() {
        let mut repo = sealed_in_memory();
        // Delete from a model that doesn't exist — should return Ok(false)
        let result = repo.delete_version("no_model", 1).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_sqlite_cleanup_empty_model() {
        let mut repo = sealed_in_memory();
        // Cleanup on a model with no versions — should return empty vec
        let deleted = repo.cleanup_old_versions("no_model", 5).unwrap();
        assert!(deleted.is_empty());
    }

    #[test]
    fn test_sqlite_list_versions_empty() {
        let repo = sealed_in_memory();
        let versions = repo.list_versions("nonexistent");
        assert!(versions.is_empty());
    }

    #[test]
    fn test_sqlite_list_models_empty() {
        let repo = sealed_in_memory();
        assert!(repo.list_models().is_empty());
    }

    #[test]
    fn test_sqlite_lineage_nonexistent() {
        let repo = sealed_in_memory();
        let lineage = repo.get_lineage("no_model", 1);
        assert!(lineage.is_empty());
    }

    #[test]
    fn test_sqlite_lineage_no_parent() {
        let mut repo = sealed_in_memory();
        repo.add_version("m", "f.enc", "pt", 100, 50, "c1", None, None)
            .unwrap();
        let lineage = repo.get_lineage("m", 1);
        assert_eq!(lineage.len(), 1);
        assert_eq!(lineage[0].version, 1);
    }

    #[test]
    fn test_sqlite_lineage_broken_chain() {
        // Lineage with a missing parent version
        let mut repo = sealed_in_memory();
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
        let mut repo = sealed_in_memory();
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
        let repo = sealed_in_memory();
        assert!(!repo.verify_checksum("no_model", 1, b"data"));
    }

    #[test]
    fn test_sqlite_get_metadata_nonexistent_model() {
        let repo = sealed_in_memory();
        assert!(repo.get_metadata("no_model", 1, "key").is_none());
    }

    #[test]
    fn test_sqlite_multiple_models_independent() {
        let mut repo = sealed_in_memory();
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
        let mut repo = sealed_in_memory();
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
        let mut repo = sealed_in_memory();
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
        let mut repo = sealed_in_memory();
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
        let repo = sealed_on_disk(dir.path());
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

        let mut repo = sealed_on_disk(vault_path);
        repo.unlock(&migration_key()).unwrap();
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
            let mut repo = sealed_on_disk(dir.path());
            let mut meta = HashMap::new();
            meta.insert("framework".to_string(), "pytorch".to_string());
            repo.add_version("persist", "f.enc", "pt", 100, 50, "c1", Some(meta), None)
                .unwrap();
            repo.update_metadata("persist", 1, "extra", "value".to_string())
                .unwrap();
        }
        // Reopen
        let repo2 = sealed_on_disk(dir.path());
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
