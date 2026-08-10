# Garbage Collection

Reclaims disk by pruning orphaned encrypted blobs and stale temp files left behind by interrupted operations.

## CLI

```bash
iv gc            # delete orphans
iv gc --dry-run  # preview only
```

## MCP tool

`gc_run` — `{ "dry_run": false }`

## REST

`POST /api/v1/gc` with `{ "dry_run": bool }`.

## What it removes

- Blobs in `vault/blobs/` that no version manifest references.
- Files in `vault/tmp/` older than the safety window.

Encrypted user content is never touched without a confirmed missing reference. See [src/gc.rs](https://github.com/nervosys/IronVault/blob/master/src/gc.rs).
