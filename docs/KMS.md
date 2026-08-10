# External KMS Integration

`iv` can fetch the vault passphrase from an external secret manager instead of
prompting for it, so passphrase-gated commands run unattended in CI and from agents.

## Supported sources

| URI                            | Backend              | Resolution                                                      |
| ------------------------------ | -------------------- | --------------------------------------------------------------- |
| `env://NAME`                   | Environment variable | Value of `$NAME`                                                |
| `file:///abs/path`             | Local file           | File contents, trailing newline trimmed                         |
| `aws-sm://secret-name`         | AWS Secrets Manager  | `GetSecretValue`; **requires `--features s3`**                  |
| `azure-kv://vault-name/secret` | Azure Key Vault      | `GET https://{vault}.vault.azure.net/secrets/{secret}`          |
| `vault://mount/path/key`       | HashiCorp Vault      | KV v2 then v1 under `$VAULT_ADDR`; `key` names the secret field |

Any value **without** a recognised `scheme://` prefix is treated as the literal
passphrase, so plain values keep working unchanged.

## Where secrets are accepted

`IRONVAULT_PASSPHRASE` accepts either a literal passphrase or any URI above.
The CLI resolves the passphrase in this order:

1. `$IRONVAULT_PASSPHRASE` — literal value, or a KMS URI to resolve
2. A line piped on stdin, when stdin is not a terminal
3. An interactive masked prompt

```bash
# Literal (simplest; fine for ephemeral CI runners)
IRONVAULT_PASSPHRASE='hunter2' iv list

# From a secret manager
IRONVAULT_PASSPHRASE='aws-sm://prod/iv-passphrase' iv store my-llm ./model.safetensors
IRONVAULT_PASSPHRASE='vault://secret/iv/passphrase' iv get my-llm ./out.safetensors

# From a mounted secret file (Kubernetes, systemd credentials)
IRONVAULT_PASSPHRASE='file:///run/secrets/iv-pass' iv list

# Piped
printf '%s\n' "$PASSPHRASE" | iv list
```

## Backend configuration

| Backend             | Required environment                                                                |
| ------------------- | ------------------------------------------------------------------------------------ |
| AWS Secrets Manager | Standard AWS credential chain + `AWS_REGION`. Binary must be built `--features s3`. |
| Azure Key Vault     | `AZURE_KEYVAULT_TOKEN` (or `AZURE_ACCESS_TOKEN`) — a bearer token for the vault     |
| HashiCorp Vault     | `VAULT_ADDR`, `VAULT_TOKEN`                                                          |

Mint an Azure token with:

```bash
export AZURE_KEYVAULT_TOKEN=$(az account get-access-token \
  --resource https://vault.azure.net --query accessToken -o tsv)
```

## Security notes

- Secrets are held in `Zeroizing<String>` and wiped from memory on drop.
- `file://` sources are rejected on Unix if the file is group- or world-readable
  (`chmod 600` them). Windows relies on filesystem ACLs.
- An unresolvable URI is a hard error — `iv` never silently falls back to an
  empty passphrase.
- A passphrase passed via the environment is visible to other processes on most
  systems; prefer `file://` or a KMS URI on shared hosts.

## Library API

```rust
use ironvault::kms::{self, KmsUri};

// Resolve a value that may be a URI or a literal.
let secret = kms::resolve("vault://secret/iv/passphrase")?;

// Or parse and fetch explicitly.
let uri: KmsUri = "azure-kv://my-vault/hmac-key".parse()?;
let secret = kms::fetch(&uri)?;
# Ok::<(), ironvault::VaultError>(())
```

See [src/kms.rs](https://github.com/nervosys/IronVault/blob/master/src/kms.rs).

## Signing keys

`iv sign --key` and `iv verify --key` accept either a file path or a KMS URI.
The stored secret may be a full keypair JSON document or a bare hex-encoded
32-byte seed — the public key is re-derived from the seed, so both produce the
same signatures.

```bash
iv sign my-llm --key "azure-kv://my-vault/hmac-key"
iv verify my-llm --signature my-llm.sig --key "vault://secret/iv/signing-key"
```

A KMS-backed key is never generated or written to disk: if the URI cannot be
resolved, the command fails rather than silently minting a new key (which a
bare file path does, by design, when the file does not yet exist).
