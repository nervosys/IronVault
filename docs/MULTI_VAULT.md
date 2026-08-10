# Multi-Vault Management

Registry for managing multiple vaults with activate/deactivate switching. Work with multiple encrypted vaults from a single CLI.

## Quick Start

```bash
# Register vaults
iv vaults register production /data/vaults/prod --description "Production models"
iv vaults register staging /data/vaults/staging --description "Staging models"

# List registered vaults
iv vaults list

# Switch active vault
iv vaults activate production

# Clear active vault
iv vaults deactivate

# Remove a vault from registry
iv vaults unregister staging
```

## CLI Reference

```
iv vaults <COMMAND>

Commands:
  register    Register a vault
  unregister  Unregister a vault
  activate    Set the active vault
  deactivate  Clear the active vault
  list        List registered vaults
```

### `iv vaults register`

```
iv vaults register <NAME> <PATH> [OPTIONS]

Arguments:
  <NAME>              Vault name/alias
  <PATH>              Path to vault directory

Options:
  -d, --description <DESC>    Description
```

### `iv vaults activate`

```
iv vaults activate <NAME>

Arguments:
  <NAME>    Vault name to activate
```

## Vault Registry

The vault registry is stored in the XDG config directory as `vault_registry.json`. It tracks:

- **Name** — Unique alias for the vault
- **Path** — Filesystem path to the vault directory
- **Description** — Optional human-readable description
- **Active** — Which vault is currently active

Only one vault can be active at a time. The active vault is used by default for all `iv` commands unless overridden by `IRONVAULT_VAULT`.

## Python API

```python
from ironvault import VaultRegistry

registry = VaultRegistry("/path/to/config")

# Register vaults
registry.register("production", "/data/vaults/prod", "Production models")
registry.register("staging", "/data/vaults/staging", "Staging models")

# List vaults
vaults = registry.list()

# Switch active vault
registry.activate("production")
active = registry.active_name()  # "production"

# Deactivate
registry.deactivate()

# Count and cleanup
count = registry.count()
registry.unregister("staging")
```

## REST API

| Method | Path                            | Description            |
| ------ | ------------------------------- | ---------------------- |
| `GET`  | `/api/v1/vaults`                | List registered vaults |
| `POST` | `/api/v1/vaults`                | Register a new vault   |
| `POST` | `/api/v1/vaults/:name/activate` | Activate a vault       |

### Example: Register Vault

```bash
curl -X POST http://localhost:8080/api/v1/vaults \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "production",
    "path": "/data/vaults/prod",
    "description": "Production models"
  }'
```

### Example: Activate Vault

```bash
curl -X POST http://localhost:8080/api/v1/vaults/production/activate \
  -H "Authorization: Bearer $TOKEN"
```

## Library API

```rust
use ironvault::{VaultRegistry, VaultEntry};

let registry = VaultRegistry::new("/path/to/config")?;

// Register
registry.register("production", "/data/vaults/prod".into(), Some("Production models".into()))?;

// List
let vaults: Vec<VaultSummary> = registry.list()?;

// Activate/deactivate
registry.activate("production")?;
let active = registry.active_name()?; // Some("production")
registry.deactivate()?;
```
