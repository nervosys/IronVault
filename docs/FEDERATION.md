# Federation

Multi-peer vault synchronization with vector clocks for conflict detection.

> **Status: wired as of 4.4.0, opt-in.**
>
> Before 4.4.0 `FederationManager` was a complete HTTP *client* pointed at
> three endpoints nothing served, so a node could only sync against a peer
> that did not exist. The server half, the `iv federation` commands, and
> transit encryption all landed in 4.4.0. An earlier revision of this page
> claimed REST exposure that was not there.

## Concepts

- **Peer** — another `iv` node identified by a stable ID and reachable endpoint.
- **Vector clock** — per-peer monotonic counter attached to every mutation.
- **Conflict** — concurrent writes to the same model from different peers.

## Configuration

```yaml
federation:
  enabled: true
  node_id: node-a          # stable, not generated per run — see below
  node_name: node-a
  seal_transfers: true
  peers:
    - node_id: node-b
      name: node-b
      endpoint: https://node-b.internal:8080
      api_key: env://FED_SHARED_KEY   # KMS URI preferred over a literal
      enabled: true
```

Enabling federation exposes `/api/v1/federation/*` on `iv serve`. Those
endpoints hand out model bytes, so this is deliberately opt-in and the routes
are not registered at all when it is off — an unauthenticated caller gets a
404 rather than a 401 confirming the endpoint exists.

**Set `node_id` to a stable value.** Vector clocks are keyed by it, so a
generated id makes the node look brand new after every restart and clock
comparison stops meaning anything. `iv` warns on stderr when it is unset.

## Authentication

Peers authenticate with a shared key in `X-API-Key` — not the JWT the rest of
the API uses. A peer is a machine holding a long-lived pre-shared secret, not
a user with a session, and issuing it a login token would hand it the full
model API just to fetch weights.

The same key works in both directions: this node presents it to the peer and
accepts it from the peer, so there is one secret per pair rather than two
half-configured ones. Disabling a peer revokes it in **both** directions.

Prefer a KMS URI (`env://`, `file://`, `aws-sm://`, `azure-kv://`, `vault://`)
over a literal — a literal is a secret sitting in a config file, and config
files end up in version control. Keys are resolved at startup, so a bad
reference aborts the server rather than failing the first sync at 3am.

## Transit encryption

`seal_transfers` (default **true**) wraps model bytes in the same `IRONSEAL`
envelope used for cloud uploads, keyed by a passphrase both nodes share:

```bash
export IRONVAULT_FEDERATION_PASSPHRASE='…'   # same value on both peers
```

TLS protects the hop; this protects the object, so a peer's reverse proxy,
request log, or on-disk cache never holds a readable model. It is read from
the environment and never from the config file.

An **unsealed** model arriving while `seal_transfers` is on is refused, not
stored: otherwise a peer could downgrade the transfer simply by sending
plaintext. Turning sealing off is only defensible on a network you fully
control, and `iv serve` warns at startup when it is off.

## Serving requires an unlocked vault

Peers hold the federation key, never the vault passphrase. When federation is
enabled, `iv serve` unlocks the vault at startup from
`$IRONVAULT_PASSPHRASE` (a literal or KMS URI); without it the vault stays
locked and every peer request fails until someone POSTs to `/auth/token` by
hand. A plain `iv serve` with federation off still starts locked, as before.

## Commands

| Command | Purpose |
| --- | --- |
| `iv federation status` | Node identity, peers, sealing mode, recent syncs |
| `iv federation manifest` | This node's manifest — what peers would see |
| `iv federation plan <peer>` | What a sync would transfer, transferring nothing |
| `iv federation sync <peer>` | Sync: download what is missing here, upload what is missing there |

`plan` is worth running first against an unfamiliar peer; it fetches the
remote manifest and prints the delta without moving any bytes.

## Endpoints

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/api/v1/federation/manifest` | This node's models and vector clocks |
| `GET` | `/api/v1/federation/models/{name}/versions/{checkpoint_id}` | Fetch a version (sealed) |
| `PUT` | `/api/v1/federation/models/{name}/versions/{checkpoint_id}` | Push a version |

`PUT` is idempotent: re-pushing a version already present returns
`already_present` rather than creating a duplicate.

## How versions are identified

Federation addresses versions by **checkpoint id**, not version number —
version numbers are per-vault sequences and mean different things on different
nodes.

A received copy keeps the checkpoint id it arrived with, recorded in version
metadata as `federation_origin_checkpoint_id` and advertised in place of the
locally minted id. This is what makes sync converge. Without it the receiver
advertises an id the sender has never seen, the sender finds its version still
"missing", and every run re-transfers the model — the vaults duplicate it
forever. That bug was real in pre-release 4.4.0 and is covered by a regression
test.

## Conflicts

A conflict is concurrent writes to the same model from different peers.
`auto_resolve_conflicts` currently applies last-writer-wins by keeping the
local copy; otherwise conflicts are reported for manual resolution. Both
`plan` and `sync` list them.

## When to use it

Use federation when multiple workstations or CI runners need to share a single
logical vault without a central server. For centralized storage, prefer the
cloud backends (S3, Azure Blob) — see [CLOUD_STORAGE.md](CLOUD_STORAGE.md).

See [src/federation.rs](https://github.com/nervosys/IronVault/blob/master/src/federation.rs),
[src/federation_transport.rs](https://github.com/nervosys/IronVault/blob/master/src/federation_transport.rs),
and [src/api/federation_routes.rs](https://github.com/nervosys/IronVault/blob/master/src/api/federation_routes.rs).
