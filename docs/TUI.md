# TUI Dashboard

A terminal UI built on `ratatui` for browsing vault contents without leaving the shell.

## Launch

```bash
iv browse
```

## Panels

- **Models** — name, latest version, size, last touch.
- **Versions** — per-selection lineage and metadata.
- **Tags & annotations** — read-only view of `iv tag` data.
- **Stats** — vault total size and dedup ratio.

## Key bindings

| Key       | Action                  |
| --------- | ----------------------- |
| `↑/↓`     | Move selection          |
| `Enter`   | Open detail panel       |
| `q`/`Esc` | Quit                    |
| `/`       | Filter by name fragment |

See [src/tui.rs](https://github.com/nervosys/IronVault/blob/master/src/tui.rs).
