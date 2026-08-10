# ironvault Architecture

## System Architecture Diagram

```mermaid
flowchart TB
    subgraph User["User Interface"]
        CLI[CLI Application]
        RESTAPI[REST API / Axum]
        GraphQL[GraphQL]
        PyBindings[Python Bindings / PyO3]
    end
    
    subgraph Vault["Vault Core"]
        VaultLogic[Vault Logic + VaultBuilder]
        Config[Configuration / XDG]
        VersionCtrl[Version Control]
        SQLiteVer[SQLite Version Backend]
    end
    
    subgraph Security["Security Layer"]
        Crypto[FIPS Crypto Module]
        Streaming[Streaming Encryption]
        Audit[Audit Logger]
        Compliance[Compliance Checker]
        Blockchain[Blockchain Audit Trail]
    end
    
    subgraph Storage["Storage Backend"]
        EncStorage[Encrypted Storage]
        Compression[Compression]
        FileSystem[File System]
        Cloud[Cloud Storage / S3, Azure, GCS]
    end
    
    subgraph Intelligence["AI/ML Features"]
        Formats[23+ Model Formats]
        Conversion[Format Conversion Pipeline]
        ModelCards[Model Cards]
        RAG[RAG / MCP Tools]
    end
    
    subgraph Network["Distributed"]
        Federation[Federated Vault Sync]
        Telemetry[Telemetry / opt-in]
    end
    
    CLI --> VaultLogic
    RESTAPI --> VaultLogic
    GraphQL --> VaultLogic
    PyBindings --> VaultLogic
    
    VaultLogic --> Config
    VaultLogic --> VersionCtrl
    VaultLogic --> SQLiteVer
    VaultLogic --> Crypto
    VaultLogic --> Audit
    VaultLogic --> Blockchain
    
    Crypto --> EncStorage
    Crypto --> Streaming
    EncStorage --> Compression
    Compression --> FileSystem
    Compression --> Cloud
    
    VaultLogic --> Formats
    VaultLogic --> Conversion
    VaultLogic --> ModelCards
    VaultLogic --> RAG
    VaultLogic --> Federation
    Compliance --> Audit
    
    style Crypto fill:#f9f,stroke:#333,stroke-width:2px
    style Audit fill:#ff9,stroke:#333,stroke-width:2px
    style VaultLogic fill:#9ff,stroke:#333,stroke-width:2px
    style RESTAPI fill:#9f9,stroke:#333,stroke-width:2px
    style Conversion fill:#f96,stroke:#333,stroke-width:2px
```

## Data Flow - Store Model

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant Vault
    participant Crypto
    participant Storage
    participant FS as File System
    
    User->>CLI: iv store model.pt
    CLI->>Vault: store_model()
    
    Note over Vault: Validate input
    Vault->>Crypto: derive_key(passphrase)
    Crypto-->>Vault: encryption_key
    
    Note over Vault: Read model data
    Vault->>Crypto: compress(data)
    Crypto-->>Vault: compressed_data
    
    Vault->>Crypto: encrypt(compressed_data, key)
    Crypto-->>Vault: encrypted_data
    
    Note over Vault: Generate checksum
    Vault->>Storage: save_encrypted_file()
    Storage->>FS: write file
    
    Note over Vault: Update version control
    Vault->>Storage: update_versions_json()
    Storage->>FS: write metadata
    
    Note over Vault: Log audit event
    Vault->>CLI: return version info
    CLI->>User: ✓ Model stored
```

## Data Flow - Retrieve Model

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant Vault
    participant Crypto
    participant Storage
    participant FS as File System
    
    User->>CLI: iv get model
    CLI->>Vault: get_model()
    
    Note over Vault: Verify authentication
    Vault->>Crypto: derive_key(passphrase)
    Crypto-->>Vault: encryption_key
    
    Vault->>Storage: get_version_metadata()
    Storage->>FS: read versions.json
    FS-->>Storage: metadata
    Storage-->>Vault: version_info
    
    Vault->>Storage: read_encrypted_file()
    Storage->>FS: read file
    FS-->>Storage: encrypted_data
    Storage-->>Vault: encrypted_data
    
    Vault->>Crypto: decrypt(encrypted_data, key)
    Crypto-->>Vault: compressed_data
    
    Vault->>Crypto: decompress(compressed_data)
    Crypto-->>Vault: original_data
    
    Note over Vault: Verify checksum
    Note over Vault: Log audit event
    
    Vault->>CLI: return model_data
    CLI->>User: ✓ Model retrieved
```

## Cryptographic Architecture

```mermaid
flowchart LR
    subgraph Input
        Pass[Passphrase]
        Data[Model Data]
    end
    
    subgraph KDF["Key Derivation"]
        Argon2[Argon2id<br/>64MB, 3 iterations]
        Salt[Random Salt<br/>32 bytes]
    end
    
    subgraph Encryption
        AES[AES-256-GCM]
        Nonce[Random Nonce<br/>12 bytes]
        Tag[Auth Tag<br/>16 bytes]
    end
    
    subgraph Output
        Encrypted[Encrypted Data]
        Metadata[Metadata + Checksum]
    end
    
    Pass --> Argon2
    Salt --> Argon2
    Argon2 --> Key[256-bit Key]
    
    Data --> Compress[Compression]
    Compress --> AES
    Key --> AES
    Nonce --> AES
    
    AES --> Encrypted
    AES --> Tag
    Tag --> Metadata
    
    style Argon2 fill:#f96,stroke:#333,stroke-width:2px
    style AES fill:#96f,stroke:#333,stroke-width:2px
    style Key fill:#ff9,stroke:#333,stroke-width:2px
```

## Compliance Framework

```mermaid
mindmap
  root((ironvault<br/>Compliance))
    FIPS_140_3
      AES-256-GCM
      Argon2id
      SHA-256
      Approved RNG
    CMMC_2_0
      Level 2
        AC: Access Control
        AU: Audit
        IA: Authentication
        SC: Crypto Protection
    MITRE_ATTACK
      T1552: Credentials
      T1486: Encryption
      T1078: Accounts
      T1005: Local Data
    CVE
      cargo-audit
      Dependency Scanning
      Vulnerability Tracking
```

## Version Control Structure

```mermaid
gitGraph
    commit id: "v1: Initial model"
    commit id: "v2: Fine-tuned"
    branch experiment
    commit id: "v3: Experimental arch"
    checkout main
    commit id: "v4: Production update"
    checkout experiment
    commit id: "v5: Advanced features"
    checkout main
    merge experiment tag: "v6: Merged"
    commit id: "v7: Optimized"
```

## Directory Structure (XDG Compliant)

```mermaid
graph TD
    Home[~/ Home Directory]
    
    Home --> Config[.config/ironvault/]
    Home --> Data[.local/share/ironvault/]
    Home --> Cache[.cache/ironvault/]
    
    Config --> ConfigYAML[config.yaml]
    
    Data --> Vaults[vaults/]
    Data --> Logs[logs/]
    
    Vaults --> Default[default/]
    Default --> Models[models/]
    Default --> Versions[versions.json]
    
    Models --> Model1[model_name/]
    Model1 --> V1[v1_timestamp.nvault]
    Model1 --> V2[v2_timestamp.nvault]
    
    Logs --> Audit[audit.log]
    
    style ConfigYAML fill:#9f9
    style V1 fill:#f99
    style V2 fill:#f99
    style Audit fill:#ff9
```

## Security Layers

```mermaid
flowchart TD
    subgraph L1["Layer 1: Application"]
        CLI[CLI Interface]
        API[Public API]
    end
    
    subgraph L2["Layer 2: Validation"]
        Input[Input Validation]
        Auth[Authentication]
    end
    
    subgraph L3["Layer 3: Business Logic"]
        Vault[Vault Operations]
        Version[Version Control]
    end
    
    subgraph L4["Layer 4: Security"]
        Encrypt[Encryption]
        Audit[Audit Logging]
    end
    
    subgraph L5["Layer 5: Storage"]
        FS[File System]
        Perms[Permissions 700/600]
    end
    
    CLI --> Input
    API --> Input
    Input --> Auth
    Auth --> Vault
    Vault --> Version
    Version --> Encrypt
    Encrypt --> Audit
    Audit --> FS
    FS --> Perms
    
    style Auth fill:#f96,stroke:#333,stroke-width:2px
    style Encrypt fill:#f96,stroke:#333,stroke-width:2px
    style Audit fill:#ff9,stroke:#333,stroke-width:2px
```

## REST & GraphQL API

The `api` feature provides 20 Axum HTTP endpoints under `/api/v1/` plus a
GraphQL playground (with the `graphql` feature).

### RBAC (Role-Based Access Control)

JWT tokens carry a `role` claim — one of **Admin**, **Operator**, or **Viewer**.
Role enforcement is applied at the route handler level:

| Role         | Capabilities                                                 |
| ------------ | ------------------------------------------------------------ |
| **Admin**    | Full access — all endpoints including audit log and events   |
| **Operator** | Store, retrieve, convert, delete models; view own audit data |
| **Viewer**   | Read-only — list models, stats, compliance, model cards      |

The `/api/v1/audit` and `/api/v1/events` endpoints filter security-sensitive
entries to Admin-only access.

### Endpoint Map

| Method | Path                               | Auth | Description               |
| ------ | ---------------------------------- | ---- | ------------------------- |
| GET    | `/api/v1/health`                   | No   | Health check              |
| POST   | `/api/v1/auth/token`               | No   | Issue JWT token           |
| GET    | `/api/v1/models`                   | Yes  | List all models           |
| GET    | `/api/v1/models/:name`             | Yes  | Get model details         |
| POST   | `/api/v1/models/:name`             | Yes  | Store model               |
| GET    | `/api/v1/models/:name/card`        | Yes  | Get model card            |
| POST   | `/api/v1/models/:name/card`        | Yes  | Create model card         |
| GET    | `/api/v1/models/:name/versions`    | Yes  | List versions             |
| GET    | `/api/v1/models/:name/versions/:v` | Yes  | Get specific version      |
| DELETE | `/api/v1/models/:name/versions/:v` | Yes  | Delete version            |
| GET    | `/api/v1/models/:name/lineage/:v`  | Yes  | Version lineage tree      |
| GET    | `/api/v1/conversions`              | No   | List conversion paths     |
| POST   | `/api/v1/convert`                  | Yes  | Convert model format      |
| GET    | `/api/v1/compliance`               | Yes  | Run compliance checks     |
| POST   | `/api/v1/rag/search`               | Yes  | RAG vector search         |
| POST   | `/api/v1/rag/documents`            | Yes  | Add RAG document          |
| GET    | `/api/v1/stats`                    | Yes  | Vault statistics          |
| GET    | `/api/v1/audit`                    | Yes  | Audit log (Admin-full)    |
| GET    | `/api/v1/metrics`                  | Yes  | Prometheus-style metrics  |
| GET    | `/api/v1/events`                   | Yes  | Event stream (Admin-full) |
| GET    | `/api/v1/openapi.json`             | No   | OpenAPI 3.1 spec          |

### Rate Limiting

The `/api/v1/auth/token` endpoint is protected by a per-IP sliding-window
rate limiter (default: 5 attempts per 60 seconds) to prevent brute-force attacks.

## Error Type Hierarchy

Domain-specific errors carry rich context and convert into the top-level
`VaultError` via `From` impls.

```mermaid
graph TD
    VaultError["VaultError (top-level)"]
    CryptoError --> VaultError
    StorageError --> VaultError
    ConversionError --> VaultError

    CryptoError --> CE1[KeyDerivation]
    CryptoError --> CE2[Encryption]
    CryptoError --> CE3[Decryption]
    CryptoError --> CE4[Integrity]

    StorageError --> SE1[Io]
    StorageError --> SE2[Serialization]
    StorageError --> SE3[Compression]
    StorageError --> SE4[Database]

    ConversionError --> XE1[Unsupported]
    ConversionError --> XE2[Validation]

    style VaultError fill:#9ff,stroke:#333,stroke-width:2px
    style CryptoError fill:#f9f,stroke:#333,stroke-width:2px
    style StorageError fill:#ff9,stroke:#333,stroke-width:2px
    style ConversionError fill:#f96,stroke:#333,stroke-width:2px
```

## Module Dependencies

```mermaid
graph LR
    Main[main.rs] --> Vault
    Main --> CLI[cli/ handlers]
    
    Vault[vault.rs] --> Config[config.rs]
    Vault --> Storage[storage.rs]
    Vault --> Version[version.rs]
    Vault --> SQLite[version_sqlite.rs]
    Vault --> Crypto
    Vault --> Audit[audit.rs]
    Vault --> Traits[traits.rs]
    
    Storage --> Crypto[crypto/mod.rs]
    Storage --> FS[File System]
    
    Crypto --> AES[aes-gcm]
    Crypto --> Argon[argon2]
    Crypto --> Compress[compression.rs]
    Crypto --> Stream[streaming.rs]
    
    Compress --> Gzip[flate2]
    Compress --> LZMA[lzma-rs]
    
    Audit --> Log[tracing]
    Audit --> Blockchain[blockchain.rs]
    
    Config --> XDG[directories]
    
    Conversion[conversion.rs] --> Formats[formats.rs]
    ModelCard[model_card.rs] --> Vault
    Federation[federation.rs] --> Vault
    Telemetry[telemetry.rs] --> Config
    API[api/] --> Vault
    API --> Auth[JWT auth]
    RAG[rag/] --> Storage
    Python[python.rs] --> Vault
    
    style Vault fill:#9ff,stroke:#333,stroke-width:2px
    style Crypto fill:#f9f,stroke:#333,stroke-width:2px
    style API fill:#9f9,stroke:#333,stroke-width:2px
    style Conversion fill:#f96,stroke:#333,stroke-width:2px
```

## Test Coverage

| Metric          | Value |
| --------------- | ----- |
| Library tests   | 584   |
| Python tests    | 62    |
| Line coverage   | 86.1% |
| Modules at 100% | 8     |
| Clippy warnings | 0     |

Coverage is measured with `cargo-llvm-cov` using `--features "full,graphql"`.
