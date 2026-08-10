# Blockchain Audit Trail

A Merkle-chained, append-only block store for audit entries.

> **Status: wired as of 4.4.0, opt-in.**
>
> Set `security.blockchain_audit = true` and every audit entry is mirrored
> into a hash-linked chain. Inspect it with `iv chain`. Off by default: the
> chain is append-only and never pruned, so it grows without bound, while
> `audit_log` alone rotates at a size cap.
>
> Before 4.4.0 this was a library primitive that nothing called. An earlier
> revision of this page documented an `iv audit` command that did not exist
> and claimed "every mutating operation is recorded as a block"; neither was
> true then.

## Enabling it

```yaml
security:
  audit_log: true          # required — the chain is fed from the audit logger
  blockchain_audit: true
  blockchain_block_size: 1
```

Entries are recorded from the moment it is switched on. History written
before then is not in the chain and cannot be added retroactively — that is
the point of a hash chain.

### Why `blockchain_block_size` defaults to 1

Pending entries live in memory until a block is finalized. At any value above
1, a process that exits before the threshold **silently drops the entries it
was asked to make tamper-evident**. At 1, every entry is written as its own
block immediately.

Raising it trades that durability for fewer, denser block files. The logger
finalizes on drop, which narrows the window on a clean exit, but a crash or
`SIGKILL` still loses whatever is pending. `iv chain status` reports the
pending count — a non-zero value is exactly what a crash would cost you.

## Commands

| Command | Purpose |
| --- | --- |
| `iv chain status` | Height, latest block hash, pending count |
| `iv chain verify` | Re-verify hash links, Merkle roots, and block hashes |
| `iv chain proof --block N --entry M` | Emit an inclusion proof as JSON |
| `iv chain verify-proof <file>` | Check a proof |
| `iv chain search --model X --event MODEL_STORED` | Find entries |

These read the chain directly rather than through the vault, so they need no
passphrase — and, more importantly, **inspecting the trail does not append to
it**. Going through `Vault` would log a `VaultOpened` entry on every command,
so a `chain verify` cron job would grow the chain by a block per run and
`verify` would disagree with the `status` printed seconds earlier.

`verify` and `verify-proof` exit non-zero (code 5, integrity) when a check
fails, so they work as CI or cron gates.

## What a proof does and does not establish

`verify-proof` confirms the entry hashes to a leaf that reaches the stated
Merkle root, and that the block chain in the proof runs to genesis. It does
**not** confirm that genesis belongs to your vault — compare it against
`iv chain status` on a vault you trust.

Until 4.4.0 it did not confirm the first part either: the Merkle walk started
from a `leaf_hash` carried inside the proof and never checked that hash came
from the entry beside it, so editing the entry left a proof that still
verified clean. `verify_proof` now recomputes the leaf from the entry.

---

## What it provides

| Type              | Purpose                                                  |
| ----------------- | -------------------------------------------------------- |
| `MerkleTree`      | Builds a root over a set of serialised entries            |
| `MerkleProof`     | Inclusion proof for a single entry against a root         |
| `AuditBlock`      | One block: entries, Merkle root, chain link, block hash   |
| `BlockchainAudit` | The chain — append, verify, produce and check proofs      |
| `ChainVerification` / `BlockVerification` | Verification results with issue lists |

## `AuditBlock` fields

| Field         | Description                                       |
| ------------- | ------------------------------------------------- |
| `index`       | Block height                                      |
| `timestamp`   | `DateTime<Utc>` of block creation                 |
| `prev_hash`   | Hash of the previous block                        |
| `merkle_root` | Merkle root over this block's entries             |
| `entries`     | `Vec<BlockEntry>`                                 |
| `signature`   | Optional base64 signature over the block          |
| `nonce`       | Proof-of-work nonce, if enabled                   |
| `hash`        | SHA-256 over index, timestamp, prev_hash, merkle_root, nonce |

Per-operation detail lives one level down: each `BlockEntry` wraps an
`audit: AuditEntry` (timestamp, event type, description, model name, version,
success flag, optional metadata) alongside its own `hash` and
`index_in_block`. There is no `principal`, `operation`, or `payload` field —
an earlier revision of this table listed those, and they do not exist.

## Usage

```rust
use ironvault::audit::AuditEntry;
use ironvault::{BlockchainAudit, Result};

fn record(chain_dir: &std::path::Path, entry: AuditEntry) -> Result<()> {
    // Blocks are sealed automatically every `block_size` entries.
    let mut chain = BlockchainAudit::new(chain_dir, 128)?;

    chain.add_entry(entry)?;
    chain.finalize_block()?; // seal early if you need a boundary now

    let result = chain.verify_chain();
    assert!(result.valid, "{:?}", result.issues);

    // Prove one entry belongs to the chain without shipping the whole log.
    let proof = chain.generate_proof(0, 0)?;
    assert!(BlockchainAudit::verify_proof(&proof).valid);
    Ok(())
}
```

`add_entry` returns the entry hash and seals a block automatically once
`block_size` entries have accumulated. `height`, `latest`, `get_block`, and
`search` round out the read side.

## What verification checks

`AuditBlock::verify` reports an issue for each of: block hash mismatch,
previous-hash mismatch, non-sequential index, timestamp earlier than the
predecessor, a non-genesis block with no predecessor, and Merkle root
mismatch against a tree rebuilt from the entries.

Note that `hash` covers the block header only. Entry tampering is caught
through `merkle_root`, not through the block hash directly.

---

See [src/blockchain.rs](https://github.com/nervosys/IronVault/blob/master/src/blockchain.rs).
