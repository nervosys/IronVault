#!/usr/bin/env bash
#
# Install the ironvault-server systemd unit with service-scoped configuration.
#
# Everything this script writes applies to the ironvault-server unit alone. Nothing
# is written to /etc/environment, /etc/profile.d, or any other machine-global
# location, so no other process on the host inherits the collector endpoint or
# the bearer token.
#
# Usage:
#   sudo ./install.sh [options]
#
#   --otlp-endpoint URL        e.g. https://collector.example.com/otlp
#   --otlp-protocol PROTO      http/protobuf (default) or http/json
#   --otlp-service-name NAME   reported as service.name
#   --otlp-headers-file PATH   file whose contents are the OTLP headers value,
#                              e.g. "Authorization=Bearer <token>"
#   --enable-telemetry         opt this deployment in to reporting
#   --binary PATH              iv binary to install (default: ./target/release/iv)
#   --dry-run                  print what would change, write nothing
#
# The credential is read from a *file*, never from an argument. Command-line
# arguments are world-readable through /proc/<pid>/cmdline for the lifetime of
# the process, so a token passed as --otlp-headers would be visible to every
# local user, which is the exact exposure EnvironmentFile exists to avoid.

set -euo pipefail

ENV_DIR=/etc/ironvault
ENV_FILE="${ENV_DIR}/server.env"
UNIT_SRC="$(dirname "$0")/ironvault-server.service"
UNIT_DST=/etc/systemd/system/ironvault-server.service
STATE_DIR=/var/lib/ironvault
SERVICE_USER=ironvault
BINARY_SRC="./target/release/iv"
BINARY_DST=/usr/local/bin/iv

OTLP_ENDPOINT=""
OTLP_PROTOCOL="http/protobuf"
OTLP_SERVICE_NAME="ironvault"
OTLP_HEADERS_FILE=""
TELEMETRY_ENABLED="false"
DRY_RUN=0

die() { printf 'error: %s\n' "$*" >&2; exit 1; }
note() { printf '  %s\n' "$*"; }
run() { if [ "$DRY_RUN" -eq 1 ]; then printf '  would run: %s\n' "$*"; else "$@"; fi; }

while [ $# -gt 0 ]; do
    case "$1" in
        --otlp-endpoint)      OTLP_ENDPOINT="${2:?--otlp-endpoint needs a value}"; shift 2 ;;
        --otlp-protocol)      OTLP_PROTOCOL="${2:?--otlp-protocol needs a value}"; shift 2 ;;
        --otlp-service-name)  OTLP_SERVICE_NAME="${2:?--otlp-service-name needs a value}"; shift 2 ;;
        --otlp-headers-file)  OTLP_HEADERS_FILE="${2:?--otlp-headers-file needs a path}"; shift 2 ;;
        --enable-telemetry)   TELEMETRY_ENABLED="true"; shift ;;
        --binary)             BINARY_SRC="${2:?--binary needs a path}"; shift 2 ;;
        --dry-run)            DRY_RUN=1; shift ;;
        -h|--help)            sed -n '3,28p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *)                    die "unknown option: $1 (try --help)" ;;
    esac
done

[ "$DRY_RUN" -eq 1 ] || [ "$(id -u)" -eq 0 ] || die "must run as root (or pass --dry-run)"

case "$OTLP_PROTOCOL" in
    http/protobuf|http/json) ;;
    *) die "--otlp-protocol must be http/protobuf or http/json, got: $OTLP_PROTOCOL" ;;
esac

if [ -n "$OTLP_HEADERS_FILE" ]; then
    [ -r "$OTLP_HEADERS_FILE" ] || die "cannot read --otlp-headers-file: $OTLP_HEADERS_FILE"
    [ -n "$OTLP_ENDPOINT" ] || die "--otlp-headers-file given without --otlp-endpoint"
fi

# Telemetry stays a separate decision from exporter configuration. Pointing a
# deployment at a collector and consenting to report are usually made by
# different people; requiring both means neither is implied by the other.
if [ "$TELEMETRY_ENABLED" = "true" ]; then
    printf 'Telemetry will be ENABLED for this unit.\n'
    [ -n "$OTLP_ENDPOINT" ] || note "no OTLP endpoint set; the built-in collector will be used"
else
    note "telemetry disabled (default); pass --enable-telemetry to opt in"
fi

printf 'Installing ironvault-server:\n'

# --- service account and directories ----------------------------------------
if ! id -u "$SERVICE_USER" >/dev/null 2>&1; then
    note "creating system user ${SERVICE_USER}"
    run useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER"
else
    note "system user ${SERVICE_USER} already exists"
fi

run install -d -m 0755 -o root -g root "$ENV_DIR"
run install -d -m 0750 -o "$SERVICE_USER" -g "$SERVICE_USER" "$STATE_DIR"

# --- binary ------------------------------------------------------------------
if [ -f "$BINARY_SRC" ]; then
    note "installing $BINARY_SRC -> $BINARY_DST"
    run install -m 0755 -o root -g root "$BINARY_SRC" "$BINARY_DST"
else
    note "binary not found at ${BINARY_SRC}; skipping (build with: cargo build --release --features api,otel)"
fi

# --- environment file --------------------------------------------------------
# Written through a 0600 temp file and renamed, so the secret is never briefly
# readable at a wider mode: install(1) creates the destination at its final
# mode, but a plain redirect would inherit the umask.
write_env_file() {
    local tmp
    tmp="$(mktemp "${ENV_DIR}/.server.env.XXXXXX")"
    chmod 0600 "$tmp"

    local jwt_secret
    if [ -f "$ENV_FILE" ] && grep -q '^IRONVAULT_JWT_SECRET=.\+' "$ENV_FILE"; then
        jwt_secret="$(grep '^IRONVAULT_JWT_SECRET=' "$ENV_FILE" | head -1 | cut -d= -f2-)"
        note "preserving existing IRONVAULT_JWT_SECRET"
    else
        jwt_secret="$(openssl rand -base64 48 | tr -d '\n')"
        note "generated a new IRONVAULT_JWT_SECRET"
    fi

    {
        echo "# Managed by deploy/systemd/install.sh. Service-scoped: loaded by the"
        echo "# ironvault-server unit only, never by /etc/environment or a profile script."
        echo "# Mode 0600, owned by root. Contains credentials -- do not commit."
        echo
        echo "IRONVAULT_JWT_SECRET=${jwt_secret}"
        echo
        echo "IRONVAULT_TELEMETRY_ENABLED=${TELEMETRY_ENABLED}"

        if [ -n "$OTLP_ENDPOINT" ]; then
            echo
            echo "OTEL_EXPORTER_OTLP_ENDPOINT=${OTLP_ENDPOINT}"
            echo "OTEL_EXPORTER_OTLP_PROTOCOL=${OTLP_PROTOCOL}"
            echo "OTEL_SERVICE_NAME=${OTLP_SERVICE_NAME}"
        fi

        if [ -n "$OTLP_HEADERS_FILE" ]; then
            # Read straight from the file into the destination. The value is
            # never echoed, never interpolated into a log line, and never
            # placed in this script's argv.
            printf 'OTEL_EXPORTER_OTLP_HEADERS=%s\n' "$(tr -d '\r\n' < "$OTLP_HEADERS_FILE")"
        fi
    } > "$tmp"

    chown root:root "$tmp"
    mv -f "$tmp" "$ENV_FILE"
}

if [ "$DRY_RUN" -eq 1 ]; then
    note "would write ${ENV_FILE} (0600 root:root) with:"
    note "    IRONVAULT_TELEMETRY_ENABLED=${TELEMETRY_ENABLED}"
    [ -n "$OTLP_ENDPOINT" ] && note "    OTEL_EXPORTER_OTLP_ENDPOINT=${OTLP_ENDPOINT}"
    [ -n "$OTLP_ENDPOINT" ] && note "    OTEL_EXPORTER_OTLP_PROTOCOL=${OTLP_PROTOCOL}"
    [ -n "$OTLP_ENDPOINT" ] && note "    OTEL_SERVICE_NAME=${OTLP_SERVICE_NAME}"
    [ -n "$OTLP_HEADERS_FILE" ] && note "    OTEL_EXPORTER_OTLP_HEADERS=<from ${OTLP_HEADERS_FILE}, not shown>"
else
    note "writing ${ENV_FILE} (0600 root:root)"
    write_env_file
fi

# --- unit --------------------------------------------------------------------
[ -f "$UNIT_SRC" ] || die "unit file not found: $UNIT_SRC"
note "installing unit -> ${UNIT_DST}"
run install -m 0644 -o root -g root "$UNIT_SRC" "$UNIT_DST"
run systemctl daemon-reload

printf '\nDone. Start it with:\n  sudo systemctl enable --now ironvault-server\n'

if [ -n "$OTLP_HEADERS_FILE" ] && [ "$DRY_RUN" -eq 0 ]; then
    printf '\nThe token is now in %s and nowhere else on this host.\n' "$ENV_FILE"
    printf 'Remove %s if it was a scratch copy.\n' "$OTLP_HEADERS_FILE"
fi
