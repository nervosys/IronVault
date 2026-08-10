# Vault Export / Import

Portable `tar.gz` bundles of an entire vault — encrypted blobs, manifests, ACLs, policies, tags. Round-trips losslessly.

## CLI

```bash
iv vault-export ./my-vault-2025-01-01.tar.gz
iv vault-import ./my-vault-2025-01-01.tar.gz /path/to/target
```

## MCP tools

`vault_export` — `{ "output": "path.tar.gz" }`
`vault_import` — `{ "archive": "path.tar.gz", "target": "/path" }`

## REST

`POST /api/v1/vault/export`, `POST /api/v1/vault/import`.

## What's included

- All encrypted blobs (still encrypted with the vault key).
- Version manifests, model cards, lineage, tags, ACLs.
- Vault config (without the master passphrase).

The passphrase is **never** included — the recipient supplies it at first open. See [src/vault_bundle.rs](https://github.com/nervosys/IronVault/blob/master/src/vault_bundle.rs).
