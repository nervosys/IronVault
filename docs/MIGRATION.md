# Migration Guide

- [v4.x → v5.0.0 — the IronVault rename](#v4x--v500--the-ironvault-rename) *(current)*
- [v0.x → v1.0.0](#migration-guide--v0x--v100) *(historical; describes the project under its former name)*

---

# v4.x → v5.0.0 — the IronVault rename

v5.0.0 renames the project from AI Model Vault to IronVault. Nothing about the
vault format, the cryptography, or the on-disk layout changed. What changed is
every name you type.

## What you have to do

### 1. Install under the new name

```bash
# Rust
cargo uninstall ai-model-vault
cargo install ironvault          # binary is now `iv`, not `aim`

# Python
pip uninstall aimodelvault
pip install ironvault
```

`ai-model-vault` and `aimodelvault` remain on crates.io and PyPI at 4.6.x and
still install. They will not receive further releases. There is deliberately no
forwarding meta-package: a package that installs something other than its own
name is worse than one that stops moving.

### 2. Update your code

```rust
// before
use ai_model_vault::{Vault, VaultConfig};
// after
use ironvault::{Vault, VaultConfig};
```

```python
# before
import aimodelvault
# after
import ironvault
```

### 3. Rename your commands

`aim` is now `iv`. Every subcommand, flag, and exit code is unchanged, so this
is a pure substitution:

```bash
aim list --format json     # before
iv list --format json      # after
```

### 4. Rename your environment variables (not urgent)

4.x had two unrelated prefixes; 5.0 has one.

| 4.x | 5.0 |
| --- | --- |
| `aimodelvault_HOME` | `IRONVAULT_HOME` |
| `aimodelvault_CONFIG` | `IRONVAULT_CONFIG` |
| `aimodelvault_VAULT` | `IRONVAULT_VAULT` |
| `aimodelvault_PASSPHRASE` | `IRONVAULT_PASSPHRASE` |
| `aimodelvault_FEDERATION_PASSPHRASE` | `IRONVAULT_FEDERATION_PASSPHRASE` |
| `AIM_JWT_SECRET` | `IRONVAULT_JWT_SECRET` |
| `AIM_HOST`, `AIM_PORT` | `IRONVAULT_HOST`, `IRONVAULT_PORT` |
| `AIM_REVOCATION_STORE` | `IRONVAULT_REVOCATION_STORE` |
| `AIM_TELEMETRY_ENABLED` / `_DISABLED` | `IRONVAULT_TELEMETRY_ENABLED` / `_DISABLED` |

**The old names still work in 5.0.** Each one warns once to stderr the first
time it is read. The warning prints the variable name only, never its value —
several of these carry passphrases. Support for the old names is removed in
6.0, so treat this as a deprecation rather than a break.

`DO_NOT_TRACK`, `AWS_*`, `AZURE_*`, and `OTEL_*` are third-party or cross-vendor
conventions and are unaffected.

## What you do *not* have to do

### Your vault does not move

The Rust XDG layout was already name-neutral and is unchanged:

```
~/.config/ai/models/          # config
~/.local/share/ai/models/     # data, vaults, logs
~/.cache/ai/models/           # cache
```

Existing vaults, versions, audit logs, and blockchain audit chains are found
exactly where they were. No export/import step, no re-encryption.

### Your encrypted files still open

The on-disk and on-wire format identifiers are deliberately **not** renamed:

- `AIMVSEAL` — the 8-byte magic on every sealed cloud object and federation transfer
- `AIMV` — the 4-byte magic on every chunked-encrypted model
- `aimv://` — the URI scheme, and `aimv:` the JSON-LD term prefix

Renaming these would not rename the bytes already written to your bucket and
your disk; it would just stop 5.0 from recognising them. Sealed objects would
read as plaintext and encrypted models would be rejected as corrupt. Any
`aimv://` URI you have stored remains valid.

## systemd deployments

The unit, user, and paths are renamed. `install.sh` does **not** migrate an
existing 4.x install — it provisions the new one alongside it, so nothing is
destroyed while you verify.

| 4.x | 5.0 |
| --- | --- |
| `aim-server.service` | `ironvault-server.service` |
| user/group `aim` | user/group `ironvault` |
| `/etc/aim/server.env` | `/etc/ironvault/server.env` |
| `/var/lib/aim` | `/var/lib/ironvault` |
| `/usr/local/bin/aim` | `/usr/local/bin/iv` |

```bash
sudo systemctl disable --now aim-server

sudo ./deploy/systemd/install.sh --dry-run    # inspect first
sudo ./deploy/systemd/install.sh

# carry over the secrets you already generated, rather than rotating
# them unnecessarily; the file stays 0600 root-owned
sudo cp /etc/aim/server.env /etc/ironvault/server.env
sudo chown root:root /etc/ironvault/server.env
sudo chmod 0600 /etc/ironvault/server.env

sudo mv /var/lib/aim/revocations.json /var/lib/ironvault/
sudo chown -R ironvault:ironvault /var/lib/ironvault

sudo systemctl enable --now ironvault-server
```

Once the new unit is healthy, remove `/etc/aim`, `/var/lib/aim`, and the `aim`
user. Verify first — the JWT secret in the old `server.env` is the only copy,
and deleting it invalidates every issued token.

## Python-only users: your directories do move

Unlike the Rust side, the Python package derived its directories from the
package name:

| 4.x | 5.0 |
| --- | --- |
| `~/.config/aimodelvault/` | `~/.config/ironvault/` |
| `~/.local/share/aimodelvault/` | `~/.local/share/ironvault/` |
| `~/.cache/aimodelvault/` | `~/.cache/ironvault/` |

If you used the Python API's own config rather than the CLI, move them:

```bash
mv ~/.config/aimodelvault  ~/.config/ironvault
mv ~/.local/share/aimodelvault ~/.local/share/ironvault
rm -rf ~/.cache/aimodelvault   # cache only; safe to discard
```

## Repository and links

`github.com/nervosys/AIModelVault` is now `github.com/nervosys/IronVault`.
GitHub redirects the old URL, including `git remote` operations, so existing
clones keep working. To update a clone explicitly:

```bash
git remote set-url origin https://github.com/nervosys/IronVault.git
```

---

# Migration Guide — v0.x → v1.0.0

> Historical. This section describes the project under its former name, AI
> Model Vault, and its former binary `aim`. It is preserved as an accurate
> record of what v1.0.0 shipped; see the section above for current names.

This document covers upgrading from any v0.x release of AI Model Vault to v1.0.0.

---

## Table of Contents

1. [Overview](#overview)
2. [Breaking Changes](#breaking-changes)
3. [Rust Crate Migration](#rust-crate-migration)
4. [Python Package Migration](#python-package-migration)
5. [CLI Migration](#cli-migration)
6. [API Migration](#api-migration)
7. [Docker Deployment (New)](#docker-deployment-new)
8. [Kubernetes Deployment (New)](#kubernetes-deployment-new)
9. [Configuration Changes](#configuration-changes)
10. [Data Migration](#data-migration)

---

## Overview

v1.0.0 is the first production-stable release of AI Model Vault. It encompasses
all features from v0.1.0 through v0.5.0 with hardened security, comprehensive
testing (1,831+ tests), and deployment-ready packaging.

| Version    | Highlights                                                   |
| ---------- | ------------------------------------------------------------ |
| v0.1.0     | Core vault, encryption, model cards, XDG compliance          |
| v0.1.1     | Hardening pass                                               |
| v0.2.0     | Code quality & architecture refactor                         |
| v0.3.0     | PyO3 Python bindings, Sphinx docs, streaming API             |
| v0.4.0     | Format conversion pipeline (10 converters, BFS path-finding) |
| v0.5.0     | REST API (14 endpoints), JWT auth, embedded web dashboard    |
| **v1.0.0** | **Production release — Docker, Helm, publication readiness** |

---

## Breaking Changes

v1.0.0 introduces **no breaking API changes** relative to v0.5.0. The public
Rust API, Python bindings, CLI commands, and REST endpoints are all
backward-compatible. The changes are additive:

- Docker and Helm deployment support (new)
- Version string bumps across all surfaces
- Classifier updated from "Alpha" to "Production/Stable"
- OpenAPI spec version now `1.0.0`

If you are upgrading from a version earlier than v0.5.0, review the relevant
section below for each intermediate release.

---

## Rust Crate Migration

### From v0.4.x or v0.5.x

Update your `Cargo.toml`:

```toml
[dependencies]
ai-model-vault = "1.0.0"
```

No source code changes required. All public types, traits, and functions
retain their existing signatures.

### From v0.3.x

The format conversion API was added in v0.4.0. If you were using the crate
purely for vault operations, no changes are needed. If you want conversion:

```rust
use ai_model_vault::conversion::{ConversionRegistry, ModelFormat};

let registry = ConversionRegistry::new();
let path = registry.find_conversion_path(ModelFormat::ONNX, ModelFormat::SafeTensors);
```

### From v0.2.x or earlier

The Python bindings module (`python` feature) was added in v0.3.0.
Enable it if needed:

```toml
[dependencies]
ai-model-vault = { version = "1.0.0", features = ["python"] }
```

---

## Python Package Migration

### From any v0.x

Update the package:

```bash
pip install --upgrade aimodelvault==1.0.0
```

Or with optional ML dependencies:

```bash
pip install "aimodelvault[ml]==1.0.0"
```

The Python API is unchanged. All functions in `aimodelvault._native` retain
their existing signatures.

---

## CLI Migration

### From any v0.x

Replace the `aim` binary. If installed via cargo:

```bash
cargo install ai-model-vault --version 1.0.0
```

All existing commands work identically. Verify:

```bash
aim --version
# aim 1.0.0
```

### New: Docker-based CLI

```bash
docker run --rm -v $(pwd)/vault:/data ghcr.io/nervosys/ai-model-vault:1.0.0 \
  aim store my-model --format safetensors --file model.safetensors
```

---

## API Migration

### From v0.5.x

The REST API is fully backward-compatible. The only change is the OpenAPI
spec version field (`0.5.0` → `1.0.0`). All 14 endpoints retain their
existing request/response schemas:

| Endpoint                        | Method | Status    |
| ------------------------------- | ------ | --------- |
| `/health`                       | GET    | Unchanged |
| `/auth/token`                   | POST   | Unchanged |
| `/models`                       | GET    | Unchanged |
| `/models`                       | POST   | Unchanged |
| `/models/{name}`                | GET    | Unchanged |
| `/models/{name}`                | DELETE | Unchanged |
| `/models/{name}/versions`       | POST   | Unchanged |
| `/models/{name}/versions`       | GET    | Unchanged |
| `/models/{name}/versions/{ver}` | GET    | Unchanged |
| `/models/{name}/versions/{ver}` | DELETE | Unchanged |
| `/models/{name}/lineage/{ver}`  | GET    | Unchanged |
| `/conversions`                  | GET    | Unchanged |
| `/convert`                      | POST   | Unchanged |
| `/stats`                        | GET    | Unchanged |

### From pre-v0.5.0

The REST API did not exist before v0.5.0. Enable it with:

```bash
cargo build --features api
aim serve --host 0.0.0.0 --port 8080
```

---

## Docker Deployment (New)

> **Removed in 4.5.0.** The Dockerfile, published images, and Helm chart no
> longer exist. This section is kept as a record of what v1.0.0 shipped.

v1.0.0 introduced first-class Docker support:

```bash
# Build locally
docker build -t aim:latest .
docker build --build-arg FEATURES=api -t aim:api .

# Pull from registry (when published)
docker pull ghcr.io/nervosys/ai-model-vault:1.0.0
```

Alpine (default) and Debian variants are available:

```bash
docker build --target alpine -t aim:alpine .
docker build --target debian -t aim:debian .
```

### Volumes

| Mount Point | Purpose                         |
| ----------- | ------------------------------- |
| `/data`     | Vault data (XDG_DATA_HOME)      |
| `/config`   | Configuration (XDG_CONFIG_HOME) |
| `/cache`    | Cache files (XDG_CACHE_HOME)    |

---

## Kubernetes Deployment (New)

> **Removed in 4.5.0.** The chart no longer exists; this records v1.0.0.

v1.0.0 provided a Helm chart at `deploy/helm/ai-model-vault/`:

```bash
# Install
helm install aim deploy/helm/ai-model-vault/ \
  --set api.jwtSecret=your-secret-key

# With ingress
helm install aim deploy/helm/ai-model-vault/ \
  --set ingress.enabled=true \
  --set ingress.hosts[0].host=aim.example.com \
  --set ingress.hosts[0].paths[0].path=/ \
  --set ingress.hosts[0].paths[0].pathType=Prefix

# Upgrade
helm upgrade aim deploy/helm/ai-model-vault/
```

The chart includes:
- Deployment with security context (non-root, read-only FS, drop all caps)
- Service (ClusterIP)
- Secret (auto-generated JWT secret)
- PersistentVolumeClaims (data, config, cache)
- Optional Ingress
- Optional HorizontalPodAutoscaler
- ServiceAccount

---

## Configuration Changes

### Environment Variables

All existing environment variables are unchanged:

| Variable          | Since  | Purpose            |
| ----------------- | ------ | ------------------ |
| `AIM_HOST`        | v0.5.0 | API listen address |
| `AIM_PORT`        | v0.5.0 | API listen port    |
| `AIM_JWT_SECRET`  | v0.5.0 | JWT signing key    |
| `XDG_DATA_HOME`   | v0.1.0 | Data directory     |
| `XDG_CONFIG_HOME` | v0.1.0 | Config directory   |
| `XDG_CACHE_HOME`  | v0.1.0 | Cache directory    |

New in v1.0.0:

| Variable           | Purpose                                       |
| ------------------ | --------------------------------------------- |
| `AIM_TOKEN_EXPIRY` | JWT token lifetime in seconds (default: 3600) |

---

## Data Migration

### Vault Data

Vault files created by any v0.x release are fully compatible with v1.0.0.
No data migration is required. The on-disk format has not changed.

### Database (SQLite)

If using the `sqlite` feature, the database schema is unchanged from v0.5.0.
No migration SQL is needed.

### Verification

After upgrading, verify your vault is accessible:

```bash
aim list
aim versions my-model   # per-version detail
aim analyze my-model    # format, size, tensor summary
aim --version
```

---

## Support

- **Issues**: https://github.com/nervosys/AIModelVault/issues
- **Discussions**: https://github.com/nervosys/AIModelVault/discussions
- **Security**: See [SECURITY.md](https://github.com/nervosys/AIModelVault/blob/master/SECURITY.md)
