# Telemetry and OTLP export

`iv` ships with **anonymous, opt-in** telemetry — off by default unless the
user explicitly enables it. Nothing is collected or transmitted until then.

## Opt-out (multiple ways, any one suffices)

```bash
export IRONVAULT_TELEMETRY_ENABLED=false
export IRONVAULT_TELEMETRY_DISABLED=1
export DO_NOT_TRACK=1
```

Or `iv telemetry disable`, or set `telemetry.enabled = false` in `config.toml`.

`iv telemetry status` reports the current state and the device ID.

## What is sent (when enabled)

`AppStart`, once per process:

| Field | Example |
|---|---|
| `app.version` | `4.3.0` |
| `os.type` | `linux` |
| `host.arch` | `x86_64` |
| `app.features` | `api,sqlite` |
| `device.id` | random UUID v4, generated on first run |
| `session.id` | random UUID v4, per process |

Neither identifier is derived from anything about the machine or the user, so
neither can be correlated back to an identity.

And `CommandRun`, once per invocation, added in 4.2.0:

| Field | Example |
|---|---|
| `command.name` | `cloud` |
| `command.subcommand` | `push`, or absent |
| `command.duration_ms` | `1420` |
| `command.success` | `true` |

Both names come from clap's registered command table, not from the command
line. `ArgMatches::subcommand_name` can only return a literal declared in
`args.rs`, so the set of values this field can ever hold is the set of
subcommands — a model name, path, or token has no route into it. There is a
test asserting that argument values do not appear in the pair. The failure
*reason* is deliberately not recorded, only the boolean: error messages
interpolate paths and model names.

`ModelOperation`, on `store` / `get` / `delete`:

| Field | Example |
|---|---|
| `model.operation` | `store` — one of three literals |
| `model.format` | `safetensors` — from a fixed set, never a custom string |
| `model.size_bucket` | `small` / `medium` / `large` / `xlarge` |
| `duration.ms`, `outcome.success` | |

The **exact size is never sent**, only the bucket. The format label comes from
`ModelFormat::telemetry_name`, not `name()` — the latter returns whatever
string the user passed for a custom format, and a test asserts it cannot leak.

`Conversion`, on `iv convert` — source and target format labels from the same
fixed set, plus duration and outcome.

`ApiCall`, per HTTP request when running `iv serve`:

| Field | Example |
|---|---|
| `http.route` | `/api/v1/models/{name}` — the **route template** |
| `http.method`, `http.status_code`, `duration.ms` | |

The route is axum's `MatchedPath`, a literal from the router table. The
resolved path is never used: it contains the model name. Requests matching no
route report the constant `<no match>` rather than the requested path, which
is attacker-controlled.

`Error`, when a command fails — `error.type` only, from
`VaultError::kind()`, which returns a fixed literal per variant
(`model_not_found`, `integrity`, …). The message is never sent: every
message-carrying variant interpolates a path or a model name.

`FeatureUsed`, currently only for KMS — `feature.name` is `kms` and
`feature.detail` is the URI *scheme* (`env`, `file`, `aws-sm`, `azure-kv`,
`vault`). The secret id and endpoint are not sent.

## Where events go

The built-in sender posts to a compiled-in default:

```
https://telemetry.nervosys.ai/v1/events
```

That is the project's own collector, operated by NERVOSYS. Nothing is sent
there unless you have explicitly enabled telemetry — `enabled` defaults to
`false`, so a default install never contacts it.

Override it in `config.toml` to send events somewhere you control instead:

```toml
[telemetry]
enabled = true
endpoint = "https://collector.internal.example.com/v1/events"
```

Or bypass the built-in sender entirely and use the `otel` feature with a
standard OTLP collector, described below.

This endpoint went undocumented until 4.2.1, and this page previously stated
that no default collector existed. It did, and it does — for the built-in
sender. The statement was written about the OTLP path and should have said so.

## What is **never** sent

- Model names, file paths, vault contents, passphrases, keys, ACL principals
- Free-form text from any flag

Three fields are free-form `String`s and are the only way the guarantees above
can break: `Error::context`, `ApiCall::endpoint`, and `FeatureUsed::detail`.
As wired today, `context` is always `None`, `endpoint` is always a router
template, and `detail` is always a KMS scheme literal. If you add a call site,
pass a constant — never a formatted message, a resolved path, or anything
derived from an argument.

`telemetry_otlp` has a test pinning the exported attribute key set, so adding
a key requires a deliberate edit.

## OTLP export

Build with the `otel` feature, then configure with the standard OpenTelemetry
environment variables. Any OTLP collector or vendor endpoint works.

```bash
cargo install ironvault --features otel
```

| Variable | Meaning |
|---|---|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | Collector endpoint. Unset means no export. |
| `OTEL_EXPORTER_OTLP_LOGS_ENDPOINT` | Signal-specific override; takes precedence. |
| `OTEL_EXPORTER_OTLP_PROTOCOL` | `http/protobuf` (default) or `http/json` |
| `OTEL_EXPORTER_OTLP_HEADERS` | e.g. `Authorization=Bearer <token>` |
| `OTEL_SERVICE_NAME` | Reported as `service.name` |

Two rules the implementation enforces:

1. **Setting an endpoint does not enable telemetry.** Configuring an exporter
   and consenting to collection are separate decisions, usually made by
   different people. Both are required.
2. **No OTLP endpoint or token is baked into the binary.** The OTLP exporter
   has no default collector and no default credential; unset means no export.
   A credential compiled into an AGPL crate published to a public registry is
   readable by everyone who installs it.

   This applies to the OTLP path only. The built-in sender *does* have a
   compiled-in default endpoint — see [Where events go](#where-events-go).

Building without the `otel` feature while `OTEL_EXPORTER_OTLP_ENDPOINT` is set
warns on stderr rather than silently dropping the configuration.

## Service-scoped configuration

Prefer per-service settings over machine-global ones. A bearer token in
`/etc/environment` or a `profile.d` script is inherited by every process on the
host, including ones that dump their environment on crash.

### systemd

`deploy/systemd/install.sh` provisions the unit and writes the environment
variables at install time, so there is no manual editing step to forget:

```bash
# Put the credential in a file first -- never in an argument.
printf 'Authorization=Bearer %s' "$TOKEN" > /tmp/otlp-headers
chmod 600 /tmp/otlp-headers

sudo ./deploy/systemd/install.sh \
    --otlp-endpoint     https://collector.example.com/otlp \
    --otlp-protocol     http/protobuf \
    --otlp-service-name ironvault \
    --otlp-headers-file /tmp/otlp-headers \
    --enable-telemetry

shred -u /tmp/otlp-headers
```

`--dry-run` prints every change without writing anything, and never prints the
token — only the path it was read from.

The credential is taken from a file rather than a flag because command-line
arguments are world-readable through `/proc/<pid>/cmdline` while the process
runs. A token passed as `--otlp-headers` would be visible to every local user,
which is the exposure `EnvironmentFile` exists to prevent.

The script creates the `ironvault` system user, `/var/lib/ironvault`, and
`/etc/ironvault/server.env` at 0600 root-owned, generates `IRONVAULT_JWT_SECRET` if there
isn't one (preserving an existing value across re-runs), and reloads systemd.
It is idempotent.

Omitting `--enable-telemetry` configures the exporter without turning
collection on — the two remain separate decisions.

To do it by hand instead, copy `ironvault-server.env.example` to
`/etc/ironvault/server.env`, `chmod 0600`, `chown root:root`, and fill it in.

The unit uses `EnvironmentFile=`, not `Environment=`. `Environment=` values are
visible in `systemctl show` and `systemd-analyze dump` to any local user, which
for a bearer token means every account on the machine can read it.

### Containers and Kubernetes

The `Dockerfile` and Helm chart were **removed in 4.5.0**, so there is no
first-party container image or chart to configure. If you run `iv` in a
container you build yourself, the same rule applies as everywhere else: pass
`OTEL_EXPORTER_OTLP_HEADERS` from a mounted secret or an injected environment
variable, never a baked-in image layer or a committed values file.

## Rotating a leaked token

A bearer token grants write access to your telemetry backend. Treat it as
compromised if it has ever appeared in a shell history, a chat window, a CI
log, a screenshot, or a commit — including one later amended or force-pushed,
since the object usually survives in the reflog and on any fork.

Rotate at the provider, then update `/etc/ironvault/server.env` or whatever supplies
the environment where `iv` runs. No application change is needed; the value is
read from the environment at process start.

See [src/telemetry.rs](https://github.com/nervosys/IronVault/blob/master/src/telemetry.rs)
and [src/telemetry_otlp.rs](https://github.com/nervosys/IronVault/blob/master/src/telemetry_otlp.rs).
