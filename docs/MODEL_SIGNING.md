# Model Signing & Verification

HMAC-SHA256 model signing with detached `.sig` JSON files for tamper detection and provenance tracking.

## Quick Start

```bash
# Sign a vault model (auto-generates key on first use)
iv sign my-model

# Sign with identity
iv sign my-model --identity "ML Team <ml@company.com>"

# Verify a signature. --key is required for a real check; without it the
# command reports the signature as NOT CHECKED and exits non-zero.
iv verify my-model --signature my-model.sig --key signing_key.json

# Sign a file on disk
iv sign my-model --file ./model.safetensors
```

## CLI Reference

### sign

```
iv sign <NAME> [OPTIONS]

Arguments:
  <NAME>              Model name in vault

Options:
  -v, --version <V>   Model version (default: latest)
  -k, --key <KEY>     Path to signing key JSON file
  -i, --identity <ID> Signer identity (name/email)
  --file <PATH>       Sign a file on disk instead of vault model
```

### verify

```
iv verify <NAME> --signature <SIG> [OPTIONS]

Arguments:
  <NAME>              Model name in vault

Options:
  --signature <SIG>   Path to .sig file
  -k, --key <KEY>     Path to signing key for verification
  --file <PATH>       Verify a file on disk
  -v, --version <V>   Model version
```

## How It Works

1. **Key Generation** — A signing keypair (`SigningKeyPair`) is auto-generated on first use and saved to `<config_dir>/signing_key.json`
2. **Signing** — HMAC-SHA256 is computed over the file content using the secret seed
3. **Detached Signature** — A `.sig` JSON file is created containing signature, public key, file hash, signer identity, and timestamp
4. **Verification** — The file is re-hashed and compared against the `.sig`, then the HMAC tag is recomputed from the secret seed and compared in constant time

> **The verification key is not optional.** Everything in a `.sig` file — including
> `file_sha256` — is attacker-controlled if the attacker controls the file. Verifying
> without the secret seed can only confirm the `.sig` is internally consistent, which
> anyone can forge. `verify` therefore reports `valid: false` and
> `signature_checked: false` when no key is supplied.

## Signature File Format

```json
{
  "signature": "hex-encoded HMAC-SHA256",
  "public_key": "hex-encoded 32-byte key",
  "file_sha256": "hex-encoded SHA-256 of model file",
  "signer": "ML Team <ml@company.com>",
  "signed_at": "2026-04-04T12:00:00Z",
  "version": 2,
  "metadata": {}
}
```

## Rust API

```rust
use ironvault::signing::{ModelSigner, SigningKeyPair};

// Generate keypair
let keypair = ModelSigner::generate_keypair(Some("ML Team"))?;
ModelSigner::save_keypair(&keypair, "signing_key.json")?;

// Sign a file
let signature = ModelSigner::sign(&keypair, Path::new("model.safetensors"), HashMap::new())?;
ModelSigner::save_signature(&signature, Path::new("model.sig"))?;

// Verify — the secret seed is required, or `valid` is false and
// `signature_checked` is false.
let loaded_sig = ModelSigner::load_signature(Path::new("model.sig"))?;
let result = ModelSigner::verify(
    &loaded_sig,
    Path::new("model.safetensors"),
    Some(&keypair.secret_seed),
)?;
assert!(result.valid);
```

## Signature Versions

| `version` | Tag construction | Status |
| --- | --- | --- |
| 1 | `SHA-256(seed \|\| file_hash)` | Accepted on verify; vulnerable to length extension. Re-sign to upgrade. |
| 2 | `HMAC-SHA256(seed, file_hash)` (RFC 2104) | Written by all current signatures. |
