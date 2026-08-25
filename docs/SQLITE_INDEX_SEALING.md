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

## Recommendation

**Option B**, on the strength of the cache finding: the ACID guarantees that
justify `--sqlite` are unaffected by sealing the payload, and the option that
does not add a C dependency to a security tool's build matrix is the cheaper
one to get right and to keep right.

Option A remains the better answer if the WAL and freelist residue matter more
than build cost — that is a threat-model decision, not a technical one, and it
is the one question here worth deciding before any code is written.

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
