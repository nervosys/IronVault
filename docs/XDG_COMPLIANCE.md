# XDG Base Directory Specification Compliance

IronVault is **fully compliant** with the [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html), ensuring proper organization of configuration files, application data, and cache across all platforms.

## Table of Contents

- [What is XDG?](#what-is-xdg)
- [XDG Directory Structure](#xdg-directory-structure)
- [Platform Support](#platform-support)
- [Environment Variables](#environment-variables)
- [Directory Organization](#directory-organization)
- [Security & Permissions](#security--permissions)
- [Compliance Checklist](#compliance-checklist)
- [Examples](#examples)

## What is XDG?

The **XDG Base Directory Specification** defines standard locations for storing:
- **Configuration files** - User preferences and settings
- **Application data** - User-specific data files (models, databases)
- **Cache files** - Non-essential cached data
- **State files** - Logs, history, and runtime state

### Benefits

✓ **User isolation** - No conflicts between different users  
✓ **Backup friendly** - Easy to backup config separately from cache  
✓ **Portable** - Works across Linux, macOS, Windows  
✓ **Configurable** - Override via environment variables  
✓ **Standards compliant** - Follows FHS (Filesystem Hierarchy Standard)  

## XDG Directory Structure

IronVault uses the following XDG-compliant directories:

### Linux

```
~/.config/ironvault/           # XDG_CONFIG_HOME
  └── config.yaml                 # Vault configuration

~/.local/share/ironvault/      # XDG_DATA_HOME
  ├── vaults/
  │   └── default/
  │       ├── models/             # Encrypted model files
  │       └── metadata/           # Model metadata
  └── logs/
      └── audit.log               # Security audit log

~/.cache/ironvault/            # XDG_CACHE_HOME
  ├── decompressed/               # LRU cache for models
  └── temp/                       # Temporary files
```

### macOS

```
~/Library/Application Support/ai.nervosys.ironvault/
  ├── config.yaml
  ├── vaults/
  └── logs/

~/Library/Caches/ai.nervosys.ironvault/
  └── decompressed/
```

### Windows

```
%APPDATA%\nervosys\ironvault\config\
  └── config.yaml

%APPDATA%\nervosys\ironvault\data\
  ├── vaults\
  └── logs\

%LOCALAPPDATA%\nervosys\ironvault\cache\
  └── decompressed\
```

## Platform Support

IronVault provides **native XDG-style directory organization** on all platforms:

| Platform    | Config Location                  | Data Location                    | Cache Location              |
| ----------- | -------------------------------- | -------------------------------- | --------------------------- |
| **Linux**   | `~/.config/`                     | `~/.local/share/`                | `~/.cache/`                 |
| **macOS**   | `~/Library/Application Support/` | `~/Library/Application Support/` | `~/Library/Caches/`         |
| **Windows** | `%APPDATA%\...\config\`          | `%APPDATA%\...\data\`            | `%LOCALAPPDATA%\...\cache\` |

While Windows doesn't natively support XDG, we maintain the same **separation of concerns**:
- APPDATA for persistent config and data
- LOCALAPPDATA for cache (can be cleared)

## Environment Variables

IronVault respects XDG environment variables:

### XDG_CONFIG_HOME

**Purpose**: Base directory for user-specific configuration files  
**Default**: `~/.config` (Linux), `~/Library/Application Support` (macOS)  
**Contains**: `config.yaml`, user preferences  

```bash
export XDG_CONFIG_HOME=/custom/config
# IronVault will use: /custom/config/ironvault/
```

### XDG_DATA_HOME

**Purpose**: Base directory for user-specific data files  
**Default**: `~/.local/share` (Linux), `~/Library/Application Support` (macOS)  
**Contains**: Encrypted models, metadata, version history  

```bash
export XDG_DATA_HOME=/custom/data
# IronVault will use: /custom/data/ironvault/
```

### XDG_CACHE_HOME

**Purpose**: Base directory for user-specific non-essential cached data  
**Default**: `~/.cache` (Linux), `~/Library/Caches` (macOS)  
**Contains**: Decompressed models, LRU cache, temporary files  

```bash
export XDG_CACHE_HOME=/custom/cache
# IronVault will use: /custom/cache/ironvault/
```

### XDG_STATE_HOME

**Purpose**: Base directory for user-specific state data  
**Default**: `~/.local/state` (Linux)  
**Contains**: Logs, history, runtime state  

```bash
export XDG_STATE_HOME=/custom/state
# Future: May be used for operation logs
```

## Directory Organization

### Configuration Directory (`XDG_CONFIG_HOME`)

```
~/.config/ironvault/
├── config.yaml              # Main configuration file
└── preferences.yaml         # User preferences (future)
```

**What goes here:**
- Vault settings (compression, encryption)
- User preferences
- CLI defaults
- No model data (config only!)

### Data Directory (`XDG_DATA_HOME`)

```
~/.local/share/ironvault/
├── vaults/
│   ├── default/             # Default vault
│   │   ├── models/
│   │   │   ├── model_id_v1.bin.enc
│   │   │   ├── model_id_v2.bin.enc
│   │   │   └── model_id_v3.bin.enc
│   │   ├── metadata/
│   │   │   ├── model_id_v1.json
│   │   │   ├── model_id_v2.json
│   │   │   └── model_id_v3.json
│   │   └── index.db         # Model index
│   └── production/          # Production vault (example)
└── logs/
    ├── audit.log            # FIPS 140-3 audit trail
    └── operations.log       # Operation history
```

**What goes here:**
- Encrypted model files
- Model metadata (version, lineage, tags)
- Audit logs (required for compliance)
- Database indexes

### Cache Directory (`XDG_CACHE_HOME`)

```
~/.cache/ironvault/
├── decompressed/            # LRU cache of decompressed models
│   ├── model_id_v1.bin
│   └── model_id_v2.bin
├── temp/                    # Temporary workspace
│   └── upload_*.tmp
└── lru_cache.db             # Cache metadata
```

**What goes here:**
- Decompressed models (LRU eviction)
- Temporary files during operations
- Can be safely deleted anytime
- Automatically recreated if missing

## Security & Permissions

IronVault enforces **secure permissions** on all directories:

### Unix/Linux/macOS

All directories created with `0700` permissions (owner-only access):

```bash
drwx------ user user ~/.config/ironvault/
drwx------ user user ~/.local/share/ironvault/
drwx------ user user ~/.cache/ironvault/
```

This ensures:
- ✓ Only the owner can read/write/execute
- ✓ Other users cannot access vault data
- ✓ Compliance with security standards (FIPS 140-3, CMMC)

### Windows

Uses platform-appropriate ACLs:
- User has full control
- System has full control
- Other users: No access

## Compliance Checklist

IronVault is **100% XDG compliant**:

- ✅ **Configuration in XDG_CONFIG_HOME** - All config in `~/.config/ironvault/`
- ✅ **Data in XDG_DATA_HOME** - All models in `~/.local/share/ironvault/`
- ✅ **Cache in XDG_CACHE_HOME** - All cache in `~/.cache/ironvault/`
- ✅ **Respects environment variables** - Honors XDG_* overrides
- ✅ **Falls back to defaults** - Works without XDG_* set
- ✅ **No hardcoded paths** - All paths computed at runtime
- ✅ **Cross-platform** - Works on Linux, macOS, Windows
- ✅ **Secure permissions** - Owner-only (0700 on Unix)
- ✅ **Proper separation** - Config ≠ Data ≠ Cache
- ✅ **Standard compliant** - Follows FHS and XDG specs

## Examples

### Basic Usage (Default XDG Paths)

```rust
use ironvault::{VaultConfig, ModelVault};

// Uses XDG directories automatically
let config = VaultConfig::new()?;
let vault = ModelVault::new(&config.dirs.vault_dir)?;

println!("Config: {}", config.dirs.config_dir.display());
println!("Data:   {}", config.dirs.data_dir.display());
println!("Cache:  {}", config.dirs.cache_dir.display());
```

### Custom XDG Paths

```bash
# Override XDG directories
export XDG_CONFIG_HOME=/mnt/config
export XDG_DATA_HOME=/mnt/data
export XDG_CACHE_HOME=/tmp/cache

# IronVault will automatically use custom paths
cargo run --example xdg_demo
```

### Testing with Temporary Directories

```rust
use std::path::PathBuf;
use ironvault::{VaultConfig, DirectoryPaths};

// Create test directories
let temp_dir = std::env::temp_dir().join("test_vault");
let dirs = DirectoryPaths {
    config_dir: temp_dir.join("config"),
    data_dir: temp_dir.join("data"),
    cache_dir: temp_dir.join("cache"),
    vault_dir: temp_dir.join("data/vaults"),
    log_dir: temp_dir.join("data/logs"),
};

let config = VaultConfig::with_dirs(dirs)?;
// Now vault uses temporary directories
```

### Multi-Environment Setup

```bash
# Development
export XDG_DATA_HOME=$HOME/dev/IRONVAULT_DEV

# Staging
export XDG_DATA_HOME=$HOME/staging/IRONVAULT_STAGING

# Production
export XDG_DATA_HOME=/mnt/production/ironvault

# Each environment has isolated vault data
```

### Backup Strategy

XDG separation makes backups easy:

```bash
# Backup configuration only (small, fast)
tar czf config_backup.tar.gz ~/.config/ironvault/

# Backup data and models (large, infrequent)
tar czf data_backup.tar.gz ~/.local/share/ironvault/

# Skip cache (not needed in backups)
# ~/.cache/ironvault/ - can be regenerated
```

### Network Storage

```bash
# Store models on network drive, config locally
export XDG_DATA_HOME=/mnt/nfs/shared
# XDG_CONFIG_HOME defaults to ~/.config/ (local)

# Now models are shared, config is per-user
```

## Run the Demo

See XDG compliance in action:

```bash
# Build and run XDG demo
cargo run --example xdg_demo

# Shows:
# - Current XDG environment variables
# - Computed directory paths
# - Directory creation and permissions
# - File organization
# - Platform-specific behavior
# - Compliance checklist
```

## References

- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html)
- [Filesystem Hierarchy Standard (FHS)](https://refspecs.linuxfoundation.org/FHS_3.0/fhs/index.html)
- [directories crate](https://docs.rs/directories/) - Used for XDG support
- [macOS File System Programming Guide](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/)
- [Windows Known Folders](https://docs.microsoft.com/en-us/windows/win32/shell/known-folders)

## FAQ

### Q: Why use XDG on Windows?

**A:** While Windows doesn't natively support XDG, the **principles** are universal:
- Separate config from data
- Separate persistent data from cache
- User-specific directories

We map XDG concepts to Windows equivalents (APPDATA/LOCALAPPDATA).

### Q: Can I change the directories after creation?

**A:** Yes, via environment variables:

```bash
export XDG_DATA_HOME=/new/location
# Restart IronVault - it will use new location
```

Note: Existing models won't be automatically moved. To migrate the whole vault,
use `iv vault-export archive.tar.gz` against the old location and
`iv vault-import archive.tar.gz` against the new one. For individual models,
`iv export <name> <dir>` then `iv store` into the new vault. (There is no
`iv import` — it takes the `vault-` prefix.)

### Q: What happens if I delete the cache directory?

**A:** Safe to delete! The cache is **non-essential**:
- Will be automatically recreated
- Models are still in `XDG_DATA_HOME` (encrypted)
- May be slower temporarily (no LRU cache)

### Q: How do I find my XDG directories?

**A:** Run the demo:

```bash
cargo run --example xdg_demo
```

Or check manually:

```bash
# Linux
echo $XDG_CONFIG_HOME  # Falls back to ~/.config
echo $XDG_DATA_HOME    # Falls back to ~/.local/share
echo $XDG_CACHE_HOME   # Falls back to ~/.cache
```

### Q: Does this work in containers?

**A:** Yes. XDG directories work in containers, though there is no first-party
image — the `Dockerfile` and Helm chart were removed in 4.5.0, so this applies
to an image you build yourself:

```dockerfile
# Set custom XDG paths for container
ENV XDG_CONFIG_HOME=/app/config
ENV XDG_DATA_HOME=/app/data
ENV XDG_CACHE_HOME=/tmp/cache

# Mount volumes
VOLUME ["/app/config", "/app/data"]
```

### Q: Is this required for FIPS 140-3 compliance?

**A:** While not strictly required, proper directory separation is a **security best practice**:
- Separate config (less sensitive) from data (highly sensitive)
- Clear audit trail in logs directory
- Cache can be on faster storage (temporary data)

This aligns with CMMC AC.3.014 (Separation of Duties).

## Troubleshooting

### Permission Denied Errors

```bash
# Check directory permissions
ls -la ~/.config/ironvault/
ls -la ~/.local/share/ironvault/

# Should show: drwx------ (0700)
# If not, fix with:
chmod 700 ~/.config/ironvault/
chmod 700 ~/.local/share/ironvault/
```

### Directory Not Found

```bash
# Ensure directories exist
mkdir -p ~/.config/ironvault
mkdir -p ~/.local/share/ironvault
mkdir -p ~/.cache/ironvault

# Or let IronVault create them:
cargo run --example xdg_demo
```

### Custom XDG Paths Not Working

```bash
# Verify environment variables are set
env | grep XDG_

# Set them in your shell profile (~/.bashrc, ~/.zshrc)
export XDG_CONFIG_HOME=/custom/config
export XDG_DATA_HOME=/custom/data
export XDG_CACHE_HOME=/custom/cache

# Reload
source ~/.bashrc
```

## Next Steps

- Read [ARCHITECTURE.md](ARCHITECTURE.md) for system design
- See [SECURITY.md](https://github.com/nervosys/IronVault/blob/master/SECURITY.md) for security practices
- Check [QUICKSTART.md](QUICKSTART.md) for usage guide
- Run `cargo run --example xdg_demo` to see it in action
