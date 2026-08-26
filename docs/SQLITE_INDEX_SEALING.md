# Sealing the SQLite version index

**Status: design, not implemented.** Nothing described here exists in 8.0.1.
`--sqlite` still stores model names, sizes, timestamps and checksums in a plain
database file, exactly as `SECURITY.md` and the doc comment on
`VersionBackend::Sqlite` say it does.

## The problem

8.0 sealed `versions.json` under the vault key. The SQLite backend was not
sealed with it, so the confidentiality of the index now depends on which
backend the user chose:

| | JSON backend | SQLite backend |
|---|---|---|
| Model names | sealed | **plaintext** |
| Sizes, timestamps, formats | sealed | **plaintext** |
| Checksums, blob paths | sealed | **plaintext** |
| User metadata | sealed | **plaintext** |
| Model contents | AEAD-sealed | AEAD-sealed |

Model *contents* are safe on both. The exposure is the inventory, and for a
vault the inventory is not incidental — `acme-fraud-detection-v3` identifies a
customer and a use case without decrypting a byte. That is the argument 8.0
made for sealing the JSON index, and it applies unchanged here.

## The question that was blocking this

Both the CHANGELOG and the handoffs state the open design question as: *how
does a queryable index look up names it cannot read?* That framing assumes the
backend depends on SQL predicates over plaintext names.

**It does not.** `SqliteVersionRepo` already loads the entire table into an
in-memory `HashMap` cache (`reload_cache`, via
`SELECT ... FROM versions ORDER BY model_name, version` — an unfiltered scan),
and answers reads from that cache. Only three statements filter on
`model_name` at all:

- `SELECT MAX(version) FROM versions WHERE model_name = ?1` (next version number)
- `SELECT version FROM versions WHERE model_name = ?1 ORDER BY version DESC` (pruning)
- the two `DELETE ... WHERE model_name = ?1 AND version = ?2`

Every one is answerable from the cache that is already in memory, over a
dataset bounded by the number of model versions in one vault. The database is
not doing work here that the process cannot do itself.

So the blocking question is smaller than recorded. It is not "how do we query
what we cannot read"; it is "what do we keep in the schema so that equality
matching and the `UNIQUE(model_name, version)` constraint still hold". That
has a standard answer.

## Option A — SQLCipher

Whole-file page-level encryption. The schema and every query stay as they are.

**For:** nothing above the storage layer changes; indices, the WAL and the
freelist are all covered, so there is no partially-sealed state to reason
about; it is the option with the least new cryptographic code in this repo.

**Against:** a real build dependency. SQLCipher means either linking a system
library or vendoring an amalgamation, on Windows, macOS and Linux, behind the
existing `sqlite` feature — and the CI matrix is where that cost lands. It
changes the file format, so every existing `--sqlite` vault needs a migration
path, not just new ones. The vault key would have to be handed to the database
layer rather than staying in `SecureKey`.

Note for the compliance surface: adding SQLCipher does not make anything here
FIPS-validated, and nothing in this document should be described that way.
`compliance.rs` holds the accurate wording.

## Option B — sealed row payload with a blind index

Keep SQLite as-is and change what a row contains:

```
versions(
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    model_tag     BLOB NOT NULL,   -- HMAC-SHA256(subkey, model_name)
    version       INTEGER NOT NULL,
    payload       BLOB NOT NULL,   -- AEAD-sealed ModelVersion, as the JSON index seals
    UNIQUE(model_tag, version)
)
```

The name is replaced by a **blind index**: a deterministic HMAC under a subkey
derived from the vault key. You cannot read a name from it, but you can match
one, which is all the three surviving predicates need. `UNIQUE` still holds.
`ORDER BY version` still holds. Everything else about a version moves into the
sealed `payload` and is decrypted into the same cache the backend already
builds.

**For:** no new dependency, no build-matrix cost, reuses `VaultCrypto` and the
`IRONVAULT-VERSIONS-v1` marker convention already in `version.rs`. Migration is
a row-by-row rewrite the backend can do in `unlock`, where the key exists —
the same place, and for the same reason, that the 8.0.1 fix moved the
`versions.json` migration to.

**Against:** more cryptographic code owned by this project, and the seal is
partial in ways worth stating rather than discovering:

- The **row count** is visible — how many model versions exist.
- **Equality is visible.** Two rows with the same `model_tag` are the same
  model. An observer learns the shape of the vault (five models, one with
  forty versions) without learning a name. This is the accepted cost of any
  blind index and should be documented, not glossed.
- `version` stays plaintext so ordering and the constraint work.
- Sizes and timestamps must move into `payload` to be covered, which means
  they stop being SQL-queryable — acceptable, since nothing queries them today.
- **SQLite's own artifacts are not covered**: the WAL, the freelist, and any
  temp store can retain fragments. Deleting a row does not scrub the page.
  Option A covers these and Option B does not.

## Resolving the threat-model question

The question left open above — whether WAL and freelist residue matters more
than build cost — is answerable from the code rather than from taste, and the
answer is that it matters for exactly one window.

**Names exist only in the index.** Blobs under `data/` are named
`{uuid}.vault` (`Uuid::new_v4()`, `vault.rs`), so the filesystem leaks the
number of stored versions and their approximate sizes, but never a model name.
The name in `acme-fraud-detection-v3` — the whole reason for sealing — lives in
the index and nowhere else. That raises the value of sealing the index and
raises the cost of leaving plaintext names recoverable inside the database file.

**After migration, Option B never writes a plaintext name to the database
again.** Every column that reaches disk is either an HMAC tag, an AEAD payload,
or an integer version. So WAL frames, freelist pages and temp spill created
from that point forward contain no plaintext names to recover. The residue
objection does not apply to steady-state operation at all.

**It applies only to the migration itself** — and to that it applies
completely. A row-by-row `UPDATE` in `unlock`, as sketched above, writes every
old plaintext row into the freelist and the WAL, where it stays recoverable for
the life of the file. That would leave the pre-migration inventory readable in
a vault the user believes is sealed, which is worse than the honest plaintext
of today.

The fix is not SQLCipher. It is to migrate by **building a new database file
and replacing the old one**:

1. Create `versions.db.new`, write only sealed rows into it, `fsync`.
2. Atomically rename over `versions.db`; remove `versions.db-wal` and
   `versions.db-shm` from the old file.
3. Remove the old file. The plaintext pages go with it.

Two pragmas close the remaining ongoing exposure, both one-line additions to
the `execute_batch` at `version_sqlite.rs:50`, which today sets only
`journal_mode=WAL` and `foreign_keys=ON`:

- `PRAGMA secure_delete = ON` — zero deleted rows' pages instead of leaving
  them on the freelist. Post-migration those pages hold only ciphertext, so
  this is defence in depth rather than a requirement.
- `PRAGMA temp_store = MEMORY` — keep any spill off disk.

## Recommendation

**Option B**, and the decision above is what makes it decisive rather than a
balance of costs. SQLCipher's one real advantage over a sealed row payload is
coverage of SQLite's own artifacts; that advantage is worth paying a C
dependency for only if those artifacts would otherwise hold recoverable
plaintext. With a fresh-file migration they do not — after the rewrite, nothing
plaintext ever enters the file. The build-matrix cost of SQLCipher buys
protection for a window that a correct migration already closes.

What Option B still does not hide, unchanged from the analysis above and worth
stating in `SECURITY.md` when this lands: the **row count**, and **equality
between rows** — that two versions belong to the same model. Neither is a name,
and `data/` already bounds the count.

Implementation order, once approved:

1. **Done.** `PRAGMA secure_delete=ON` and `PRAGMA temp_store=MEMORY` at
   `version_sqlite.rs:59`, with `test_sqlite_residue_pragmas_are_set`. No
   format change. The fresh-file migration helper is deliberately *not* here —
   until rows are sealed it has no caller, and scrubbing freelist residue is
   pointless while the rows above it are plaintext names.
2. The sealed schema and blind index, behind the migration. Scoped below.
3. Update `SECURITY.md`, the `VersionBackend::Sqlite` doc comment and
   `CHANGELOG.md` together — the three places that currently promise this.

### What step 2 actually costs

Larger than "change the schema", because it changes the backend's contract.

**The repo must retain the vault key.** `unlock` (`version_sqlite.rs:334`)
currently takes the key, runs the JSON migration and drops it. None of
`add_version`, `delete_version`, `cleanup_old_versions` or `update_metadata`
takes a key. Sealing requires one at every write, so `unlock` must store it —
and `new()` can no longer populate the cache, because decrypting payloads
needs a key that does not exist at construction.

**Writes and reads before `unlock` must start failing.** They succeed today.
This is inherent to sealing rather than a design choice, but it is a
behavioural change: all 35 tests in `version_sqlite.rs` build a repo and write
to it without unlocking. `Vault::store_model` is safe — it only reaches
`add_version` with a key in scope (`vault.rs:637`) — but the trait permits
what the sealed backend must refuse.

**Blind index primitive: `hmac` (RustCrypto), a new dependency.** Only `sha2`
is present today. This does not contradict the argument against SQLCipher
above: that argument is about a *C* dependency and its cost across the
three-platform CI matrix, which a pure-Rust crate does not carry. The
alternative — hand-rolling `SHA256(subkey ‖ name)` — is length-extension-
weak and not worth owning in a security tool. Derivation:
`tag_subkey = HMAC(vault_key, "IRONVAULT-INDEX-TAG-v1")`, then
`model_tag = HMAC(tag_subkey, model_name)`.

**Migration covers two shapes, not one:** a legacy plaintext `versions.db`,
and the `versions.json` path that already exists. Both must land in the new
file via the fresh-file rewrite, never an in-place `UPDATE`.

Estimated at ~350 lines plus the test rework. Worth staging as its own branch
with the contract change reviewed separately from the crypto.

## Not in scope

The `data/` directory is unchanged either way. File names and sizes there
remain visible on both backends, which bounds how many models exist and roughly
how large each is. Sealing the index does not change that, and `SECURITY.md`
already says so.

## Where the current state is documented

- `SECURITY.md` — "The SQLite backend is not sealed"
- `VersionBackend::Sqlite` doc comment in `src/vault.rs`
- `CHANGELOG.md`, 8.0.1, "Still not sealed"

All three should be updated together when this is implemented; they are the
three places that currently promise it is coming.
