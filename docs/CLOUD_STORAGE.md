# Cloud Storage Guide

Push and pull vault models to AWS S3 or Azure Blob Storage.

---

## Contents

- [Security model](#security-model)
- [Build](#build)
- [Command surface](#command-surface)
- [AWS S3](#aws-s3)
- [Azure Blob Storage](#azure-blob-storage)
- [Google Cloud Storage](#google-cloud-storage)
- [Library usage](#library-usage)
- [Troubleshooting](#troubleshooting)

---

## Security model

**As of 4.3.0, `iv cloud push` encrypts before upload.** The payload is
sealed with AES-256-GCM under a key derived from your vault passphrase with
Argon2id, using a fresh random salt per object. What lands in the bucket is
ciphertext; the cloud provider never sees the model.

| Threat                              | Protected?                                     |
| ----------------------------------- | ---------------------------------------------- |
| Network interception                | Yes — TLS, enforced by both SDKs                |
| Local disk theft                    | Yes — AES-256-GCM vault encryption              |
| Bucket read by cloud provider / IAM | Yes — object is ciphertext                      |
| Misconfigured public bucket         | Yes — object is ciphertext                      |
| Passphrase compromise               | **No** — the passphrase is the key              |

Server-side encryption is still worth enabling as defence in depth, but it is
no longer the only thing standing between a bucket misconfiguration and your
models.

### Sealed object format

Each object is self-contained:

```
magic "IRONSEAL" | version | KDF id | salt length | salt | nonce ‖ ciphertext ‖ tag
```

The salt travels with the object rather than living in the vault, which is
what makes a pushed model **portable**: a colleague or CI runner who knows
the passphrase can `pull` into a *different* vault. Uploading the vault's own
on-disk blob would have produced an object only the originating vault
directory could open.

Every header field feeds key derivation, so altering any of them changes the
key and the GCM tag check fails. Tampering produces an error, never wrong
plaintext.

Pushing the same model twice yields different ciphertext — a fresh salt and
nonce per call — so an observer cannot tell that two objects hold the same
model.

### Objects pushed before 4.3.0

Those are plaintext. `pull` detects the absence of the magic bytes, accepts
them so existing data is not stranded, and warns. To fix: re-push with 4.3.0
or later, then delete the old object. Nothing re-encrypts in place.

Versions before 4.3.0 uploaded the decrypted model, and revisions of this
document before 4.2.1 wrongly claimed otherwise. If you sized bucket controls
against the old claim, the objects already up there are still plaintext until
you re-push them.

---

## Build

Cloud backends are optional features:

```bash
cargo build --release --features s3       # AWS S3
cargo build --release --features azure    # Azure Blob Storage
cargo build --release --features cloud    # both
```

There is no `gcs` feature — see [Google Cloud Storage](#google-cloud-storage).

A binary built without the relevant feature still accepts the commands; it
prints `⚠️  S3 support not enabled in this build` and exits without doing
anything.

---

## Command surface

Four subcommands under `iv cloud`. There is no persistent "remote"
configuration — the provider and bucket are passed per invocation, and
credentials come from the environment.

```bash
iv cloud push   <MODEL> --provider <s3|azure> --bucket <NAME> [--version N]
iv cloud pull   <MODEL> --provider <s3|azure> --bucket <NAME> --remote-path <KEY>
iv cloud list   --provider <s3|azure> --bucket <NAME> [--prefix <PATH>]
iv cloud config --provider <s3|azure|gcs> [--show]
```

`push` takes the model name positionally and derives the remote key as
`<model>/<format>/v<version>.vault`. Without `--version` it pushes the
latest. `pull` needs the full `--remote-path` key — it does not derive it —
and stores the downloaded bytes as a *new version* in the local vault, so
pulling a model you already have appends rather than replaces.

Both `push` and `pull` prompt for the vault passphrase.

---

## AWS S3

### IAM policy

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": ["s3:PutObject", "s3:GetObject", "s3:ListBucket"],
      "Resource": [
        "arn:aws:s3:::my-model-bucket",
        "arn:aws:s3:::my-model-bucket/*"
      ]
    }
  ]
}
```

`iv` never calls `DeleteObject`; omit it.

### Credentials

Resolved by the standard AWS SDK chain — environment variables, `~/.aws/config`,
SSO, or an instance/task role. No `iv`-specific configuration.

```bash
export AWS_ACCESS_KEY_ID="..."
export AWS_SECRET_ACCESS_KEY="..."
export AWS_REGION="us-east-1"          # defaults to us-east-1 if unset
```

Note that the backend reads `AWS_REGION`, not `AWS_DEFAULT_REGION`.

Check what is visible to `iv`:

```bash
iv cloud config --provider s3 --show
```

### Usage

```bash
# Push the latest version of gpt-3
iv cloud push gpt-3 --provider s3 --bucket my-model-bucket

# Push a specific version
iv cloud push gpt-3 --version 4 --provider s3 --bucket my-model-bucket

# See what is there
iv cloud list --provider s3 --bucket my-model-bucket --prefix gpt-3/

# Pull it back by full key
iv cloud pull gpt-3 \
    --provider s3 \
    --bucket my-model-bucket \
    --remote-path gpt-3/safetensors/v4.vault
```

### Server-side encryption

Given the security model above, set a bucket default:

```bash
aws s3api put-bucket-encryption \
    --bucket my-model-bucket \
    --server-side-encryption-configuration '{
      "Rules": [{
        "ApplyServerSideEncryptionByDefault": {
          "SSEAlgorithm": "aws:kms",
          "KMSMasterKeyID": "alias/model-vault"
        },
        "BucketKeyEnabled": true
      }]
    }'
```

---

## Azure Blob Storage

Backed by the Azure SDK for Rust v1 (`azure_storage_blob`).

### Credentials

**`AZURE_STORAGE_KEY` is not supported.** The v1 SDK provides no shared-key
credential. If it is set without a SAS token, `iv` fails with an explicit
error rather than silently ignoring it.

Two supported paths, both alongside `AZURE_STORAGE_ACCOUNT`:

**SAS token** — authenticates via the URL itself:

```bash
export AZURE_STORAGE_ACCOUNT="myaccount"
export AZURE_STORAGE_SAS_TOKEN="$(az storage container generate-sas \
    --account-name myaccount --name models \
    --permissions rwdl --expiry 2030-01-01 --output tsv)"
```

**Entra ID** — service principal, managed identity, or `az login`:

```bash
export AZURE_STORAGE_ACCOUNT="myaccount"
export AZURE_TENANT_ID="..."
export AZURE_CLIENT_ID="..."
export AZURE_CLIENT_SECRET="..."
```

Managed identity and developer sign-in are picked up automatically when the
explicit service-principal triple is absent.

```bash
iv cloud config --provider azure --show
```

### Usage

`--bucket` is the container name.

```bash
iv cloud push llama-7b --provider azure --bucket models
iv cloud list --provider azure --bucket models
iv cloud pull llama-7b \
    --provider azure --bucket models \
    --remote-path llama-7b/gguf/v1.vault
```

---

## Google Cloud Storage

**Not available.** GCS support was removed, and there is no `gcs` cargo
feature to enable. The `cloud-storage` crate it depended on carries
unmaintained transitive dependencies (`ring` 0.16, `dotenv`, `instant`).

`iv cloud <cmd> --provider gcs` is accepted by the argument parser and
prints a notice explaining this. Use S3 or Azure.

---

## Library usage

The storage backends are usable directly, and unlike the CLI path you control
what bytes you hand them:

```rust
use ironvault::storage::StorageConfig;

# async fn run() -> ironvault::Result<()> {
let backend = StorageConfig::S3 {
    bucket: "my-model-bucket".to_string(),
    region: "us-east-1".to_string(),
    prefix: Some("production".to_string()),
}
.create_backend()
.await?;

backend.upload("gpt-3/checkpoint-1000", &encrypted_bytes).await?;
let fetched = backend.download("gpt-3/checkpoint-1000").await?;
let keys = backend.list().await?;
# Ok(())
# }
```

`prefix` is honoured by `list`; the CLI always passes `None` for push and
pull and builds the full key itself.

---

## Troubleshooting

**`⚠️  S3 support not enabled in this build`** — rebuild with `--features s3`.

**`AZURE_STORAGE_ACCOUNT env var not set`** — required for every Azure
operation, including `list`.

**`AZURE_STORAGE_KEY (shared key) is no longer supported`** — mint a SAS from
that key, or switch to Entra ID. See above.

**`Unsupported provider: ...`** — only `s3`, `azure`, and `gcs` parse; only
the first two do anything.

**Pull created a duplicate version** — expected. `pull` calls `store_model`,
which appends a new version rather than reconciling with an existing one.

**Region mismatch on S3** — the backend reads `AWS_REGION` and falls back to
`us-east-1`. `AWS_DEFAULT_REGION` is not consulted.

---

See [src/storage.rs](https://github.com/nervosys/IronVault/blob/master/src/storage.rs)
and [src/cli/handlers/cloud.rs](https://github.com/nervosys/IronVault/blob/master/src/cli/handlers/cloud.rs).
