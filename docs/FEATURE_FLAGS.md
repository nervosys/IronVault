# Feature Flags

IronVault uses Cargo feature flags to keep the default binary lean while enabling optional capabilities.

## Default Features

Enabled by default with `cargo build`:

| Feature       | Crate(s)    | Purpose                                 |
| ------------- | ----------- | --------------------------------------- |
| `safetensors` | safetensors | SafeTensors format read/write           |
| `ndarray`     | ndarray     | N-dimensional array support for tensors |
| `sqlite`      | rusqlite    | SQLite backend for RAG and versioning   |

## Optional Features

### Database Backends

| Feature     | Crate(s)      | Purpose                                | Build Command                      |
| ----------- | ------------- | -------------------------------------- | ---------------------------------- |
| `kv-store`  | sled          | Sled embedded key-value store for RAG  | `cargo build --features kv-store`  |
| `vector-db` | qdrant-client | Qdrant vector database for RAG         | `cargo build --features vector-db` |
| `database`  | _(meta)_      | Enables `sqlite` + `kv-store` together | `cargo build --features database`  |

### Cloud Storage

| Feature | Crate(s)                           | Purpose            | Build Command                  |
| ------- | ---------------------------------- | ------------------ | ------------------------------ |
| `s3`    | aws-config, aws-sdk-s3             | AWS S3 backend     | `cargo build --features s3`    |
| `azure` | azure_storage, azure_storage_blobs | Azure Blob backend | `cargo build --features azure` |
| `cloud` | _(meta)_                           | All cloud backends | `cargo build --features cloud` |

> **Note:** GCS support (`gcs` / `cloud-storage` crate) is disabled due to security vulnerabilities in upstream dependencies.

### API Server

| Feature   | Crate(s)                                             | Purpose                         | Build Command                    |
| --------- | ---------------------------------------------------- | ------------------------------- | -------------------------------- |
| `api`     | axum, tower, tower-http, jsonwebtoken, utoipa, hyper | REST API with JWT auth          | `cargo build --features api`     |
| `graphql` | async-graphql, async-graphql-axum (+ `api`)          | GraphQL endpoint (requires api) | `cargo build --features graphql` |

### Bindings

| Feature  | Crate(s) | Purpose                | Build Command                       |
| -------- | -------- | ---------------------- | ----------------------------------- |
| `python` | pyo3     | Python native bindings | `maturin develop --features python` |

## Meta Features

| Feature   | Includes                                      | Purpose                 |
| --------- | --------------------------------------------- | ----------------------- |
| `default` | safetensors, ndarray, sqlite                  | Minimal working set     |
| `full`    | safetensors, ndarray, sqlite, sled, vector-db | All non-system features |

> `full` excludes `python` (requires Python), `api`/`graphql` (server features), and cloud backends.

## Common Build Recipes

```bash
# Default (SafeTensors + SQLite)
cargo build --release

# Everything except system-dependent features
cargo build --release --features full

# API server with cloud storage
cargo build --release --features api,cloud

# Full API + GraphQL server
cargo build --release --features graphql,cloud

# All features (except Python — needs maturin)
cargo build --release --features full,api,graphql,cloud

# Python bindings
pip install maturin
maturin develop --features python
```

## Environment Variables for Feature Behavior

| Variable                | Feature  | Purpose                        |
| ----------------------- | -------- | ------------------------------ |
| `IRONVAULT_SQLITE_VERSIONS`   | `sqlite` | Use SQLite for version storage |
| `AWS_ACCESS_KEY_ID`     | `s3`     | AWS credentials                |
| `AWS_SECRET_ACCESS_KEY` | `s3`     | AWS credentials                |
| `AWS_REGION`            | `s3`     | AWS region                     |
| `AZURE_STORAGE_ACCOUNT` | `azure`  | Azure credentials              |
| `AZURE_STORAGE_SAS_TOKEN` | `azure` | Azure SAS (or Entra ID vars)  |
