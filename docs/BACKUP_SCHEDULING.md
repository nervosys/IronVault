# Backup Scheduling

Configurable vault backup schedules with rotation limits and history tracking. Automate vault backups on hourly, daily, weekly, or monthly intervals.

## Quick Start

```bash
# Create a daily backup schedule
iv backup set nightly --frequency daily --max-backups 7 --output-dir /backups/vault

# List schedules
iv backup list

# View backup history
iv backup history

# Remove a schedule
iv backup remove nightly
```

## CLI Reference

```
iv backup <COMMAND>

Commands:
  set      Create or update a backup schedule
  remove   Remove a backup schedule
  list     List backup schedules
  history  Show backup history
```

### `iv backup set`

```
iv backup set <NAME> --frequency <FREQ> --output-dir <PATH> [OPTIONS]

Arguments:
  <NAME>              Schedule name

Options:
  -f, --frequency <FREQ>       Frequency: hourly, daily, weekly, monthly
  -m, --max-backups <N>        Maximum backups to retain (default: 7)
  -o, --output-dir <PATH>      Output directory for backup archives
```

### `iv backup history`

```
iv backup history [OPTIONS]

Options:
  -s, --schedule <NAME>    Filter by schedule name
```

## Backup Frequencies

| Frequency | Aliases | Description |
| --------- | ------- | ----------- |
| `hourly`  | `1h`    | Every hour  |
| `daily`   | `1d`    | Every day   |
| `weekly`  | `1w`    | Every week  |
| `monthly` | `1m`    | Every month |

## Rotation

When max-backups is reached, the oldest backup is removed before creating a new one. This keeps disk usage bounded while maintaining recent backup availability.

## Python API

```python
from ironvault import BackupManager

manager = BackupManager("/path/to/vault")

# Create schedule
manager.set_schedule("nightly", "daily", 7, "/backups/vault")

# List schedules
schedules = manager.list_schedules()

# Remove schedule
manager.remove_schedule("nightly")

# Check backup count
count = manager.backup_count()
```

## REST API

| Method | Path                        | Description                           |
| ------ | --------------------------- | ------------------------------------- |
| `GET`  | `/api/v1/backups/schedules` | List all backup schedules             |
| `POST` | `/api/v1/backups/schedules` | Create/update a backup schedule       |
| `GET`  | `/api/v1/backups/history`   | Show backup history (query: schedule) |

### Example: Create Schedule

```bash
curl -X POST http://localhost:8080/api/v1/backups/schedules \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "nightly",
    "frequency": "daily",
    "max_backups": 7,
    "output_dir": "/backups/vault"
  }'
```

## Library API

```rust
use ironvault::{BackupManager, BackupFrequency};

let manager = BackupManager::new("/path/to/vault")?;

// Set up a schedule
manager.set_schedule("nightly", BackupFrequency::Daily, 7, "/backups/vault".into())?;

// List schedules
let schedules = manager.list_schedules()?;

// Check history
let history = manager.get_history(Some("nightly"))?;
```
