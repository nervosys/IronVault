# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [5.1.0] - 2026-08-10

### Added

- **The 14 REST endpoints the OpenAPI spec had been promising since 1.x.** `.well-known/openapi.yaml` documented 53 paths against 44 registered routes, and the sets were not nested — 14 documented paths had no handler at all. Any client generated from the published spec called them and got a 404. For a project whose premise is machine-readable discovery, a discovery document describing endpoints that do not exist is worse than not shipping one.

  Now implemented: `/introspect`, `/telemetry/status`, `/license-scan`, `/models/diff`, `/models/pull`, `/vault/export`, `/vault/import`, and the `/models/{name}/` sub-resources `sign`, `verify`, `scan`, `register`, `benchmarks` (GET and POST), `card/validate`, and `card/generate`. The server now registers 58 routes and the spec documents all 56 that are REST resources.

- **`GET /api/v1/introspect` serves the same schema as `iv introspect`.** The builder moved from the binary into the library as `ironvault::cli_schema`, which is why the endpoint could not exist before — the schema was unreachable from the API. Both surfaces now call one function, so they cannot describe different CLIs. A test asserts the HTTP response equals `cli_schema::build(false)` exactly. It is unauthenticated on purpose: it holds no vault data, and requiring a token to learn how to obtain a token is a loop.

- **`tests/openapi_drift_test.rs`.** Reconciling the spec once was the easy part; this is the part that keeps it reconciled. Six assertions: every registered route is documented, every documented path has a handler, the spec parses as YAML, its version matches the crate, every tag used is declared and every declared tag used, and the exemption list stays short and real. The only exemptions are `/` (HTML dashboard) and `/graphql` (a different protocol with its own schema).

### Security

- **The documented shapes for four endpoints were arbitrary file read and write, and were not implemented as written.** The spec had `/license-scan` taking a `path`, `/vault/export` taking an `output`, `/vault/import` taking an `archive`, and `/models/diff` taking "file path or name@version" — all server-side filesystem paths supplied by the caller. Over HTTP that turns any API token into read and write access to every file the server user can reach; `output` alone is a write primitive aimable anywhere.

  Each is scoped instead: a model is addressed by `name` or `name@version` and resolved against the vault, `export` streams the bundle in the response body, and `import` takes the bundle as the request body. `sign` returns the detached signature inline rather than reporting a server-side `signature_path`. The spec was corrected to match the implementation. Nothing depended on the old shapes, because nothing implemented them.

  `tests/api_reconciliation_tests.rs` asserts the rejection directly: a `path` body to `/license-scan` and a filesystem path to `/models/diff` must both fail rather than read the file.

### Fixed

- **Vault bundles are now actually gzipped.** The CLI help, the OpenAPI spec, `AGENTS.md`, and `docs/VAULT_BUNDLE.md` all called the export a `.tar.gz`; through 5.0 it was an uncompressed tar wearing that name. Model blobs compress, so the mislabel also cost real bytes on every export.

  `import_vault` sniffs the gzip magic rather than the file extension, so every bundle written by 5.0 and earlier still imports — extension-based dispatch would have rejected exactly the archives the compatibility exists for. Both directions are tested: a pre-5.1 plain tar must still import *and pass its checksum*, and a gzipped bundle must round-trip.

- **`iv introspect` and `.well-known/ontology.jsonld` minted different IRIs for the same terms**, so a consumer joining the two saw two unrelated vocabularies. Carried over from the 5.0.0 notes and now also covered by the tag consistency test.

- **Twelve OpenAPI tags were used but never declared**, including `Cards` and `Model Cards` as two names for one group. Undeclared tags render as bare strings with no description in generated documentation. All tags are now declared, and the drift test fails if that stops being true in either direction.

## [5.0.0] - 2026-08-10

### Changed

- **The project is now IronVault.** Every identity moves at once, which is why this is a major:

  | | 4.x | 5.0 |
  |---|---|---|
  | crates.io | `ai-model-vault` | `ironvault` |
  | PyPI | `aimodelvault` | `ironvault` |
  | Rust lib path | `ai_model_vault::` | `ironvault::` |
  | Binary | `aim` | `iv` |
  | Environment | `aimodelvault_*`, `AIM_*` | `IRONVAULT_*` |
  | Repository | `nervosys/AIModelVault` | `nervosys/IronVault` |
  | Python import | `import aimodelvault` | `import ironvault` |

  The old crate and PyPI names stay published and installable at 4.6.x. They receive no further releases; there is no meta-package forwarding to the new name, because a package that silently installs something else is worse than one that stops moving.

- **Two environment prefixes collapse into one.** 4.x read `aimodelvault_*` (the package name) and `AIM_*` (the binary name) as unrelated families. Both are now `IRONVAULT_*` — `IRONVAULT_HOME`, `IRONVAULT_PASSPHRASE`, `IRONVAULT_JWT_SECRET`, `IRONVAULT_TELEMETRY_DISABLED`, and the rest.

  **The old names still work in 5.0**, and warn once each to stderr naming only the variable, never its value — several of them carry passphrases. A rename that silently stops reading a deployment's `EnvironmentFile` is a bad rename, so the fallback exists to make the upgrade non-breaking in practice; it is scheduled for removal in 6.0.

  Two mechanisms, because one was not enough. `env::var` covers reads inside the library, but several flags take their value from clap's `#[arg(env = "…")]`, which consults the process environment directly and cannot be intercepted — `--jwt-secret`, `--host`, `--port`, `--revocation-store`. Without the second mechanism those would have been the one family that silently stopped working, so `iv` normalises the environment via `env::migrate_legacy()` before parsing arguments. An explicitly-set new name always wins.

- **systemd installs use `ironvault`, not `iv`.** The unit is `ironvault-server.service`, the service user and group are `ironvault`, state lives in `/var/lib/ironvault`, and credentials in `/etc/ironvault/server.env`. Only the executable is `iv` — a two-letter system user and a `/etc/iv` are needlessly collision-prone for no readability gain. Existing installs are not migrated in place; see `docs/MIGRATION.md`.

- **The format and interface identifiers are renamed too — write-new, read-both.**

  | | 4.x | 5.0 writes | 5.0 reads |
  |---|---|---|---|
  | Envelope magic | `AIMVSEAL` | `IRONSEAL` | both |
  | Stream magic | `AIMV` | `IRNV` | both |
  | URI scheme | `aimv://` | `iv://` | both |
  | JSON-LD prefix | `aimv:` | `iv:` | — |
  | Bundle extension | `.aimvault` | `.ivault` | any |

  Renaming a magic byte string does not rename the bytes already written. Every object sealed into an S3 bucket, every chunked model encrypted inside an existing vault, and every `aimv://` URI stored in someone's manifest still carries the 4.x spelling, and nothing in this project ever rewrites them in place. So the constants move but the readers accept both: `is_sealed`, `open`, `StreamHeader::from_bytes`, `is_chunked_format`, and `IvUri::parse` all take either form, while `seal`, `to_bytes`, and `Display` emit only the new one. A bucket or vault may hold a mix indefinitely and every object in it opens.

  Four tests pin this rather than leaving it to comments: a 4.x-sealed object must still decrypt, a 4.x chunked model must still decrypt, an `aimv://` URI must still parse, and — the other half, which is what stops the compatibility from quietly becoming the behaviour — new writes must carry the new magic and `Display` must never emit the old scheme.

  `.aimvault` → `.ivault` is free: the extension was documented convention, never checked in code.

- **The Rust vault directory does not move.** The XDG layout was already name-neutral (`~/.config/ai/models/`, `~/.local/share/ai/models/`), so existing vaults are found exactly where they were. Only the *Python* package had a name-derived directory; see `docs/MIGRATION.md` for that one.

### Fixed

- **The CLI's JSON-LD and the published ontology minted different IRIs for the same terms.** `iv introspect` bound the `aimv` prefix to one host while `.well-known/ontology.jsonld` bound it to `https://nervosys.com/ontology/aimv#`, so a consumer joining the two saw two unrelated vocabularies. The CLI now uses the published namespace. Pre-existing; found while auditing which names were branding and which were interfaces.

## [4.6.1] - 2026-08-10

### Changed

- **jsonwebtoken 9.3 → 11.0**, with an explicit crypto provider.

  The bump alone is a trap. From 11.0 the crate requires exactly one of the `rust_crypto` / `aws_lc_rs` features; without one it **compiles cleanly and panics at runtime** on the first sign or verify — *"Could not automatically determine the process-level CryptoProvider"*. Dependabot's PR was the bare version bump, so merging it on a green build would have shipped an API where every authenticated request crashes the handler.

  Verified by doing it: in an isolated worktree the bare bump passed `cargo check`, then failed 6 of 10 auth tests, including basic token creation. With `default-features = false, features = ["rust_crypto"]` all 10 pass — including expired-token rejection and invalid-secret rejection, which confirms `Validation::default()` still validates `exp` and the signature.

  `rust_crypto` over `aws_lc_rs`: it matches the pure-Rust posture of the rest of the crypto stack (`aes-gcm`, `argon2`) and keeps the build toolchain-free. `aws_lc_rs` was tried and rejected — `aws-lc-sys` hard-requires NASM on `x86_64-pc-windows-msvc` and panics the build without it, so it would break `cargo install ai-model-vault --features api` for every Windows user.

  This is the case for reading a major bump rather than trusting a green check — the compiler had nothing to say about it.

### Security

- **RUSTSEC-2023-0071 (`rsa` 0.9.10) is now suppressed, with a test holding the justification in place.** The `rust_crypto` provider above is all-or-nothing: it cannot be selected without `rsa`, which carries an unfixed timing sidechannel (Marvin attack) and no upgrade path. Taking the jsonwebtoken bump therefore turned the Security Audit red — a regression this release introduced and is closing here.

  The advisory is unreachable in this crate. Every JWT is signed and verified with HMAC: `src/api/auth.rs` uses `Header::default()` and `Validation::default()` (HS256) with `EncodingKey::from_secret` / `DecodingKey::from_secret`. No RSA key is ever constructed, so the RSA decryption path that leaks timing never runs.

  "Unreachable" is a claim that rots, so it is now pinned rather than asserted: `tests/jwt_algorithm_test.rs` fails if issued tokens stop being HS256, and fails if a token whose header advertises `RS256` is ever accepted. Introducing an RSA algorithm breaks the build and forces the suppression to be revisited. Both `.cargo/audit.toml` and `deny.toml` carry the crate, the path that pulls it in, and what clears it.

## [4.6.0] - 2026-08-07

### Removed

- **The `safetensors` and `ndarray` crates.** Both were optional dependencies enabled by default that no code ever called: `safetensors::` and `ndarray::` appear in no `.rs` file in the repository, and neither feature gated a single `#[cfg]`. SafeTensors support is real and unaffected — it is implemented in `src/formats.rs` and never used the upstream crate.

  Every default install compiled and linked them for nothing. Dropping them takes the default dependency graph from **476 to 464 crates**, along with their build time, binary size, and advisory surface. Two of the open Dependabot PRs existed only because of crates the project does not use.

  Found while assessing whether a `safetensors` 0.4 → 0.8 major bump was safe to merge. It was safe in the sense that nothing could break, which turned out to be the more interesting finding.

  The feature *names* are kept as no-ops, so `--features safetensors` and `--features ndarray` still resolve rather than failing the build for anyone who passes them. `default` and `full` are unchanged.

## [4.5.0] - 2026-08-07

### Removed

- **Containers.** The `Dockerfile`, `.dockerignore`, the `docker.yml` workflow that published to `ghcr.io`, and the Helm chart under `deploy/helm/` are gone, along with the Docker and Kubernetes documentation pages and the chart-linting CI job.

  Helm went with Docker rather than surviving it: the chart's only job was to deploy `ghcr.io/nervosys/ai-model-vault`, so without a Dockerfile it would have been a deployment path that could never obtain an image. Keeping it would have been worse than removing it — a chart that looks supported but cannot work.

  `aim` continues to ship as a static binary for five targets, a crate, and a Python wheel. For a hardened service install, `deploy/systemd/install.sh` remains and is unaffected. Images already published to `ghcr.io` stay pullable but receive no further updates; nothing re-tags or deletes them.

  Documentation that described containers as a supported path now says they were removed. `docs/MIGRATION.md` keeps its Docker and Kubernetes sections, annotated — it is a record of what v1.0.0 shipped, and rewriting history there would make it a worse migration guide.

### Fixed

- **`aimodelvault.__version__` reported the previous release.** The Python package carries its own version constant, separate from `Cargo.toml` and `pyproject.toml`, and the 4.4.0 bump missed it — so the published 4.4.0 wheel reports `4.3.0`. Cosmetic, but wrong. The existing `test_version_matches_crate` drift test caught it; it is now correct and all three constants move together.

## [4.4.0] - 2026-08-06

### Added

- **Blockchain audit trail is wired up** (`security.blockchain_audit`, default off). `blockchain.rs` has been a complete Merkle-chain implementation since it was written, and nothing ever called `add_entry`. It now hangs off `AuditLogger::log`, which is the single choke point every audit helper routes through — so a call site cannot record to the plain log while skipping the chain.

  `blockchain_block_size` defaults to **1** for a reason worth stating: pending entries live in memory until a block is finalized, so at any higher value a process that exits early silently drops the entries it was asked to make tamper-evident. At 1 every entry is written immediately. The logger also finalizes on drop, which narrows the window for larger block sizes but cannot close it — a crash still loses what is pending, and `aim chain status` reports that count.

  New `aim chain` commands: `status`, `verify`, `proof`, `verify-proof`, `search`. They open the chain directly rather than through `Vault`, so they need no passphrase and — more to the point — **reading the trail does not append to it**. Going through the vault logs a `VaultOpened` entry, which meant a `chain verify` cron job grew the chain by one block per run and `verify` reported a different height than the `status` printed seconds earlier. `verify` and `verify-proof` exit 5 on failure so they work as CI gates.

- **Federation is wired up** (`federation.enabled`, default off). `FederationManager` was already a complete HTTP client aimed at `/api/v1/federation/*`; nothing served those paths, so a node could only sync against a peer that did not exist. This release adds the server half, the `aim federation` commands (`status`, `manifest`, `plan`, `sync`), and transit encryption.

  Peers authenticate with a shared key in `X-API-Key`, not the JWT the rest of the API uses — a peer is a machine with a long-lived pre-shared secret, not a user with a session, and handing it a login token would grant the full model API just to fetch weights. One key per pair serves both directions, and disabling a peer revokes it both ways. Keys resolve through the existing KMS layer, so config can hold `env://NAME` rather than a secret. They resolve at startup, so a bad reference aborts the server instead of failing the first sync at 3am.

  Transfers are sealed by default with the same `AIMVSEAL` envelope used for cloud uploads, keyed by `$aimodelvault_FEDERATION_PASSPHRASE` (read from the environment, never the config file). TLS protects the hop; this protects the object, so a peer's reverse proxy, request log, or on-disk cache never holds a readable model. An unsealed model arriving while sealing is on is **refused rather than stored**, so a peer cannot downgrade a transfer by sending plaintext.

  The routes are not registered at all when federation is off — an unauthenticated caller gets a 404 rather than a 401 confirming the endpoint exists. When it is on, `aim serve` unlocks the vault at startup from `$aimodelvault_PASSPHRASE`, because peers hold the federation key and never the vault passphrase; without that every peer request failed until a human POSTed to `/auth/token`.

### Fixed

- **A tampered audit proof verified as valid.** `BlockchainAudit::verify_proof` walked the Merkle path from a `leaf_hash` carried *inside* the proof and never checked that hash came from the entry beside it. Rewriting `proof.entry` — changing a model version, flipping the `success` flag — left a proof that still passed. A tamper-evidence mechanism that accepts tampering is worse than none, since it converts "unverified" into "verified". The leaf is now recomputed from the entry. Found by editing a proof by hand and watching it pass; covered by a regression test that alters an entry and one that flips only the boolean.

- **`verify_proof` panicked on a crafted proof.** The block-chain walk used `0..block_chain.len() - 1`, which underflows on an empty chain. That function parses a JSON file supplied by whoever runs `aim chain verify-proof`, so malformed input crashed instead of reporting invalid.

- **Federation sync never converged.** `add_version` mints a checkpoint id from the model name, local version number, and current time, so a received copy got an id its sender had never seen. The next sync found the sender's version still "missing" and transferred it again — every run duplicating the model on both nodes, without bound. A received version now keeps the id it arrived with (recorded as `federation_origin_checkpoint_id` and advertised in place of the local one), which is what gives a version one identity across the federation. Verified against two live nodes: one transfer, then zeros on every subsequent sync, in both directions, with bytes identical to the original.

- `FederationSettings` derived `Default`, which made `seal_transfers` **false** while the serde default said true — models would have shipped in the clear depending on which path constructed the struct. Now hand-written, with a test pinning it.

## [4.3.0] - 2026-08-03

### Added

- **`aim cloud push` now encrypts client-side.** Until now it uploaded what `Vault::get_model` returns, which is plaintext — the vault decrypts on read — so the object in the bucket was the bare model and confidentiality rested entirely on the bucket's own access policy and server-side encryption. 4.2.1 documented that honestly; this release removes the problem.

  Payloads are sealed with AES-256-GCM under an Argon2id key derived from the vault passphrase, with a fresh salt **per object**. Pushing the same model twice produces different ciphertext, so an observer cannot tell that two objects hold the same model.

  The salt travels inside the object rather than living in the vault, and that choice is deliberate: uploading the vault's own on-disk blob would have produced an object only the originating vault directory could open, which defeats the case cloud storage exists for. As sealed, a colleague or a CI runner who knows the passphrase can `pull` into a *different* vault.

  The header is not separately authenticated because it does not need to be — every field in it feeds key derivation, so altering any of them yields a different key and the GCM tag check fails. Tampering produces an error, never wrong plaintext. There are tests for a flipped ciphertext bit, a flipped salt byte, a wrong passphrase, truncation at eight different offsets, and that the plaintext does not appear anywhere in the sealed bytes.

  **Objects pushed before 4.3.0 are still plaintext.** `pull` detects the missing magic, accepts them so nothing already in a bucket is stranded, and warns. Re-push to seal, then delete the old object; nothing re-encrypts in place.

- **The five dormant telemetry helpers are now wired.** Before 4.2.0 an opted-in install reported one event at startup and nothing else; 4.2.0 added `CommandRun`. The remaining five event types existed with public `track_*` helpers and no call sites. All are now emitted — still only when telemetry is explicitly enabled, which it is not by default.

  | Event | Where | What it carries |
  |---|---|---|
  | `ModelOperation` | store / get / delete | operation, format label, **size bucket**, duration, outcome |
  | `Conversion` | `aim convert` | source and target format labels, duration, outcome |
  | `ApiCall` | every HTTP request | matched **route template**, method, status, duration |
  | `Error` | any failed command | variant name only |
  | `FeatureUsed` | KMS URI resolution | the URI scheme only |

  Every label is a `&'static str` from a closed set, and three specific hazards were closed rather than assumed away:

  `ModelFormat::name()` returns the *caller's own string* for `Custom`, so a `--format` argument would have been reported verbatim. Added `telemetry_name()`, which collapses every `Custom` to the literal `"custom"`; a test feeds it paths, S3 URLs, and a bearer token and asserts none survive.

  `ApiCall::endpoint` had to be the route template, not the resolved path — `/api/v1/models/gpt-4-customer-tuned` names a model. It is taken from axum's `MatchedPath`, which is only available in middleware and is a literal from the router table. Unmatched requests report the constant `<no match>` rather than the requested path, which is attacker-controlled.

  `Error` reports `VaultError::kind()`, a new fixed literal per variant, and never `Display` — every message-carrying variant interpolates a path or model name. `context` is passed as `None`.

  Exact model sizes are never sent, only the existing four buckets. No new OTLP attribute keys: all twenty were already on the pinned approved list.

### Fixed

- **`.well-known/agents.json` told agents to run `pip install ai-model-vault`.** The PyPI distribution is `aimodelvault`; that command does not resolve. A test now derives the expected string from `pyproject.toml`.

- **`agents.json` advertised `AZURE_STORAGE_KEY`**, which `AzureBackend::new` rejects outright, and `GOOGLE_APPLICATION_CREDENTIALS` / `GCP_PROJECT` for a GCS backend that no longer exists. Replaced with the SAS and Entra ID variables actually consulted, plus the OTLP and `DO_NOT_TRACK` variables that were missing. A test fails if the shared-key variable reappears.

## [4.2.1] - 2026-08-03

### Added

- **`deploy/systemd/install.sh`** — provisions the unit and writes the OTLP environment variables at install time, rather than leaving a hand-editing step in the documentation. Creates the `aim` system user and `/var/lib/aim`, writes `/etc/aim/server.env` at 0600 root-owned through a temp file and rename (a plain redirect would inherit the umask and be briefly world-readable), generates `AIM_JWT_SECRET` if absent while preserving an existing one, and reloads systemd. Idempotent, with `--dry-run`.

  The OTLP credential is read from a file (`--otlp-headers-file`), never an argument: command-line arguments are world-readable through `/proc/<pid>/cmdline` for the life of the process, so passing a bearer token as a flag exposes it to every local user — the exact thing `EnvironmentFile=` is there to prevent. The token is never echoed, including under `--dry-run`, which reports only the path it came from.

  `--enable-telemetry` is separate from the exporter flags, so configuring a collector still does not turn collection on.

### Fixed

- **The default telemetry collector was undocumented, and the docs denied it existed.** `TelemetryConfig::endpoint` defaults to `https://telemetry.nervosys.ai/v1/events`, compiled into the binary — but `docs/TELEMETRY.md` stated flatly that "there is no default collector and no default token", and the 4.1.0 changelog said "nothing is baked into the binary". Both statements were written about the OTLP exporter, which genuinely has no defaults, and neither said so.

  The practical effect: someone who opted in had no way to learn from the documentation where their events were being sent. That is the one thing a telemetry document exists to tell you. The endpoint is now disclosed, with instructions for pointing it at a collector you control, and both claims are scoped to the OTLP path.

  No data was ever sent without consent — `enabled` defaults to `false` and always has. This is a disclosure failure, not a collection one.

## [4.2.0] - 2026-08-03

An audit of the shipped surface against the shipped documentation. Most of what follows is a correction rather than a feature — the code was in better shape than the pages describing it.

### Added

- **`CommandRun` telemetry is now emitted** (still only when telemetry is enabled, which it is not by default). Six of the seven `track_*` helpers had no call sites, so an opted-in session reported exactly one event at startup. This wires `track_command`: subcommand name, duration, and a success boolean.

  The binary takes both names from clap's registered command table via `ArgMatches`, not from the parsed value or the raw command line. That is the property that makes it safe rather than a promise about it: `subcommand_name` can only return a literal declared in `args.rs`, so the field's value set is the set of subcommands, and a model name or path has no route in. A test asserts argument values do not appear. The failure reason is not recorded, only the boolean — error messages interpolate paths.

### Fixed

- **`RetrievalOptimizer` was not reliably LRU.** `evict_lru` selected `min_by_key(last_access)` over a `HashMap`, and `last_access` is a `SystemTime`. Entries touched inside a single clock tick carry *equal* timestamps; `min_by_key` returns the first minimum in iteration order, and `HashMap` randomises that order per instance. So which entry got evicted was arbitrary whenever timestamps tied — which is the common case on a fast machine, since three inserts complete in well under a tick. `SystemTime` is also not monotonic and can move backwards under NTP.

  Both caches now carry a strictly increasing sequence stamp and evict on that. `src/rag/cache.rs` had the same latent defect behind its `access_count` comparison and gets the same tiebreak.

  This surfaced as `cache_eviction` failing one macOS release build after passing all 27 CI jobs on the same commit. The regression test forces the tie rather than waiting for a coarse clock to produce one — the previous formulation passed on Windows even with the bug present, because the clock there advances between the calls.

- **`docs/CLOUD_STORAGE.md` claimed cloud uploads were encrypted client-side.** In bold: "only encrypted data leaves your machine", "cloud providers never see your plaintext models". Neither is true. `aim cloud push` calls `Vault::get_model`, which decrypts and decompresses, and uploads that buffer — the object in the bucket is the plaintext model. Anyone who sized their bucket controls against that promise was less protected than they believed.

  The push handler now warns at the point of upload, and both cloud documents lead with the real threat model. The wire format is unchanged: sending ciphertext instead changes what `pull` must do and how vaults with differing passphrases interoperate, which is a design decision rather than a documentation fix.

- **`docs/BLOCKCHAIN_AUDIT.md` documented a CLI that does not exist.** `aim audit`, `aim audit --verify`, and `aim audit --export` are not commands — there is no `audit` subcommand. The page also stated that every mutating operation was recorded as a block; `blockchain.rs` has no callers anywhere in the tree, so nothing was ever recorded. It is a library primitive and now says so, pointing at `audit.rs`, which *is* wired into `Vault` and is what actually produces an audit trail. The block-fields table listed `principal`, `operation`, and `payload`, none of which are fields of `AuditBlock` or `BlockEntry`.

- **`docs/FEDERATION.md` claimed REST API exposure.** `src/api/` contains no reference to the module. Federation is library-only, as the README's capability table already said.

- **`docs/CLOUD_STORAGE.md` was written against commands that do not exist** — `aim remote add`, `aim push`, `aim sync --direction`, and a "remote" concept the CLI has never had. Rewritten against the real `aim cloud push|pull|list|config`. Stray references to `aim import` and `aim info` corrected in `XDG_COMPLIANCE.md` and `MIGRATION.md`.

- **`aim cloud config --provider azure --show` advertised `AZURE_STORAGE_KEY`**, which `AzureBackend::new` rejects outright — the Azure SDK for Rust v1 has no shared-key credential, a constraint recorded in `Cargo.toml` since 4.0.0 but never reflected in the CLI. It now lists the SAS and Entra ID variables actually consulted, and flags the dead one when set.

- Website dependencies: seven advisories cleared, including Next.js request smuggling in rewrites and a Server Actions CSRF bypass via null origin (16.1.6 → 16.2.12). The remaining postcss and sharp findings are vendored inside Next.js and have no stable fix — `npm audit fix --force` "resolves" them by downgrading to next@9.3.3.

## [4.1.1] - 2026-07-31

Both fixes are test-only or inert for consumers; 4.1.0 is not broken for anyone depending on the crate. This release exists so the git tag, the crates.io artifact, and the source all agree.

### Fixed

- **The revocation tests raced against each other.** `test_revoke_claims` revoked a token and asserted it no longer verified, but `configure_revocation_store` replaces `entries` wholesale, so a concurrent store test deleted the revocation in between. The existing `STORE_LOCK` guarded only the `store` field, on the reasoning that the file was the shared resource — the entries map was shared too. Every test touching the global list now takes one lock, and resets state on *acquire* rather than on release, so a panicking test cannot cascade into an unrelated failure.

  It surfaced on macOS and passed everywhere else, which reads like a platform bug and is not one: locally it reproduced 1 run in 30 without the lock and 0 in 30 with it. Thread scheduling, not the platform.

- **`src/aimodelvault/__init__.py` still declared `3.0.0`.** The 4.0.0 release bumped `Cargo.toml` and `pyproject.toml` and missed the Python constant. A test already compared `__init__.py` against `Cargo.toml`, but CI had not run at 4.0.0, so nothing caught it until now. Added a matching check for `pyproject.toml`, which was the one version source with nothing asserting on it.

## [4.1.0] - 2026-07-31

### Added

- **OTLP export for telemetry events**, behind a new `otel` feature. Configured entirely from the standard OpenTelemetry environment variables — `OTEL_EXPORTER_OTLP_ENDPOINT` (and the signal-specific `..._LOGS_ENDPOINT`, which takes precedence), `OTEL_EXPORTER_OTLP_PROTOCOL` (`http/protobuf` or `http/json`), `OTEL_EXPORTER_OTLP_HEADERS`, and `OTEL_SERVICE_NAME` — so any collector or vendor endpoint works without bespoke configuration. Events map onto OTLP log records. Feature-gated because it pulls in prost and the OpenTelemetry SDK; default and `full` builds are unchanged.

  Two properties are enforced by the code rather than described in prose. **Configuring an exporter does not enable collection**: pointing a build at a collector and consenting to report are separate decisions, usually made by different people, so both are required — there is a test for it. And **no OTLP endpoint or token is baked into the binary**: the exporter has no default collector and no default credential. A credential compiled into an AGPL crate published to a public registry is readable by everyone who installs it. (As originally written this said "nothing is baked into the binary", unqualified. That was wrong: the *built-in* sender has always had a compiled-in default endpoint. Corrected in 4.2.1, where it is also documented.)

  The exported attribute key set is pinned by a test. `Error::context`, `ApiCall::endpoint` and `FeatureUsed::detail` are the only fields that could carry a file path or a model name, so adding a key is now a deliberate edit to an approved list rather than something that can happen by accident. Building without the feature while `OTEL_EXPORTER_OTLP_ENDPOINT` is set warns on stderr instead of silently discarding the configuration.

- **Service-scoped deployment configuration.** New `deploy/systemd/` unit and example environment file; the unit uses `EnvironmentFile=` rather than `Environment=`, because `Environment=` values are readable by any local user through `systemctl show` and `systemd-analyze dump` — for a bearer token that means every account on the host. The Helm chart gained a `telemetry` block that defaults to disabled and sources the `Authorization` header from a Secret created out of band, never from `values.yaml`, which is committed and printed back by `helm get values`. Neither path writes anything machine-global.

- Helm can now set `AIM_REVOCATION_STORE`, which shipped in 4.0.0 with no way to configure it from the chart.

### Fixed

- **`docs/TELEMETRY.md` described collection that does not happen.** It stated command names were sent and documented a `GET /api/v1/telemetry/status` endpoint. Neither exists — `track_app_start` is the only tracker with a caller, and there is no such route. Rewritten to match the code.

- **`src/telemetry.rs` documented `enabled` as defaulting to `true`** while `Default` set it to `false`. The code was right; the comment was the kind of wrong that gets quoted in a privacy review. The module's "Data Collected" list also named commands, errors and feature use, none of which are collected.

- **`.env` was not gitignored.** The `env/` entry above it matches the virtualenv directory, not the file, so a dotenv file — the most common way a credential reaches a public repository — was committable. Added it along with `credentials.toml`, keystores and SSH keys, keeping `.env.example` allowed.

### Changed

- CI feature matrix builds `otel`.

## [4.0.0] - 2026-07-31

Hardening items that the 3.0.0 audit identified and reported but did not fix. Each was a real weakness left standing; this closes all four.

### Security

- **Revoking a JWT did nothing after a restart, and nothing ever revoked one.** The revocation list was a process-local `HashSet` with no persistence, so restarting the server re-admitted every revoked token that had not yet expired — a leaked token was "revoked" only until the next deploy. It was also unreachable: `revoke_token` had no caller outside its own test, so there was no way to invalidate a single token short of rotating `jwt_secret`, which invalidates every other token at the same time.

  `POST /api/v1/auth/logout` now revokes the presenting token. Revocations persist to the file named by `--revocation-store` / `AIM_REVOCATION_STORE` / `ApiConfig::revocation_store`, written through a temporary file and renamed so a crash mid-write leaves the previous list intact rather than a truncated one — a truncated list is the dangerous failure, because it un-revokes. A store that exists but cannot be parsed aborts startup instead of silently starting with zero revocations. Starting without a store logs a warning naming the consequence.

  Entries now carry the token's `exp` and are pruned once past it, so the list no longer grows for the life of the process. This remains a single-node store: replicas do not share it. That limitation is documented on `configure_revocation_store` rather than left to be discovered.

- **A poisoned revocation lock was read as "not revoked".** `if let Ok(revoked) = REVOKED_TOKENS.read()` fell through on a poisoned lock, so a panic in any thread holding it would have admitted every revoked token for the remaining life of the process. The lock is now recovered with `PoisonError::into_inner`.

- **Archive extraction allocated whatever the archive claimed (uncontrolled resource consumption, CWE-409).** `extract_tar` and `extract_zip` called `read_to_end` on each member with no ceiling, so a compressed archive of a few hundred KiB expanded to as much memory as it declared — an out-of-memory kill for the process and anything sharing it. Members are now capped at 8 GiB each and 16 GiB per archive, and the read is bounded independently of the declared size, so a header that understates its payload is caught too.

- **Passphrases were left in freed memory (ATT&CK T1552).** `prompt_passphrase` copied the secret out of three intermediate buffers — the `String` from `kms::resolve`, the `String` filled by `read_line`, and the `$aimodelvault_PASSPHRASE` value itself — and dropped all three without clearing them, leaving the plaintext in the allocator to resurface in a later allocation or a core dump. Every intermediate is now zeroized on all paths, including the error paths.

### Fixed

- **`helm upgrade` silently rotated the JWT signing key.** `randAlphaNum 64` is evaluated on every render, so any upgrade that did not set `api.jwtSecret` minted a new key and invalidated every token the running deployment had issued: blanket 401s until clients re-authenticated, and any in-flight agent run died mid-task. Nothing in the chart signalled that upgrading was a credential rotation. The template now reads the live Secret with `lookup` and carries the generated key forward.

### Changed

- **`ApiConfig` is now `#[non_exhaustive]`.** Adding `revocation_store` broke every downstream struct literal. Marking it non-exhaustive makes this the last time a field addition is a breaking change; construct with `ApiConfig::default()` and assign the fields you need.

- `auth::revoke_token` is deprecated in favour of `auth::revoke_claims`, which records the token's expiry so the entry can be pruned. The old function stores an entry that can never be retired.

## [3.0.0] - 2026-07-30

Findings from a security and privacy audit against CVE, MITRE ATT&CK, NIST FIPS and CMMC 2.0.

### Security

- **`aim extract` wrote files outside the `--output` directory (zip slip, CWE-22, ATT&CK T1574).** `ModelArchive::extract_zip` returned each member's name verbatim from the archive, and `handle_extract` passed it to `Path::join` — which discards its base when handed an absolute path and walks upward on `..`. Demonstrated against the built binary: a ZIP containing `../../ESCAPED.txt`, extracted to `out/deep/nested`, wrote the file two directories above the target, printed `✓ Extracted`, and exited 0. A longer prefix or a drive letter reaches anywhere the invoking user can write — a shell rc file, a startup folder, `authorized_keys`. Triggered simply by extracting an untrusted model archive.

  Member names must now be a single ordinary file name, which is all `create_tar` and `create_zip` have ever produced. Validation happens while the archive is read, before the caller writes anything, so a hostile archive aborts the whole extraction rather than leaving a partial result — verified: the same exploit now writes nothing at all and exits `6`.

  The TAR path was not exploitable — it reduced names with `file_name()` — but it did so *silently*, so `../../etc/passwd` became `passwd` and could quietly overwrite a legitimate member of that name. It now rejects rather than renames.

- **The API accepted any non-empty JWT signing secret.** Only emptiness was checked, so `--jwt-secret hunter2` started a server whose HS256 tokens could be forged after recovering the key offline from a single issued token. RFC 7518 §3.2 requires an HMAC key at least as large as the hash output — 256 bits for HS256. Secrets shorter than 32 bytes are now refused at startup with an actionable message.

  The rest of the JWT path audited clean: `Validation::default()` pins the algorithm allowlist to HS256 (so `alg: none` and RS256 key-confusion are not possible) and requires `exp`, and the secret is zeroized on drop.

- **`aim compliance` spawned `cargo` from an untrusted working directory (CWE-426).** The CVE check ran `Command::new("cargo")`, resolved from `PATH` — and on Windows `CreateProcess` searches the current directory first, so a `cargo.exe` dropped in whatever directory the user happened to be in would execute. It also audited that directory's manifest, which says nothing about the vault's own dependencies. The check now runs only when a `Cargo.toml` is actually present, and reports `NOT VERIFIED` otherwise, stating plainly that it inspects the current project rather than the installed binary.

- **The signing private key was written with inherited ACLs on Windows (CWE-276, ATT&CK T1552.001).** `save_keypair` tightened permissions under `#[cfg(unix)]` only, so on Windows the secret seed — the one thing that lets an attacker forge signatures for an identity — inherited whatever the parent directory granted. Observed on a real run: `BUILTIN\Administrators` held `FullControl` with inheritance enabled, while the vault's *own* config had inheritance stripped and was owner-only. The key was less protected than the config describing it. Every other module in the crate already used the cross-platform `permissions::restrict_file`; `signing.rs` was the lone outlier.

  It also used `fs::write` followed by a chmod, leaving the seed briefly world-readable on Unix. The file is now created with restrictive permissions *before* the seed is written, then tightened again for Windows. Verified end-to-end: the key file is now owner-only with inheritance disabled.

- **`SigningKeyPair`'s derived `Debug` printed the secret seed verbatim.** Nothing logged it today, but any `{:?}` — a `tracing` call, an `unwrap` panic, an error report — would have put signing key material into a log. `Debug` is now hand-written and redacts the seed while keeping the public fields useful. `Serialize` is deliberately unchanged: writing the seed is what `save_keypair` is for.

- **`aim compliance` could not fail, and said so in the language of certification.** Three of its four checks were hardcoded (`check_fips_140_3` → `true`, `check_mitre_attack` → `true`, `check_cmmc` → `2`) and every one was printed as `✓ PASS`. An organisation putting the command in a CI gate for CMMC evidence would collect a green result regardless of the state of the system.

  Worse, the CVE check — the only one that does real work — returned **pass** when `cargo audit` could not run. That is the normal case for an installed binary, where there is no `Cargo.toml` in the working directory to audit. A scan that never happened reported as clean.

  Checks now report a `CheckOutcome`: `Verified` (tested this run), `AssertedByDesign` (a property of how the software is built, not evidence of certification), `NotVerified` (the check could not run — explicitly *not* a pass), or `Failed`. Only a verified failure is blocking, and it exits `8`.

### Fixed

- **FIPS 140-3 and CMMC 2.0 were claimed as achieved status across the documentation.** `AGENTS.md` stated CMMC 2.0 Level 2 **"Certified"**; `README.md`, `docs/index.md`, `docs/EXECUTIVE_SUMMARY.md` and others stated "FIPS 140-3 compliant"; the README carried a `security-FIPS 140-3` badge. None of this is true, and none of it can be:

  - FIPS 140-3 validates a *cryptographic module* through NIST's CMVP, which issues a certificate number. The RustCrypto implementations (`aes-gcm`, `sha2`, `argon2`) hold no CMVP certificate.
  - **Argon2id is not a FIPS-approved KDF.** SP 800-132 approves PBKDF2. Argon2 is the better choice against modern cracking hardware, which is why it is used — but it puts the KDF outside FIPS regardless of the module question. "FIPS 140-3 compliant … Argon2id" was self-contradictory.
  - CMMC certification is granted to an *organisation* by a C3PAO. No software product can be CMMC certified, and a contractor relying on that claim in an assessment would fail it.

  Claims are now stated accurately: FIPS-*approved algorithms*, not a validated module; CMMC controls *supported*, not certified; MITRE ATT&CK mitigations *by design*, not a penetration test. The `.well-known/ontology.jsonld` crypto description and the `Cargo.toml` / `pyproject.toml` comments were corrected too, since agents read the first and developers the others. Historical `CHANGELOG` entries and `docs/archived/` were deliberately left alone — rewriting them would falsify the record.

- **Two structs named "telemetry enabled" had opposite defaults.** `config::TelemetrySettings::enabled` defaulted to `true` with a comment saying so, while `telemetry::TelemetryConfig::enabled` defaulted to `false`. The effective behaviour is the one the README promises — a default install transmits nothing, confirmed empirically by running `aim init` in an isolated home and reading the generated `telemetry.yaml` — but the arrangement is one well-meaning "fix" away from silently beaconing a persistent device UUID to `telemetry.nervosys.ai`. The outer field is now documented as a permission gate that defers to the inner switch, and a test pins the opt-in default.

### Breaking Changes

- **Archive members with directory separators are rejected.** `aim extract`, `ModelArchive::extract_tar` and `ModelArchive::extract_zip` now fail on any member name that is not a single file name. Archives produced by `aim archive` are unaffected. Third-party archives with nested directories, previously flattened (TAR) or written out (ZIP), now error.
- **`aim serve` refuses a JWT secret shorter than 32 bytes.** Existing deployments using a short secret will fail to start until it is replaced — which is the point.

### Audit notes (no change required)

- **No secrets in the repository or its history.** The only credential-shaped string is `AKIAIOSFODNN7EXAMPLE`, AWS's published documentation placeholder. The Helm `secret.yaml` template generates a random JWT secret rather than shipping one.
- **`cargo audit` / `cargo deny`: no vulnerabilities.** Three unmaintained-crate advisories (`fxhash` RUSTSEC-2025-0057, `instant` RUSTSEC-2024-0384, `rustls-pemfile` RUSTSEC-2025-0134), all transitive and none with a known exploit path.
- **Crypto primitives are sound.** AES-256-GCM with 96-bit nonces from `OsRng`, Argon2id at 64 MiB / t=3 / p=1 (above the OWASP minimum), keys held in a `ZeroizeOnDrop` container.
- **File permissions hold on both platforms.** Verified on Windows that vault directories and config get inheritance stripped and are restricted to the owner.
- **The telemetry event schema is privacy-conscious** — size *buckets* rather than sizes, no model names, no paths — and the tracking functions that take free-text `context`/`detail` are currently dead code.

### Fixed

- **`aim diff` panicked on a crafted SafeTensors header (found by fuzzing).** `parse_safetensors_header` guarded with `data.len() < 8 + header_size || header_size > 100_000_000`. `||` evaluates left to right, so a file declaring a header size near `usize::MAX` overflowed the addition — `attempt to add with overflow` — before the cap could reject it. Any untrusted `.safetensors` file could abort the process. The cap is now checked first and the comparison uses subtraction against a length already known to be ≥ 8.

  This was found by the `diff_engine` fuzz target on its first ever execution — see below.

- **No fuzz target had ever run in CI.** Three of the eight steps invoked `crypto_roundtrip`, `format_detection` and `model_metadata`, but the targets are named `fuzz_crypto_roundtrip` and so on. The job aborted on the first step, so none of the eight targets the README advertises had executed. With the names corrected, six passed and the seventh immediately found the overflow above.

- **Benchmarks never ran in CI.** `cargo bench -- --output-format bencher` passes the argument to every bench target, including the built-in libtest harness for the lib and the binary, which rejects it with `Unrecognized option: 'output-format'` before any benchmark starts. The criterion benches already set `harness = false`; the lib and bin now set `bench = false`. The results-storage step also read and wrote history on a `gh-pages` branch that does not exist, failing every run with `couldn't find remote ref gh-pages` — so the benchmarks ran and their results were discarded. History now lives in a cached JSON file via `external-data-json-path`, the action's documented alternative; creating a published branch is a repository decision rather than a build fix. (`auto-push: false` alone is not enough — it stops the push, not the fetch.)

- **`examples/xdg_demo.rs` never compiled on Unix.** `use std::os::unix::fs::PermissionsExt` imports the trait, not the `std::fs` module, so two `fs::metadata` calls in `#[cfg(unix)]` blocks were unresolved (E0433). It broke the whole Test Suite matrix, every Feature Combinations job, and the Security Audit workflow on Linux and macOS, while compiling cleanly on Windows where the blocks are stripped.

- **Beta clippy gated CI, so new lints turned it red on a schedule.** The test matrix ran `clippy -D warnings` on both stable and beta; beta ships new lints every six weeks, and `chunks_exact_to_as_chunks` and `unused_async` duly failed the build for reasons unrelated to any change under test. Clippy now gates on stable only — the beta run still happens and its findings still appear in the log, which is the early warning worth having, it just does not block. Build and tests remain gated on beta.

  The four lints it found were fixed rather than suppressed wholesale: the two `chunks_exact(4)` sites now use `as_chunks::<4>()`, which yields `[u8; 4]` directly and removes manual indexing (verified against MSRV 1.89 locally, not assumed). The two `unused_async` sites carry a narrow `#[allow]` with the reason — `StorageConfig::create_backend` genuinely awaits under the `s3` and `azure` features, so the lint only fires in a build where those arms are compiled out, and dropping `async` would break every feature-enabled build and every caller.

- **Two tests asserted platform- or environment-specific behaviour as universal.** `test_check_cve_enabled` asserted a CVE scan always "passes", documenting in its own comment that an unavailable `cargo-audit` counted as a non-failure — the exact bug removed in this release; it went green locally because cargo-audit is installed and would have gone green on CI because it is not. `test_safe_archive_name_rejects_escapes` required `a\b` to be rejected, which is right on Windows and wrong on Unix, where a backslash is an ordinary filename character and such a member is one legal file rather than a path. Both now assert the invariant that actually holds, with the Windows/Unix split made explicit.

## [3.0.0] - 2026-07-29

A follow-on to 2.0.0 in the same vein, and for the same reason: the audit that produced 2.0.0 kept turning up the same defect — code that reported a confident result where it should have reported that it could not do what was asked. 2.0.0 fixed that in the signing, scanning, bundle and metadata paths. This release fixes it in the **process exit code**, which is the one channel every non-interactive caller actually reads.

Twelve commands exited `0` for work that did not happen, including three integrity and regression gates. If you script `aim`, the exit codes you were branching on were not the ones documented, and several failures were indistinguishable from success. Re-run anything you gated on `aim validate` or `aim eval compare`.

### Security

- **Commands that could not do what was asked reported success.** `main` returned `Result`, so every failure collapsed to exit code 1 — and separately, a dozen handlers printed a "not found" message to stdout and then returned `Ok(())`, exiting **0**. The affected commands are worse than merely imprecise:

  - **`aim validate` printed "Some checks failed." and exited 0.** It is an integrity gate, so every pipeline running it treated a failing model as valid. It now exits `5`.
  - **`aim vaults unregister`, `aim quantize remove` and `aim backup remove` exited 0 having removed nothing** when the named resource did not exist. A script checking the exit code concluded the deletion had happened.
  - **`aim versions`, `aim lineage`, `aim validate`, `aim analyze`, `aim export`, `aim profile show` and `aim database get` exited 0** for names that do not exist.

  - **`aim eval compare` exited 0 printing "No matching evaluation runs found for comparison."** It is a regression gate; a CI job reading that exit code concluded nothing had regressed when in fact nothing had been compared.
  - **`aim database build-index` exited 0 having built no index** when no document had an embedding. The next search against the index it claimed to build was the thing that failed.

  All of these now return an error and exit with the documented code. `VaultError` gains a `NotFound(String)` variant for named resources that are not models or versions, because reporting a missing profile as `ModelNotFound` would have been its own small lie.

- **A mistyped subcommand exited 2, which the published table defines as "authentication failed."** clap's default usage-error code is 2, so `aim versoins my-model` was indistinguishable from a wrong passphrase to any agent branching on the exit code — and would plausibly trigger a credential-refresh path. The CLI now parses with `try_parse` and maps usage errors to `6` (invalid input), while keeping `--help` and `--version` at 0.

### Fixed

- **Four mutually contradictory exit-code contracts were published, and none of them was implemented.** `README.md` and `AGENTS.md` said `2` = not found, `3` = integrity, `4` = permission denied. `docs/CLI.md` and `.well-known/agents.json` said `2` = authentication failed, `3` = not found, `5` = integrity. `.well-known/ontology.jsonld` used a fourth scheme keyed by error type, `1`–`8`, with `1` = crypto and no catch-all. `examples/agent_bootstrap.rs` asserted a fifth reading in a comment. The binary implemented none of them: it emitted only `0` and `1`, and printed the `Debug` form of the error rather than its message.

  This mattered more than an ordinary docs bug because the project advertises `.well-known/` and `aim introspect` as authoritative, machine-readable interfaces and instructs agents to branch on them.

  There is now one contract — `0` ok, `1` general, `2` auth, `3` not found, `4` permission, `5` integrity, `6` invalid input, `7` config, `8` compliance — implemented by `VaultError::exit_code`, and all five documents were rewritten to match it. Codes `0`–`5` keep the meanings the two machine-readable manifests already agreed on; `6`–`8` are new codes for categories that previously fell through to `1`, so no published meaning changed. The mapping is pinned by unit tests, by end-to-end tests that run the real binary, and by a test that parses both `.well-known/` manifests and fails if they drift from the implementation again.

- **Three `std::process::exit(1)` calls skipped the telemetry flush** and bypassed the exit-code mapping. They were invalid-input cases in `aim archive`, `aim extract` and `aim introspect`, and now return `VaultError::InvalidInput` like every other argument error.

- **`--config` failures were reported as generic I/O or serialization errors.** A missing or malformed config file now returns `ConfigError` with the path attached, and exits `7`.

### Changed

- **`VaultError` is now `#[non_exhaustive]`.** Adding a category is no longer a breaking change for downstream matches. Inside the crate the enum stays exhaustive, which is what forces `exit_code` to assign every future variant a code instead of letting it fall through a wildcard — the omission that let the published tables drift from reality in the first place.

### Breaking Changes

- **`VaultError` gained a `NotFound(String)` variant and is `#[non_exhaustive]`.** Downstream `match` expressions over it need a wildcard arm.
- **Commands that previously exited 0 on a missing resource or a failed validation now exit non-zero.** Any script that relied on `aim validate`, `aim versions`, `aim lineage`, `aim analyze`, `aim export`, `aim vaults unregister`, `aim quantize remove`, `aim backup remove`, `aim profile show`, `aim database get`, `aim database build-index` or `aim eval compare` succeeding for absent or invalid input will now see a failure. That is the point: the previous behaviour reported success for work that did not happen.

  Deliberately left at `0`: `aim deduplicate` finding no duplicates and `aim benchmark` listing no records for a model are genuine empty results, not failures.
- **Exit codes `6`, `7` and `8` are now returned** where `1` was returned before. Callers testing `== 1` for those categories need updating; callers testing `!= 0` are unaffected.
- **A usage error now exits `6` rather than clap's `2`**, including a mistyped subcommand, an unknown flag, a missing required argument, and `aim` with no subcommand at all. `--help` and `--version` remain `0`.

### Version

- **2.0.0 → 3.0.0** (Python package and Helm chart synced to 3.0.0). A major bump because the exit-code changes and the `VaultError` additions are both breaking. Note that 2.0.0 was tagged but never reached crates.io or PyPI — all release jobs were blocked on GitHub billing — so 3.0.0 is still the first release either registry would see.

## [2.0.0] - 2026-07-29

A security release. Five defects are fixed here, and they share one shape: code that emitted a confident, plausible answer where it should have said it could not tell. `aim verify` called forged models valid, the license scanner reported non-commercial models as MIT, `aim diff` saw a full-precision model and its 4-bit quantization as identical, the pickle scanner called a malicious checkpoint clean, and `aim vault-import` trusted a path out of an untrusted manifest. Anyone relying on those commands as a gate should treat prior results as unverified and re-run them.

The major bump is required by two changes: `ModelSigner::verify` no longer reports `valid: true` without a key, and the `gpu` and `hdf5-support` feature flags are gone. See **Breaking Changes** at the end of this section for the full list.

With `gpu` and `hdf5-support` gone, **every feature flag the crate declares is now built by CI** — the `full,graphql` Test Suite job plus the `feature-matrix` job cover `default`, `s3`, `azure`, `cloud`, `api`, `database`, `python` and, transitively, `sqlite`, `kv-store` and `vector-db`. Both removed flags were the two that no job compiled, and both turned out to be broken or inert.

### Security

- **`aim verify` reported forged models as valid, and exited 0 when it did not.** Two independent defects compounded. First, `ModelSigner::verify` returned `valid: true` when no key was supplied, having compared the file's SHA-256 against `file_sha256` — a field stored in the `.sig` file itself. An attacker who can replace the model can also replace the `.sig`, recompute that hash, and keep the signer identity; the check passes with no key material involved, and the CLI printed "✓ Cryptographic signature valid / Verification PASSED". Verification without a key is now explicitly refused: `valid` and the new `signature_checked` are both `false`, with a `reason` explaining why, and the CLI prints `? Cryptographic signature NOT CHECKED`. Second, `handle_verify` returned `Ok(())` after printing "✗ Verification FAILED", so `aim verify` exited **0** on failure — every non-interactive caller gating a pipeline on that exit code treated a tampered model as good. It now returns `VaultError::IntegrityError`.

  **Breaking:** callers that relied on `verify(..., None)` returning `valid: true` must pass the secret seed. That code was not verifying anything.

- **Signatures labelled HMAC-SHA256 were `SHA-256(seed || file_hash)`.** The bare-prefix construction is not HMAC and is vulnerable to length extension — it is not exploitable here, because the message is a fixed-length hash, but the crate documented and named it as HMAC while implementing something else, and any future change to a variable-length message would have made it exploitable silently. `sign` now computes RFC 2104 HMAC-SHA256 (tested against the RFC 4231 vectors, including the oversized-key case) and writes `version: 2`. Version-0/1 signatures still verify via the legacy construction, so existing `.sig` files keep working; re-sign to upgrade.

- **Signature tags were compared with `==` on `String`.** Byte-string equality short-circuits on the first differing byte, leaking a timing oracle that lets a tag be recovered one byte at a time. Comparison is now length-checked and XOR-accumulated over the full input.

- **`aim vault-import` could write outside the vault (path traversal, CWE-22).** `ModelVersion::file_path` is read from `versions.json` *inside* the bundle, which is attacker-controlled for any archive the user did not produce. It was passed straight to `Path::join` for both the read and the write — and `join` discards its base when handed an absolute path and walks upward on `..`. A bundle declaring `file_path: "../versions.json"` made the importer copy over the target vault's own version index; deeper prefixes reach outside the vault directory entirely. tar-rs's own extraction guard does not help here, because the hostile value travels in the manifest rather than in a tar entry name. Blob paths are now required to be a single ordinary file name — which is all `export_vault` has ever produced — and the whole bundle is rejected before any file is touched, so a bad entry cannot leave a half-merged vault.

- **The pickle scanner reported malicious models as clean when the payload was compressed.** `PickleScanner` searched the raw file bytes for dangerous opcodes and for strings such as `os\nsystem`. A PyTorch checkpoint is a ZIP archive, and while `torch.save` writes *stored* (uncompressed) members — so the payload happens to be visible — `torch.load` accepts a DEFLATE-compressed archive just as readily. In that case nothing in the file literally contains the opcodes or the marker strings, and the scanner returned `safe: true` with "No dangerous patterns detected" for a file that executes `os.system` on load. The scanner now decompresses ZIP members and scans what the loader will actually unpickle, with a bounded inflate budget and member cap so a zip bomb cannot turn a scan into an OOM. Findings from multiple members are merged per code, and each records which member it came from.

- **Vault bundles carried an integrity checksum that was never checked.** `data_checksum` was computed on export and never read on import, so a truncated or corrupted bundle imported silently. The digest was also unverifiable in principle: it folded blobs in the exporter's enumeration order, which an importer holding a `HashMap` cannot reconstruct. The digest is now taken over blobs in sorted-name order with length framing, the bundle format version is bumped to 2, and import verifies it before writing anything. Version-1 bundles still import but report `checksum_verified: false` rather than implying a check occurred — and the CLI prints `Integrity: NOT VERIFIED` for them. Note this is a corruption check, not an authenticity one: the digest ships inside the archive it describes, so `aim sign` / `aim verify` remain the provenance mechanism.

### Added

- **`src/gguf.rs`** — one bounds-checked GGUF header reader, shared by `aim diff` and license scanning. Both previously carried their own ad-hoc parser, and both were wrong in the same way: they inferred content from the shape of the bytes near a marker instead of walking the metadata block. `gguf::tensors` returns real tensor descriptors; `gguf::metadata_string` returns the value stored under exactly the requested key, or nothing.

### Removed

- **The `gpu` feature and `src/crypto/gpu.rs` are gone.** The feature had never compiled in any release: launching the OpenCL kernel requires an `unsafe` block, and the crate sets `unsafe_code = "forbid"` at the manifest level, so `cargo build --features gpu` failed with `usage of an unsafe block` before it emitted a single object file. CI never caught it because the feature matrix did not include `gpu`.

  The consequence was worse than a broken build flag: the module carried a hand-written AES-256 implementation as an OpenCL kernel that, never having compiled, had also never been executed or checked against NIST known-answer vectors. Shipping an unvalidated reimplementation of a cipher is a worse trade than not offering GPU offload, so the module, the feature flag, the `ocl` dependency, and the GPU documentation were removed rather than repaired.

  This closes three open findings in `reports/SECURITY_AUDIT_REPORT.md`: **C-01** (AES-CTR without authentication), **C-02** (AES key left resident in GPU memory after the kernel ran), and **C-03** (unsafe OpenCL FFI). It is not a breaking change for any consumer — no version of the crate could be built with the feature enabled.

- **The `hdf5-support` feature and the `hdf5` dependency are gone.** The flag gated nothing: no code in the crate ever referenced the `hdf5` crate, so enabling it linked the HDF5 C library and changed no observable behaviour. `docs/HDF5_SUPPORT.md` and the feature tables nonetheless claimed `.h5` files were unsupported without it — they were never unsupported. `.h5` / `.hdf5` files store, encrypt, checksum, version, and round-trip byte-exactly in a default build, exactly as they always did. `docs/HDF5_SUPPORT.md` has been rewritten to describe what actually happens, including the one real limitation: `aim diff` compares HDF5 files at the file level, not per tensor.

### Fixed

- **License scanning could report a restricted model as MIT.** `extract_gguf_license` searched the raw bytes for the literal text `general.license`, then scanned the following **512 bytes** for any entry in `KNOWN_LICENSES`, returning the first table entry that matched anywhere in that window. Two things made that unsound. The window ran far past the value and into unrelated metadata, so a license name could be picked up from a description or a tokenizer token. Worse, the table is scanned in order with `("mit", "MIT")` first and matched as a bare substring — so any window containing `"limitations"` or `"permitted"`, which is ordinary license boilerplate, yielded **MIT, classified Permissive**, regardless of the model's actual license. A Llama-3.1 or CC-BY-NC model could therefore be reported as permissively licensed by a tool whose purpose is compliance. The value is now read from the `general.license` key itself; if the key is absent, empty, or not a string, no license is reported rather than a guessed one.

- **`aim diff` reported meaningless results for GGUF models.** `parse_gguf_header` read the tensor *count* out of the header and then fabricated that many entries named `tensor_0`, `tensor_1`, … each with an empty shape, dtype `"unknown"`, and a parameter count of zero — the metadata key/value block was never walked, so the real tensor descriptors were never reached. Two GGUF files therefore compared as identical whenever their tensor counts matched, no matter how different the tensors were: a full-precision model and its Q4_K quantization diffed as zero changes. The header is now parsed properly (metadata KV pairs are skipped by type to locate the tensor-info block, which yields real names, shapes, and `ggml_type` dtypes), with every read bounds-checked so a truncated or malformed file degrades to a partial map instead of panicking.
- **The Python package declared no runtime dependencies, and its wheel could not be built.** In `pyproject.toml`, the `dependencies` array sat *below* the `[project.urls]` header. TOML scopes every key to the table above it, so the array was parsed as `project.urls.dependencies` rather than `project.dependencies` — a silent redefinition, since nothing about the syntax is wrong. `cryptography`, `pyyaml`, `click`, `tqdm`, `platformdirs`, `filelock`, `jsonschema` and `packaging` were therefore absent from the package metadata, and an install into a clean environment would fail on the first import. maturin, which validates the table types, rejected the manifest outright with `invalid type: sequence, expected a string`, so `maturin build` and any `pip install .` through the build backend failed. The array is now inside `[project]`, with a comment recording why its position matters.

- **`GpuCrypto::decrypt` panicked in debug builds on short input.** Its GPU-routing size calculation subtracted the nonce and tag lengths without checking, so any ciphertext between 12 and 43 bytes underflowed. Fixed with a saturating subtraction and a regression test over every length up to the minimum valid blob — then removed along with the rest of the module, above.

### Breaking Changes

- **`ModelSigner::verify(&sig, path, None)` no longer returns `valid: true`.** Callers must pass the secret seed. Code that did not is not losing a check — it never had one. `SignatureVerification` gains a `signature_checked` field distinguishing "the tag failed" from "no key was supplied, so nothing was checked".
- **`aim verify` exits non-zero when verification fails or cannot be performed.** It previously exited 0 in both cases. Pipelines that gated on this command were not gating on anything; pipelines that swallowed its exit code will now surface failures.
- **The `gpu` feature flag is gone.** Not breaking in practice: no released version could be built with it enabled, because launching the OpenCL kernel needs `unsafe` and the crate forbids `unsafe_code`. `cargo build --features gpu` now fails on an unknown feature rather than on a lint.
- **The `hdf5-support` feature flag is gone.** It gated no code. `.h5` handling is unchanged in a default build.
- **Vault bundles are now format version 2**, with a checksum that is verified on import. Version-1 bundles still import, but report `checksum_verified: false`. `ImportReport` gains that field.
- **Signatures are now version 2** (RFC 2104 HMAC-SHA256 rather than `SHA-256(seed ‖ file_hash)`). Existing version-0/1 `.sig` files still verify; re-sign to upgrade.
- **Blob paths inside a bundle must be a single file name.** Anything with a directory separator, a parent reference, or a root/drive prefix is rejected. Only hand-edited bundles are affected — `export_vault` has never produced anything else.

### Version

- **1.7.0 → 2.0.0** (Python package and Helm chart synced to 2.0.0).

## [1.7.0] - 2026-07-27

### Security

- **All RUSTSEC advisories are now genuinely resolved — both ignore lists are empty.** Migrating Azure to the SDK for Rust v1 (`azure_storage_blob`) cleared the last two: `azure_core` 0.21 pinned quick-xml 0.31 (RUSTSEC-2026-0194/0195) and pulled `http-types` (RUSTSEC-2026-0174); the v1 stack uses quick-xml 0.41 and drops `http-types` entirely. Six further ignores (`fxhash`, `instant`, `paste`, `rustls-pemfile`, `lru`, `rand`) no longer matched the dependency graph and were removed. `cargo audit` and `cargo deny check` now pass with **no suppressions at all**.
- **An empty passphrase can no longer unlock a vault.** On a closed or non-interactive stdin, `rpassword` returns `""`, which was accepted and used to derive a key — so `aim list` on a fresh vault succeeded with no secret at all. The prompt now rejects an empty passphrase and points at the three supported sources.
- **9 advisories resolved.** `cargo audit` and `cargo deny check` both pass again; they had gone red as advisories were published after the last release.
  - `rustls-webpki` 0.101 (RUSTSEC-2026-0098/0099/0104 — name-constraint bypasses and a CRL parse panic) reached the tree because the AWS SDKs' default `rustls` feature selects their *legacy* hyper-0.14/rustls-0.21 stack. Fixed by building the SDKs with `default-features = false` plus `default-https-client`, which uses rustls 0.23 / webpki 0.103.
  - `pyo3` 0.24 → 0.29 (RUSTSEC-2026-0176 out-of-bounds read in `PyList`/`PyTuple` iterators, RUSTSEC-2026-0177 missing `Sync` bound).
  - `quinn-proto` (RUSTSEC-2026-0185), `crossbeam-epoch` (RUSTSEC-2026-0204), `lru` (RUSTSEC-2026-0002) resolved by `cargo update`.
  - `quick-xml` 0.31 (RUSTSEC-2026-0194/0195) remains, pinned by `azure_core` 0.21 — the last release of the legacy Azure SDK line. Both are denial-of-service via a malicious XML *response*, so they require a hostile storage endpoint and are unreachable from vault data. Documented with the upgrade path in `deny.toml` and the new `.cargo/audit.toml`; clearing them needs the rewritten `azure_storage_blob` crate, still in beta.
- **Stale advisory ignores removed** from `deny.toml` — the three `rustls-webpki` entries were suppressing advisories that are now actually fixed.


### Added

- **Non-interactive passphrase resolution** — `prompt_passphrase()` now resolves in order: `$aimodelvault_PASSPHRASE` (literal value or KMS URI) → a line piped on stdin when stdin is not a terminal → interactive masked prompt. Every passphrase-gated command (`store`, `get`, `list`, `sign`, `cloud *`, …) is now usable from CI and from agents. The env var was documented in `AGENTS.md` but had never been read by any code path.
- **KMS URIs for signing keys** — `aim sign --key` and `aim verify --key` accept a KMS URI as well as a file path; `docs/KMS.md` advertised this and it had never been implemented. The stored secret may be a keypair JSON document or a bare hex seed (`ModelSigner::keypair_from_seed` / `parse_keypair`). A KMS-backed key is never generated or written to disk.
- **`aimodelvault_HOME`** — relocates all config/data/cache directories under one root, for test isolation, containers, and per-project vaults.
- **KMS URI scheme** (`src/kms.rs`) — `KmsUri` parser and `kms::fetch` / `kms::resolve` for `env://NAME`, `file:///path`, `aws-sm://secret`, `azure-kv://vault/secret`, `vault://mount/path/key`. `docs/KMS.md` documented this scheme; no parser existed.
- **KMS backends implemented** — `file://` (rejects group/world-readable files on Unix), Azure Key Vault and HashiCorp Vault over REST (KV v2 with v1 fallback), and AWS Secrets Manager via `aws-sdk-secretsmanager` behind the `s3` feature. Previously three of four backends were stubs that returned an error unconditionally — including with `s3` enabled, which the stub's own message told users to turn on.
- **CI feature matrix job** — clippy over `default`, `s3`, `azure`, `cloud`, `api`, `database`. CI only ever built `full,graphql`, so breakage in the cloud features went unnoticed.
- **CLI integration tests** — 7 tests covering the store → list → get round-trip, wrong-passphrase rejection, `env://` and `file://` URIs, unresolvable-URI failure, and stdin. No CLI test previously exercised a passphrase-gated command.


### Fixed

- **First-run directory creation raced with its own permission tightening.** `ensure_directories` created each directory and immediately rewrote its ACL before creating the next — but several are nested under `data_dir`, and on Windows `icacls /inheritance:r` briefly leaves a directory without a usable DACL, so a concurrent create of a child failed with "Access is denied". All directories are now created first and restricted afterwards, with a single retry for a genuinely concurrent creator. Only reachable when the config directory does not yet exist, which is why it survived: on any machine that had already run `aim`, the `if !dir.exists()` guard skipped the whole path.
- **`aim convert` never worked on a vaulted model.** Version records persist `format.name()` (`"PyTorch"`), but the handler parsed it with `ModelFormat::from_extension`, which only knows extensions (`pt`/`pth`/`bin`). Every stored format silently became `Custom("pytorch")`, so path lookup always failed with "No conversion path from PyTorch to ONNX" — while the header printed `Source format: PyTorch`, because `Custom` renders its own string. `aim diff` on `name@version` had the same bug and silently fell back to a generic byte diff instead of tensor-level comparison. Added `ModelFormat::from_name` / `from_stored` with a round-trip test over every variant, and pointed both call sites at it. This went unnoticed because no CLI test could unlock a vault until this release.
- **`POST /api/v1/convert` returned plan JSON labelled as the target format.** Four converters (PyTorch→ONNX, ONNX→TensorRT, ONNX→CoreML, SafeTensors→GGUF) need an external Python toolchain and emit a JSON *plan* instead of model bytes. The REST endpoint base64-encoded that plan into `data_base64` and returned HTTP 200 with `target_format: "onnx"` — a client decoding it into `model.onnx` got a corrupt file. The response now carries `converted: false` and a `plan` object, and omits `data_base64` entirely. `.well-known/openapi.yaml`'s `ConversionResult` schema, which described a completely different shape (`success`/`output_path`/`output_size_bytes`) than the endpoint actually returns, was corrected to match.
- **"Is this a plan?" is now typed, not sniffed.** `ConversionResult` gained `plan: Option<Value>` and `is_plan()`, and `Converter` gained `produces_plan()`. The CLI previously detected this by parsing its own output looking for a `"converter"` key; any other consumer of the library API had no way to tell at all. When a conversion is a plan, `data` is empty, so no caller can write it out as a model file.
- **Multi-step conversions no longer feed a plan into the next converter.** PyTorch→ONNX→TensorRT used to run step 2 on step 1's plan JSON, producing a meaningless plan-of-a-plan. The pipeline now stops at the first step needing external tooling and returns that plan.
- **`aim convert` writes `<output>.plan.json`** rather than leaving the user with no artifact, and states plainly that no target-format file was produced.
- **`aimodelvault_VAULT` and `aimodelvault_CONFIG` were never read.** Both are documented in `AGENTS.md`; nothing consumed them. The consequence was that the entire CLI test suite believed it was writing to a tempdir and was in fact operating on the developer's real vault — which is why tests could only run serially. Implemented as documented, pointed the tests at `aimodelvault_HOME`, and the CLI suite now passes in parallel (11s, down from 103s).
- **Python package version drift** — the package was at 1.3.0 while `test_version_is_set` asserted 1.2.1, so CI's python job failed. Both are now 1.7.0, matching the crate, and the test asserts the version's shape plus equality with `Cargo.toml` rather than a literal that can rot.
- **Dockerfile pinned `rust:1.85`** while the crate's MSRV is 1.89 — the image build could not have succeeded. Bumped to 1.89, and the stale `1.5.0` OCI version labels corrected.
- **Helm chart pinned to 1.2.1** (`Chart.yaml` version/appVersion, `values.yaml` image tag), three releases behind. Synced to 1.7.0.
- **CI never ran on the default branch.** `ci.yml` and `security.yml` triggered on `main` and `develop`, but this repository's default branch is `master` — so the test suite, clippy, MSRV check, docs build, coverage, fuzz targets and benchmarks were skipped on every push, and the security audit only ever fired on its daily schedule. This is the root cause of nearly everything else fixed in this release: three red CI gates, two cargo features that did not compile, nine outstanding advisories, and a flagship command that had never worked all went unnoticed because nothing was checking. Both workflows now include `master`.
- **`mkdocs build --strict` failed with 51 warnings**, so CI's docs job was red despite v1.6.0 recording it as added and passing. 52 links from `docs/*.md` pointed at repo files outside `docs/` (`../src/kms.rs`, `../SECURITY.md`, …) which mkdocs cannot resolve for a published site; they now use absolute GitHub URLs. Fixed three genuinely broken targets, and added a `validation:` block that keeps link checking strict while allowing the one nav entry for rustdoc output that a different CI job injects after the build.
- **CI's mkdocs job installed only `mkdocs-material`** while `mkdocs.yml` declares the `minify` plugin — the build would have failed at config load, before rendering a single page. Verified the corrected install list builds strict-clean in a fresh virtualenv.
- **15 orphaned docs** added to the mkdocs nav (access control, KMS, policies, GC, tags, profiles, webhooks, plugins, lineage graph, TUI, vault bundle, validation, federation, blockchain audit, telemetry).

- **`s3` feature did not compile** — `src/cli/handlers/cloud.rs` used `ModelFormat` / `ModelMetadata` without importing them in the `cloud pull --store` path. Affected both `s3` and `azure`.
- **`azure` feature did not compile** — `put_block_blob` needs an owned body, `list_blobs().prefix()` takes a string rather than an `Option`, and `Pageable::next()` needs `StreamExt` in scope (`futures-util` added under the `azure` feature).
- **Deprecated AWS API** — `aws_config::from_env()` → `aws_config::defaults(BehaviorVersion::latest())`.
- **Clippy failures on current stable** — `manual_checked_ops`, `unnecessary_sort_by`, `manual_string_new`, `single_char_pattern`, `io_error_other`, `field_reassign_with_default`, `no_effect_underscore_binding`, `needless_range_loop`, `let_and_return`, `unit_arg`, `missing_const_for_thread_local`, `needless_borrow`. `float_cmp` is allowed per-file in test modules that assert on literal constants.
- **Two unused-code warnings in examples** — `examples/license_scan_demo.rs`, `examples/download_demo.rs`.


### Changed

- **BREAKING — Azure shared-key authentication removed.** `src/storage/azure.rs` now targets the Azure SDK for Rust v1, which provides no shared-key credential; `AZURE_STORAGE_KEY` is rejected with an error naming both alternatives rather than failing opaquely. Use `AZURE_STORAGE_SAS_TOKEN` (mint a SAS from the account key with `az storage container generate-sas`) or Entra ID via `AZURE_TENANT_ID` / `AZURE_CLIENT_ID` / `AZURE_CLIENT_SECRET`. This was the trade-off for clearing the last two advisories; docs updated across README, AGENTS.md, CLI.md, CLOUD_CLI.md, CLOUD_STORAGE.md and FEATURE_FLAGS.md.
- **Version** — 1.6.0 → 1.7.0 (Python package 1.3.0 → 1.7.0). Breaking for downstream code: `KmsBackend` gained a `File` variant, so exhaustive matches need a new arm; `aim sign`/`aim verify` take `--key` as a `String` rather than a `PathBuf`; `ConversionResult` gained a `plan` field, so struct literals need it; and `ConvertResponse.data_base64` is now optional.
- **CI clippy runs `--all-targets`** — examples, benches, and tests are now linted, not just the lib and bin.
- **CI feature matrix** extended with `python`.
- **Published crate trimmed 8.3 MiB → 3.8 MiB** (3.9 MiB → 802 KiB compressed). `Cargo.toml` now excludes repository infrastructure that was shipping to crates.io: the Next.js website, Helm charts, CI workflows, status reports, fuzz harness, and coverage artifacts. Sources, examples, tests, benches, docs and the `.well-known/` manifests are kept. Verified with `cargo publish --dry-run`, which builds the packaged crate in isolation.
- **Test count** — 2,059 → 2,088 Rust tests, 84 Python tests.

## [1.6.0] - 2026-04-06

### Added

- **Module integration tests** (`tests/module_integration_tests.rs`, 51 tests) — Cross-module integration tests covering tags/search, access control, lineage DAG, plugins, profiles, policies, validation, webhooks, quantization, evaluation, scheduler, multi-vault, signing, scanning, diff, license scanning, benchmarks, GC, and cross-module workflows
- **Property-based tests** (`tests/proptest_tests.rs`, 11 tests) — Proptest strategies for crypto round-trips, format detection, version serialization, SHA-256 invariants
- **Fuzz target expansion** (`fuzz/fuzz_targets/`, 3 new targets) — Pickle scanner, diff engine, model card parser (8 total)
- **Feature benchmarks** (`benches/feature_bench.rs`) — Criterion benchmarks for tags/search, ACL, lineage graph, plugins, profiles, policies, validation, webhooks, signing, scanning, diff, license scanning
- **CI benchmark tracking** — `benchmark-action/github-action-benchmark` job with 150% regression alert threshold
- **mkdocs nav expansion** — Added 8 missing docs to navigation: Examples, Model Download, Model Signing, Model Diffing, Engine Interop, Safety Scanning, License Scanning, Benchmarks
- **mkdocs build validation** — CI job with `mkdocs build --strict`
- **API reference generation** — Rustdoc auto-generated in CI, copied to mkdocs site, uploaded as artifact

### Changed

- **Version bump** — 1.5.0 → 1.6.0
- **MSRV** — Updated from 1.75 to 1.89 (ecosystem deps require edition 2024: `time-macros`, `async-graphql-value`, `asynk-strim`)
- **Test count** — 1,917 → 2,059

### Fixed

- **Import fixes** — Restored incorrectly removed imports in `vault.rs` (`VersionRepo`) and `database.rs` (`ChunkInfo`, `Document`)

### Security

- **`aws-lc-sys`** upgraded to v0.39.1 — fixed RUSTSEC-2026-0044 and RUSTSEC-2026-0048
- **Dependency audit** — 6 unmaintained transitive dep warnings documented in `deny.toml` ignore list; `cargo deny check` and `cargo audit` pass clean

## [1.5.0] - 2026-04-05

### Added

- **Quantization Pipeline** (`src/quantization.rs`, ~250 lines) — Profile-based quantization management with method selection (Q4_0, Q4_K_M, Q5_K_M, Q8_0, F16, F32), size estimation, and batch reporting. `QuantProfileStore` with `set`/`remove`/`get`/`list`. CLI: `aim quantize set/remove/list/estimate`
- **Evaluation Harness** (`src/evaluation.rs`, ~250 lines) — Record, compare, and query model evaluation results across suites and metrics. `EvalStore` with `record`/`get_runs`/`compare`/`suites`/`count`. CLI: `aim eval record/list/compare/suites`
- **Backup Scheduling** (`src/scheduler.rs`, ~250 lines) — Configurable vault backup schedules (hourly/daily/weekly/monthly) with rotation limits and history tracking. `BackupManager` with `set_schedule`/`remove_schedule`/`list_schedules`/`record_backup`. CLI: `aim backup set/remove/list/history`
- **Multi-Vault Management** (`src/multi_vault.rs`, ~200 lines) — Registry for managing multiple vaults with activate/deactivate switching. `VaultRegistry` with `register`/`unregister`/`activate`/`deactivate`/`list`. CLI: `aim vaults register/unregister/activate/deactivate/list`
- **4 new CLI handler files** in `src/cli/handlers/` — `quantization.rs`, `evaluation.rs`, `scheduler.rs`, `multi_vault.rs`
- **4 new Python binding classes** — `PyQuantProfileStore`, `PyEvalStore`, `PyBackupManager`, `PyVaultRegistry` (15 classes total)
- **12 new API endpoints** — REST routes for quantization profiles, evaluation runs, backup schedules, and multi-vault management under `/api/v1/`
- **28 new tests** from 4 new modules (1,865 → 1,917 total with integration tests)

### Changed

- **Version bump** — 1.4.0 → 1.5.0
- **CLI command count** — 38+ → 42+ commands
- **Test count** — 1,865 → 1,917
- **`src/lib.rs`** — 4 new `pub mod` declarations and 18 new type re-exports
- **`src/cli/args.rs`** — 4 new Commands variants, 4 new subcommand enums (QuantizeCommands, EvalCommands, BackupCommands, VaultsCommands)
- **`src/main.rs`** — Imports and match arms for all 4 new command variants
- **`src/python.rs`** — 4 new pyclass types registered in module init (11 → 15 classes)
- **`src/api/routes.rs`** — 12 new route handlers, 6 new request/response structs
- **`src/api/server.rs`** — 9 new route registrations under v1.5.0 endpoints section
- **Updated AGENTS.md** — New CLI commands, project layout, feature list
- **Updated CI/CD** — `.github/workflows/ci.yml` updated for new features

## [1.4.0] - 2026-04-04

### Added

- **Model Tags & Search** (`src/tags.rs`, ~250 lines) — Tag models with arbitrary labels and key-value annotations. Full-text search by name pattern, tags, or annotations. `TagStore` with `add_tags`/`remove_tags`/`search`. CLI: `aim tag add/remove/list/annotate`, `aim search`
- **Vault Export/Import** (`src/vault_bundle.rs`, ~200 lines) — Export entire vaults (or filtered subsets) as portable tar.gz bundles. Import bundles into new vaults with overwrite control. CLI: `aim vault-export`, `aim vault-import`
- **Garbage Collection** (`src/gc.rs`, ~200 lines) — Detect orphaned blobs, stale temp files, and reclaimable storage. Dry-run mode for safe preview. CLI: `aim gc [--dry-run]`
- **TUI Dashboard** (`src/tui.rs`, ~150 lines) — Terminal UI browser showing all vault models with version counts, sizes, formats, and timestamps. CLI: `aim browse`
- **Webhooks** (`src/webhooks.rs`, ~250 lines) — HTTP notification targets for vault events. Implements `EventSubscriber` for automatic dispatch on VaultEvent. CLI: `aim webhook add/remove/list/test`
- **Access Control** (`src/access_control.rs`, ~200 lines) — Role-based ACL (Reader/Writer/Admin) per principal with JSON persistence. CLI: `aim acl grant/revoke/list/check`
- **KMS Integration** (`src/kms.rs`, ~150 lines) — Fetch vault passphrases from external secrets managers (env, AWS Secrets Manager, Azure Key Vault, HashiCorp Vault). Library API only.
- **Model Validation** (`src/validation.rs`, ~250 lines) — Integrity probes with SHA-256 checksums per model version. CLI: `aim validate <NAME> [--version V]`
- **Retention Policies** (`src/policies.rs`, ~250 lines) — Configurable retention rules per model: max versions, max age, keep minimum. Dry-run enforcement. CLI: `aim policy set/remove/list/apply/apply-all`
- **Cross-Model Lineage DAG** (`src/lineage_graph.rs`, ~200 lines) — Directed acyclic graph tracking model derivation chains (fine-tune, quantization, distillation, merge, prune, conversion). CLI: `aim lineage-graph add/show/ancestors/descendants`
- **Plugin System** (`src/plugins.rs`, ~200 lines) — Discover, install, and uninstall plugins via JSON manifests with capability listing. CLI: `aim plugin discover/install/uninstall/list/info`
- **Config Profiles** (`src/profiles.rs`, ~200 lines) — Named configuration profiles with activate/deactivate switching and vault setting overrides. CLI: `aim profile create/remove/list/activate/deactivate/show`
- **11 new CLI handler files** in `src/cli/handlers/` for all new subcommands
- **56 new tests** from 12 new modules (1,809 → 1,865 total)

### Changed

- **Version bump** — 1.3.0 → 1.4.0
- **CLI command count** — 25+ → 38+ commands
- **Test count** — 1,809 → 1,865
- **`src/lib.rs`** — 12 new `pub mod` declarations and re-exports for all new public types
- **`src/version.rs`** — Added `list_models_owned()` and `import_version()` helper methods
- **`src/cli/args.rs`** — 13 new Commands variants, 8 new subcommand enums
- **`src/main.rs`** — Imports and match arms for all new command variants
- **Updated AGENTS.md** — New CLI commands, project layout, feature list
- **Updated README.md** — Feature comparison table, command count, test count, architecture tree
- **Updated ROADMAP.md** — v1.4.0 section with all 12 features

## [1.3.0] - 2026-04-04

### Added

- **Model download** (`src/download.rs`, ~350 lines) — Pull models from HuggingFace Hub, Ollama registry, or arbitrary URLs with streaming SHA-256 verification. `ModelSource` enum with `parse()`, `ModelDownloader` builder with `.with_hf_token()`. CLI: `aim pull <SOURCE> [-o DIR] [--sha256 HASH] [--token TOKEN] [--store] [--name NAME]`
- **Model signing & verification** (`src/signing.rs`, ~280 lines) — HMAC-SHA256 model signing with detached `.sig` files for provenance. `ModelSigner` static methods: `generate_keypair`, `sign`, `verify`, `save_signature`, `load_signature`. `SignatureVerification` struct with validity, hash match, and signer identity. CLI: `aim sign <NAME>`, `aim verify <NAME> --signature <SIG>`
- **Pickle safety scanning** (`src/scanning.rs`, ~300 lines) — Detect 7 dangerous opcodes (`REDUCE`, `GLOBAL`, `INST`, `OBJ`, `NEWOBJ`, `STACK_GLOBAL`, `NEWOBJ_EX`) and 12 suspicious patterns (`os.system`, `subprocess`, `eval`, etc.) in PyTorch/pickle files. `ScanReport` with severity classification. CLI: `aim scan [<NAME>] [--file PATH]`
- **Model diffing** (`src/diff.rs`, ~350 lines) — Tensor-level comparison for SafeTensors and GGUF models with generic binary fallback. SafeTensors header parsing, GGUF header parsing, `TensorMap` comparison. `DiffSummary` with human-readable display. CLI: `aim diff <LEFT> <RIGHT>` (supports `name@version` syntax)
- **Engine interop** (`src/interop.rs`, ~250 lines) — Register models with Ollama (`ollama create` via Modelfile generation) and LM Studio (copy to models directory). Cross-platform default path detection. CLI: `aim register <NAME> --engine <ollama|lm-studio> [--alias NAME]`
- **Benchmark metadata** (`src/benchmark.rs`, ~250 lines) — Store and query benchmark results (MMLU, HellaSwag, etc.) per model version with JSON filesystem storage. `BenchmarkStore` with `add_result`/`add_detailed_result`. CLI: `aim benchmark add <NAME> --benchmark <BENCH> --score <N>`, `aim benchmark show <NAME>`
- **License scanning** (`src/license_scan.rs`, ~370 lines) — Detect licenses from YAML frontmatter, `config.json`, GGUF metadata, and LICENSE files. 24 known licenses, SPDX normalization, `LicenseClass` classification (Permissive/Copyleft/NonCommercial/Proprietary/Unknown), compatibility warnings. CLI: `aim license-scan <PATH>`
- **7 CLI handler files** — `src/cli/handlers/{pull,sign,scan,diff,register,benchmark,license_scan}.rs`
- **46 new CLI integration tests** — expanded `tests/cli_tests.rs` from 17 to 63 tests covering all new subcommands

### Changed

- **Version bump** — 1.2.1 → 1.3.0
- **Test count** — 1,831 → 1,809 (consolidated; 623 lib + 63 CLI + 873 coverage + 250 other)
- **CLI command count** — 15+ → 25+ commands
- **Main thread stack** — Spawned `run()` on 4 MiB thread to prevent stack overflow on Windows from enlarged `Commands` enum
- **`tempfile`** moved from dev-dependencies to regular dependencies (used by handlers at runtime)
- **Updated AGENTS.md** — Added features #11–#17, 8 new CLI commands, 7 new source files in project layout
- **Updated README.md** — New features in comparison table, CLI section, architecture tree, additional capabilities

### Fixed

- **License scan Windows dedup** — Added `break` after first match in README/LICENSE file loops to prevent case-insensitive filesystem duplicates
- **Handler API patterns** — Fixed all 7 new handlers to use correct Vault API (`prompt_passphrase` with string arg, `build_vault` with config+sqlite, separate `unlock`, `get_model` not `get`, etc.)

## [1.2.1] - 2026-03-13

### Added

- **Fuzz testing targets** — 5 `cargo-fuzz` targets in `fuzz/`: `fuzz_crypto_roundtrip` (AES-256-GCM encrypt/decrypt roundtrip), `fuzz_format_detection` (ModelFormat::from_extension with arbitrary input), `fuzz_model_metadata` (ModelMetadata builder with fuzzed strings), `fuzz_version_parsing` (ModelVersion JSON deserialization), `fuzz_conversion_pipeline` (format string parsing and conversion path lookup)
- **API route tests** — 17 unit tests for `src/api/routes.rs`: `parse_format` (all 34 format aliases), `validate_model_name` (valid/empty/too-long/special-chars), `is_security_event` (all event types + negatives), `uuid_v4_simple` uniqueness, response struct serialization, stateless route handlers (`health`, `list_conversions`, `openapi_json`, `dashboard_index`)
- **API error tests** — 15 unit tests for `src/api/error.rs`: all 6 constructor methods (`bad_request`, `not_found`, `unauthorized`, `internal`, `conflict`, `rate_limited`), `IntoResponse` impl, `From<VaultError>` for all 7 match arms, `ApiErrorBody` serialization
- **API server tests** — 7 unit tests for `src/api/server.rs`: `RateLimiter` under/over limit, per-IP isolation, window reset, prune expired/active entries
- **Domain error Display tests** — exercises all `CryptoError`, `StorageError`, and `ConversionError` Display variants
- **Code coverage baseline** — 92.82% line coverage (12,094/13,029 lines) measured with cargo-llvm-cov (full features); 87.35% function coverage; 8 modules at 100% coverage
- **Performance baselines** — updated `docs/PERFORMANCE.md` with measured crypto benchmark results (AES-256-GCM, Argon2id, gzip/LZMA compression), vault benchmark results (store/retrieve, format detection, SHA-256, model card serialization), and per-module coverage table
- **Coverage improvements** — 53 new tests for low-coverage modules: `federation.rs` (VectorClock, delta computation, FederationManager lifecycle), `telemetry.rs` (event serialization, client enable/disable, tracking), `compliance.rs` (serialization, severity variants, checker toggle); total lib tests 447 → 505, full-feature tests 1,667
- **Vault benchmark fix** — fixed TempDir lifetime bug in `vault_bench.rs` (replaced `_` with `_tmp` to prevent premature directory cleanup)
- **Python bindings: VaultBuilder export** — registered `PyVaultBuilder` in the PyO3 module init and added `VaultBuilder` to `__init__.py` exports
- **Python bindings documentation** — new `docs/PYTHON_BINDINGS.md` with complete API reference for all 8 PyO3 classes, installation guide, quick start, and feature matrix
- **Python bindings: parse_format tests** — 25 Rust-side unit tests in `src/python.rs` covering all 23+ format aliases and case-insensitive parsing
- **Python test suite expansion** — added compression roundtrip tests, package init tests, vault property/error tests, and compression level tests
- **CLI integration tests** — 50 `assert_cmd` tests covering all major subcommands, help text, error handling, and format listings

### Changed

- **Python package version** — bumped from 1.1.0 to 1.2.0 in both `pyproject.toml` and `__init__.py`
- **Documentation polish** — updated 17 stale references across 10 files: test count 1,609→1,667, lib tests 447→505, coverage ~90%→92.82%, tarpaulin→cargo-llvm-cov

## [1.2.0]

### Added

- **Domain-specific error types** — introduced `CryptoError`, `StorageError`, and `ConversionError` enums in `src/error.rs` with typed variants and `From` conversions into the top-level `VaultError`. All three types are re-exported from the crate root.
- **REST API endpoints for model cards** — `GET /api/v1/models/{name}/card` generates a model card from vault metadata; `POST /api/v1/models/{name}/card` creates/overwrites a custom model card from JSON
- **REST API endpoint for compliance checks** — `GET /api/v1/compliance` runs FIPS 140-3, CVE, MITRE ATT&CK, and CMMC 2.0 checks and returns results as JSON
- **REST API endpoints for RAG** — `POST /api/v1/rag/search` searches the RAG document store; `POST /api/v1/rag/documents` adds a document with metadata
- **GraphQL routing** — wired existing `async-graphql` schema into the Axum router at `/graphql` (GET for Playground, POST for queries/mutations), gated behind `#[cfg(feature = "graphql")]`

### Changed

- **Removed `async-graphql-axum` dependency** — replaced with a manual bridge handler to avoid axum 0.7 / 0.8 version conflict; the `graphql` feature now only requires `async-graphql`
- **Fixed deprecated `TimeoutLayer::new`** — migrated to `TimeoutLayer::with_status_code(REQUEST_TIMEOUT, ...)` per tower-http 0.6.7+
- **Added `timeout` feature to tower-http** in Cargo.toml (was missing, caused compilation failure with `api` feature)
- **Removed unused `ConnectInfo` import** from `src/api/server.rs`
- **Version bump** — 1.1.0 → 1.2.0

### Changed

- **Real SafeTensors ↔ PyTorch converters** — replaced shim/plan converters with real pure-Rust implementations
  - SafeTensors → PyTorch: generates valid ZIP archives with pickle v2 bytecode and tensor data files
  - PyTorch → SafeTensors: parses ZIP archives, extracts tensor metadata from pickle bytecode, produces SafeTensors binary output
  - Full roundtrip conversion support with dtype mapping (F32↔FloatStorage, F16↔HalfStorage, BF16↔BFloat16Storage, etc.)
- **Telemetry changed to opt-in** — disabled by default for privacy
  - `TelemetryConfig::default()` now sets `enabled: false`
  - Unified environment variable handling: both `AIM_TELEMETRY_ENABLED=false` and `AIM_TELEMETRY_DISABLED=1` are respected in all code paths
  - Updated module documentation to reflect opt-in model
  - CLI `telemetry status` now shows both env var options
- **CI/CD hardening**
  - Added `permissions` and `concurrency` blocks to all GitHub Actions workflows
  - Release workflow now generates SHA-256 checksums for all binary artifacts
  - Release binaries properly renamed (e.g., `aim-linux-amd64`, `aim-darwin-arm64`)
  - Removed automatic crates.io publishing from release workflow
  - Consolidated Docker workflow: removed redundant API image job, added per-variant features
  - Fixed duplicate Alpine target in Docker workflow matrix
  - Updated `dependency-review-action` from v3 to v4
  - Added `--locked` flag to cargo install commands in CI
  - Added cargo cache to coverage job
- **deny.toml**: Rewrote for cargo-deny 0.19 schema — removed deprecated fields (`vulnerability`, `unmaintained`, `yanked`, `notice`, `unlicensed`, `copyleft`, `allow-osi-fsf-free`, `default`, `deny`), added `version = 2` to `[licenses]`, added `CC0-1.0`, `CDLA-Permissive-2.0`, `OpenSSL`, `Zlib`, `MPL-2.0` to license allow list
- **Updated qdrant-client** from 1.7 to 1.13 — migrated to builder-pattern API (`CreateCollectionBuilder`, `UpsertPointsBuilder`, `SearchPointsBuilder`, `DeletePointsBuilder`)
- **Replaced deprecated `serde_yaml`** (0.9) with maintained `serde_yml` (0.0.12) — drop-in replacement across all source and test files
- **Updated `zip` crate** from 0.6 to 4 — migrated `FileOptions` → `SimpleFileOptions` API in conversion.rs and utils.rs
- **Updated `bytes`** 1.10.1 → 1.11.1 (fixes RUSTSEC-2026-0007)
- **Updated `time`** 0.3.44 → 0.3.47 (fixes RUSTSEC-2026-0009)
- **Removed unused lancedb dependency** — v0.4 depends on arrow-arith v51 which is incompatible with Rust 1.93+
- **README overhaul** — updated test counts (331 → 1,580), fixed architecture diagram, added Architecture v2 features (API, GraphQL, federation, blockchain, GPU, streaming, VaultBuilder), fixed broken demo script paths, removed stale "NEW" labels, fixed AIMV_PATH_UPDATE link
- **AGENTS.md** — updated project layout, added `vector-db` feature, added telemetry env vars
- **Removed unused `futures` dependency** — confirmed zero usage in src/, not in any feature gate
- **Consolidated 12 coverage test files** into single `coverage_tests.rs` — reduced test binaries from 27 to 16, preserving all 1,609 tests
- **Expanded Makefile `examples` target** — now runs all 10 examples (was 2)
- **Fixed OpenAPI spec** — aligned `.well-known/openapi.yaml` with actual API routes: corrected model store path (`POST /api/v1/models/{name}`), version download path, version delete endpoint, added undocumented routes (health, audit, metrics, events, openapi.json), removed unimplemented routes (model cards, compliance, RAG, GraphQL)
- **Fixed Helm chart health probes** — corrected probe paths from `/health` to `/api/v1/health`, updated image tag to 1.1.0, added `startupProbe` for slow cold starts
- **Rewrote docs/PROJECT_STRUCTURE.md** — updated entire file to reflect current codebase: added 15+ missing src/ modules (crypto/gpu.rs, streaming.rs, cli/, api/, rag/, model_card.rs, blockchain.rs, federation.rs, telemetry.rs, traits.rs, version_sqlite.rs), updated tests/ from 6 to 14 files, examples/ from 5 to 10, docs/ with all new files, fixed license from MIT to AGPL-3.0, added deploy/, website/, .well-known/ directories
- **Updated reports/TEST_COVERAGE.md** — corrected test count from 119 to 1,609, updated test binary count to 16, added all missing test file entries, expanded coverage matrix with 8 new categories (model cards, CLI, VaultBuilder, blockchain, federation, telemetry, format conversion, RAG)
- **Updated all dependencies** — ran `cargo update` (141 packages updated within semver-compatible ranges), all 1,609 tests pass
- **Fixed format count references** — corrected "22 formats" to "23+ formats" across reports/TEST_COVERAGE.md, reports/COMPREHENSIVE_TEST_REPORT.md, reports/UTILITIES_IMPLEMENTATION_COMPLETE.md
- **Fixed MSRV references** — corrected "Rust 1.70+" to "Rust 1.75+" in docs/PROJECT_SUMMARY.md, updated Dockerfile example in docs/SECURITY_HARDENING.md to `rust:1.85-slim-bookworm`
- **Updated website version** — changed version badge from v1.0.0 to v1.1.0 in Header.tsx and page.tsx
- **Fixed stale test count in MIGRATION.md** — corrected "330+ tests" to "1,609+ tests"
- **Fixed stale test count in ROADMAP.md** — corrected "227 tests" to "1,609 tests"
- **Fixed website test count** — corrected "331+" to "1,609" in homepage stats
- **Fixed DEVELOPMENT.md MSRV** — corrected "Rust 1.70" to "Rust 1.75"
- **Fixed remaining "22+" → "23+" format count references** — docs/EXECUTIVE_SUMMARY.md, docs/api/formats.rst, reports/COMPREHENSIVE_TEST_REPORT.md, reports/PRODUCTION_READY.md, reports/PROJECT_COMPLETE.md, website Python docs
- **Root directory cleanup** — moved 5 Python coverage scripts (`analyze_cov.py`, `analyze_coverage.py`, `parse_coverage.py`, `parse_extra.py`, `parse_uncovered.py`) to `scripts/`, deleted tarpaulin artifacts, added `tarpaulin-report.json`, `tarpaulin_stderr.log`, `.cache/` to `.gitignore`
- **README overhaul (round 2)** — removed duplicate Project Structure section, removed duplicate Documentation section with garbled emoji headings, consolidated documentation table with 7 new entries (Architecture, Providers & Formats, Version Control, Cloud Storage, Model Cards, XDG, Roadmap, Changelog), fixed last `(22+)` → `(23+)` format count, updated architecture tree with new `scripts/` directory
- **Fixed all remaining "22+" → "23+" format count references** — ROADMAP.md, examples/huggingface_demo.rs, docs/EXECUTIVE_SUMMARY.md, docs/TOP_10_FEATURES.md, docs/guide/formats.rst, docs/archived/LAUNCH_READINESS.md, docs/archived/LAUNCH_READY.md, reports/COMPREHENSIVE_TEST_REPORT.md, reports/FEATURES_DEMO.md, reports/PRODUCTION_READY.md, reports/PROJECT_COMPLETE.md, reports/TESTING_COMPLETE.md, reports/UTILITIES_IMPLEMENTATION_COMPLETE.md

### Fixed

- Fixed all clippy warnings (29 warnings → 0)
  - Replaced `field_reassign_with_default` patterns with struct init syntax across src/ and tests/
  - Replaced `vec_init_then_push` with pre-initialized `vec![]` literals
  - Fixed `unused_must_use` on `cache_results()` calls
  - Fixed `unnecessary_get_then_check` in traits.rs
  - Fixed `unwrap_on_ok` / `expect_on_ok` in error.rs test
  - Removed unused imports (`EventSubscriber`, `VersionRepo`, `super`)
  - Fixed constant assertions (`assert!(X > 0)` → `assert_ne!(X, 0)`)
  - Suppressed deprecated `assert_cmd::Command::cargo_bin` warning
- Fixed 26 broken internal links across 8 docs/ files
  - Added `../` prefix for root-level files referenced from docs/ (README.md, LICENSE, SECURITY.md, CONTRIBUTING.md, DEVELOPMENT.md, FORMATS.md)
  - Removed redundant `docs/` prefix for same-directory references (QUICKSTART.md, CLI.md, UTILITIES.md)
  - Fixed reports/ directory references (FEATURES_DEMO.md, PRODUCTION_READY.md)
  - Removed links to non-existent files (COMPLIANCE.md, CRYPTO.md, API.md)
  - Fixed incorrect license references (MIT → AGPL-3.0-or-later)
  - Replaced manual `div_ceil` with standard library method in crypto/streaming.rs
  - Used `keys()` iterator instead of destructuring in conversion.rs
  - Added `#[allow(clippy::too_many_arguments)]` where appropriate

### Added

- **Next.js documentation website** (`website/`)
- **docs/FEATURE_FLAGS.md** — comprehensive documentation of all Cargo feature flags with build recipes
- **docs/PERFORMANCE.md** — benchmark baseline for encryption, hashing, compression, model card serialization
- **docs/GPU_ACCELERATION.md** — user guide for OpenCL GPU-accelerated encryption
- **docs/archived/** — moved stale launch readiness docs out of main docs/
- **ROADMAP: Future Improvements** section — documented error type granularity, API expansion, GraphQL routing as v1.2.0+ items
  - 21 documentation pages covering all features
  - Responsive layout with sidebar navigation and mobile menu
  - Light/dark theme with CSS custom properties
  - Reusable components: CodeBlock, Callout, FeatureCard
  - Static generation — all 25 routes prerendered
- Updated README badges and stats (331 tests, v1.0.0, Rust 1.75+)
- Updated ROADMAP version header to v1.0.0

## [1.0.0] - 2026-02-10

### Changed

- **Version bump to 1.0.0** — first production-stable release
  - Cargo.toml: `0.1.0` → `1.0.0`
  - pyproject.toml: `0.1.0` → `1.0.0`, classifier `Alpha` → `Production/Stable`
  - CLI version: `0.1.0` → `1.0.0`
  - OpenAPI spec: `0.5.0` → `1.0.0`

### Added

- **Multi-stage Dockerfile** with Alpine (default, ~12 MB) and Debian variants
  - Static musl binary via `x86_64-unknown-linux-musl` target
  - Non-root user, tini init, XDG volume mounts
  - Configurable `FEATURES` build arg (e.g., `--build-arg FEATURES=api`)
  - `.dockerignore` for minimal build context
- **Kubernetes Helm chart** (`deploy/helm/ai-model-vault/`)
  - Deployment with hardened security context (non-root, read-only FS, drop all caps)
  - Service (ClusterIP), Secret (auto-generated JWT), ServiceAccount
  - PersistentVolumeClaims for data, config, and cache
  - Optional Ingress with TLS support
  - HorizontalPodAutoscaler
  - Values: image, replicas, API config, persistence, resources, probes, autoscaling
- **Docker CI/CD workflow** (`.github/workflows/docker.yml`)
  - Builds and pushes Alpine, Debian, and API images to GHCR on tag push
  - Docker Buildx with GitHub Actions cache
  - OCI metadata labels via `docker/metadata-action`
- **Comprehensive migration guide** (`docs/MIGRATION.md`)
  - Covers Rust crate, Python package, CLI, REST API, Docker, and Kubernetes
  - Breaking changes summary, data migration notes, environment variables
- **Publication readiness metadata**
  - Cargo.toml: added `readme`, `homepage`, `documentation`, `rust-version` fields
  - pyproject.toml: added `[project.urls]` section (Homepage, Docs, Repo, Issues, Changelog)
  - Keywords trimmed to 5 for crates.io compliance

## [0.5.0] - 2026-02-10

### Added

- **REST API server** (`src/api/`, ~1200 lines, behind `api` feature flag)
  - Axum 0.7 HTTP server with 14 RESTful endpoints
  - JWT authentication (`jsonwebtoken` 9.3) with Bearer token auth
  - Endpoints: health, auth/token, models (list/get/store), versions (list/get/delete), lineage, conversions (list/convert), stats, audit
  - Multipart file upload for model storage
  - Base64-encoded conversion API for format conversion over HTTP
  - CORS support via `tower-http` with `--cors-permissive` flag
  - Request body size limits (default 512 MiB)
  - HTTP request tracing via `tower-http::trace`
- **OpenAPI 3.1 specification** at `/api/v1/openapi.json`
  - Complete API documentation with schemas, parameters, and security definitions
- **Embedded web dashboard** served at `/`
  - Single-page HTML/JS/CSS application (no build step required)
  - Model inventory browser with version drill-down
  - Storage statistics (models, versions, size, files)
  - Audit log viewer, conversion registry browser
  - Passphrase-based login with JWT session management
- **CLI `serve` command** (`aim serve`)
  - Flags: `--host`, `--port`, `--jwt-secret`, `--token-expiry`, `--cors-permissive`, `--no-dashboard`
  - Environment variables: `AIM_HOST`, `AIM_PORT`, `AIM_JWT_SECRET`
- **15 API tests** (3 auth unit + 12 integration via tower `oneshot`)
- Dependencies: axum 0.7, tower 0.5, tower-http 0.6, jsonwebtoken 9.3, utoipa 5, base64 0.22, hyper 1.4

## [0.4.0] - 2026-02-10

### Added

- **Format conversion pipeline** (`src/conversion.rs`, ~1350 lines)
  - `Converter` trait with `convert()`, `validate()`, `name()`, `source_format()`, `target_format()`
  - `ConversionPipeline` with BFS multi-step path finding and `with_builtins()` factory
  - `ConversionOptions`: quantization, opset_version, tolerance, preserve_metadata, extra params
  - `ConversionResult`: output data, conversion path, input/output sizes, optional validation report
  - `ConversionProgress` with step tracking and Display impl
  - `ValidationReport` and `ValidationCheck` structures
- **10 built-in format converters**
  - Pure Rust: SafeTensors↔Raw roundtrip, GGUF header parser, ONNX metadata extractor
  - Shim converters (JSON conversion plans): SafeTensors↔PyTorch, PyTorch→ONNX, ONNX→TensorRT, ONNX→CoreML, SafeTensors→GGUF
- **Magic-bytes validation** for SafeTensors, GGUF, PyTorch (ZIP/pickle), ONNX (protobuf), TFLite
- **CLI commands**
  - `aim convert` with `--opset`, `--validate`, `--plan-only` flags
  - `aim list-conversions` to show all registered converters and multi-step paths
- **53 conversion tests** (22 unit + 31 integration)

## [0.3.0] - 2026-02-10

### Added

- **Native Python bindings via PyO3** (`src/python.rs`, ~640 lines)
  - `Vault`: create, unlock, lock, store_model, get_model, list_models, list_versions, get_lineage, delete_version, get_stats, change_passphrase
  - `VaultConfig`: XDG-compliant configuration with optional custom vault directory
  - `ModelFormat`: 23+ format detection with name/extension properties
  - `ModelMetadata`: builder-style constructor (name, format, description, framework, task, architecture, parameters)
  - `ModelVersion`: read-only version snapshot (version, checkpoint_id, timestamp, format, size, checksum)
  - `ModelCard`: create, set_training_data, add_metric, add_metadata, serialization (JSON/YAML/Markdown), deserialization
  - `sha256_hex()`: FIPS-compliant SHA-256 hex digest
  - `version()`: native library version string
- `python` feature flag in Cargo.toml gating PyO3 dependency
- maturin build backend in `pyproject.toml` (replaced setuptools)
- Native import with graceful fallback in `__init__.py` (`_NATIVE` flag)
- **Streaming API** for large models
  - `Vault.store_model_streamed()`: ingest from any iterable of `bytes` chunks
  - `Vault.get_model_streamed()`: retrieve as `ModelStream` iterator (default 8 MiB chunks)
  - `ModelStream`: Python iterator with `total_size`, `remaining` properties
  - Rust `ModelStream` + `Vault::store_model_streamed()` / `Vault::get_model_chunked()`
- **Sphinx documentation** (`docs/`)
  - API reference: Vault, VaultConfig, ModelFormat, ModelMetadata, ModelVersion, ModelCard, utilities
  - User guides: vault lifecycle, format detection, model cards, version control
  - Quick start and installation guides (uv-based)

### Changed

- Python package now uses native Rust FFI instead of CLI subprocess wrappers when built with maturin
- `pyproject.toml`: build system switched from setuptools to maturin ≥1.7

## [0.2.0] - 2026-02-10

### Changed

- **License**: Switched from MIT to AGPL-3.0-or-later with commercial dual-license option + CLA
- **Architecture**: Split `rag.rs` (2,168 lines) into 7 submodules with backward-compatible re-exports
- **Architecture**: Split `main.rs` (2,931 lines) into 87-line dispatcher + `cli/` module tree (11 files)
- **Performance**: `ModelFormat::name()` and `extension()` return `&'static str` (zero allocation)
- **Performance**: `model_card.rs` uses `write!()` instead of `format!()+push_str()`, `String::with_capacity(2048)`

### Added

- `COMMERCIAL_LICENSE.md` for proprietary/commercial licensing inquiries
- `Vault::key_manager()` getter (resolves dead_code suppression)
- `VersionControl::vault_path()` getter (resolves dead_code suppression)
- `ComplianceChecker` gated methods with `enabled_checks` map
- 19 new tests (246 total): `change_passphrase`, `audit` logging, `FormatConverter`, `cleanup_old_versions`, `verify_checksum`, compliance check toggling
- `vault_bench` benchmarks: store/retrieve, format detection, SHA-256, model card ser/de

### Fixed

- Resolved all 5 `#[allow(dead_code)]` annotations in production code
- Removed redundant `CachedResult.query_hash` field; used timestamp in LRU eviction as tiebreaker

### Removed

- 10 temporary artifacts from root (test outputs, status files)
- Moved 23 status/completion files to `reports/`
- Moved 12 guides/demo scripts to `docs/`

## [0.1.1] - 2026-02-07

### Fixed

- **Critical**: Replaced panicking `.expect()` in `Vault::new()` with `match` returning `Result`
- **Critical**: Guarded `validate_sql_identifier()` against empty-string panic
- Deprecated `actions-rs/toolchain@v1` → `dtolnay/rust-toolchain@stable` in CI
- Deprecated `actions/create-release@v1` → `softprops/action-gh-release@v2`
- Fixed binary name `aimv` → `aim` in release.yml
- Made heavyweight Python deps optional in `pyproject.toml` (`[project.optional-dependencies] ml`)

### Added

- 40+ Python tests for ModelFormat, VaultConfig, Vault, FIPSCrypto
- `#[must_use]` annotations on all 15 pure functions
- `///` doc comments on 17+ public types and builder methods
- Warning docstring to `fips.py` documenting PBKDF2 vs Argon2id incompatibility

### Changed

- Synced Python `ModelFormat` enum 1:1 with Rust's 23-variant enum
- Committed `Cargo.lock` for reproducible binary builds
- Updated test count references from 171/119 → 227

## [0.1.0] - 2025-11-03

### Added

- **Core Vault System**
  - FIPS 140-3 compliant encryption using AES-256-GCM
  - Argon2id key derivation function (64MB memory, 3 iterations)
  - XDG Base Directory compliance for cross-platform support
  - Version control system with complete checkpoint history
  - Secure key storage with memory zeroization
  - Comprehensive audit logging for compliance

- **Model Format Support (23+ formats)**
  - PyTorch (.pt, .pth, .bin)
  - TensorFlow (.pb, .keras, .h5)
  - ONNX (.onnx)
  - Safetensors (.safetensors)
  - GGUF (.gguf) - Quantized LLMs
  - TensorRT (.plan)
  - TFLite (.tflite)
  - MLX (.npz) - Apple Silicon
  - Core ML (.mlmodel, .mlpackage)
  - And 13+ more formats
  - Automatic format detection
  - Metadata management

- **Compression**
  - Gzip (fast, moderate compression)
  - LZMA (slow, high compression)
  - Zlib (balanced)
  - Configurable compression levels (Fast/Balanced/Maximum)
  - Compression analysis and recommendations

- **Model Utilities (8 Components)**
  - ModelArchive: TAR/ZIP archiving for model backup
  - CompressionAnalyzer: Compression ratio analysis
  - RetrievalOptimizer: LRU cache for fast model access
  - QuantizationInfo: Track 10 quantization schemes
  - PruningInfo: Pruning metadata and sparsity calculation
  - ModelAnalyzer: Size and parameter analysis
  - ModelExporter: Export with JSON metadata
  - ModelDeduplicator: SHA-256 duplicate detection

- **Cloud Storage Support** ⭐ NEW
  - **StorageBackend trait**: Pluggable storage architecture
  - **AWS S3 backend**: Full S3 support with multipart uploads
  - **Azure Blob Storage backend**: Azure cloud storage integration
  - **Google Cloud Storage backend**: GCS support
  - **Async operations**: Non-blocking cloud uploads/downloads
  - **Multiple authentication methods**: IAM roles, access keys, service accounts
  - **Optional features**: Build only what you need (s3, azure, gcs, cloud)
  - **Complete documentation**: 600+ line cloud storage guide
- CLI interface with full command set
  - Core commands: `init`, `store`, `get`, `list`, `versions`, `lineage`, `delete`, `stats`, `compliance`
  - **Utility commands**: `archive`, `extract`, `analyze`, `deduplicate`, `export`, `cache`
- **Model Utilities Module** with comprehensive AI model operations:
  - **ModelArchive**: TAR/ZIP archiving for multiple models
  - **CompressionAnalyzer**: Compression ratio analysis and format-specific estimates
  - **RetrievalOptimizer**: LRU cache for fast model retrieval
  - **QuantizationInfo**: Quantization metadata tracking (10 schemes: FP32, FP16, INT8, Q4_0, etc.)
  - **PruningInfo**: Pruning information and sparsity calculation
  - **ModelAnalyzer**: Model analysis with human-readable size/parameter formatting
  - **ModelExporter**: Export models with JSON metadata
  - **ModelDeduplicator**: SHA-256 based duplicate detection and similarity scoring
- **RAG & AI Agent Integration** ⭐ NEW
  - Document store with vector embeddings
  - Knowledge base with text chunking
  - Rule engine for business logic
  - Retrieval cache with LRU eviction
  - Model Context Protocol (MCP) tools
  - Database abstraction layer
  - 23 comprehensive RAG tests

- **CLI Interface (15 Commands)**
  - Core: init, unlock, store, get, list, versions, lineage, delete, stats
  - Utilities: archive, extract, analyze, deduplicate, export, cache
  - Compliance: compliance check
  - Interactive help system
  - User-friendly error messages

- **Comprehensive Test Suite (148 tests)**
  - 37 library unit tests
  - 22 configuration and error tests
  - 14 cryptography tests
  - 15 format detection tests
  - 8 integration tests
  - 38 utilities tests
  - 23 RAG tests
  - 100% passing rate

- **Example Programs (4 demos)**
  - `basic_usage.rs`: Core vault operations
  - `security_demo.rs`: Security features
  - `utilities_demo.rs`: Model utilities showcase
  - `rag_demo.rs`: RAG pipeline demonstration

- **Complete Documentation (5,000+ lines)**
  - Quick start guide (5-minute tutorial)
  - CLI reference (all 15 commands)
  - Utilities guide (600+ lines)
  - RAG guide (600+ lines)
  - MCP tools guide (500+ lines)
  - Cloud storage guide (600+ lines)
  - HDF5 support guide
  - Security policy
  - Development guide
  - Test coverage report

### Security

- **FIPS 140-3** approved cryptographic algorithms
- **Authenticated encryption** with AES-256-GCM (128-bit auth tags)
- **Secure key derivation** with Argon2id (64MB memory, 3 iterations)
- **SHA-256 integrity** verification for all stored models
- **Memory zeroization** for sensitive data (keys, passphrases)
- **Audit logging** for all security-relevant operations
- **CMMC 2.0 Level 2** compliance (17 controls implemented)
- **MITRE ATT&CK** framework alignment (T1552, T1486, T1078, T1005)
- **CVE scanning** with automated vulnerability checks

### Changed

- Made HDF5 support optional (requires system library installation)
- Separated HDF5 into `hdf5-support` feature flag
- Updated build to work without HDF5 by default
- Optimized compression for large model files

### Fixed

- HDF5 build dependency issue (now truly optional)
- Build failures on systems without HDF5 installed
- Generic array deprecation warnings
- Cross-platform path handling improvements

### Documentation

- Added comprehensive HDF5 support guide
- Created launch readiness checklist
- Updated README with HDF5 installation instructions
- Expanded cloud storage documentation
- Added troubleshooting guides

## Future Releases

### Planned for v0.3.0

- Native Python bindings (PyO3/maturin)
- Direct Python API without subprocess
- PyPI publication as `aimodelvault`

### Planned for v0.4.0

- Real model format conversion pipeline
- PyTorch ↔ ONNX, SafeTensors ↔ PyTorch, GGUF ↔ SafeTensors

---

[0.2.0]: https://github.com/nervosys/AIModelVault/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/nervosys/AIModelVault/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/nervosys/AIModelVault/releases/tag/v0.1.0
