#!/usr/bin/env bash
# Boot the built binary and check what it actually says.
#
# `cargo test` exercises the router in-process; it never starts `iv serve` and
# reads what comes back over a socket. That gap is how `/api/v1/openapi.json`
# shipped declaring version 1.3.0 through five major releases, and how a refused
# bind printed "Listening on ..." before erroring. Both were found by hand, after
# publishing. This is the same check, before.
#
# Usage: scripts/smoke_release.sh [path-to-binary]
set -uo pipefail

BIN="${1:-target/release/iv}"
[ -x "$BIN" ] || BIN="${BIN}.exe"
if [ ! -x "$BIN" ]; then
  echo "no binary at $BIN — build with: cargo build --release --features api" >&2
  exit 1
fi

EXPECTED="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
PORT="${PORT:-18973}"
PASS=0
FAIL=0

check() { # name, actual, expected
  if [ "$2" = "$3" ]; then
    printf 'PASS  %s (%s)\n' "$1" "$2"; PASS=$((PASS + 1))
  else
    printf 'FAIL  %s — got %s, want %s\n' "$1" "$2" "$3"; FAIL=$((FAIL + 1))
  fi
}

echo "verifying $BIN against Cargo.toml version $EXPECTED"
echo

check "iv --version" "$("$BIN" --version 2>/dev/null | awk '{print $2}')" "$EXPECTED"

# A non-loopback bind with no TLS must be refused, and must not claim to be
# listening on the way out.
REFUSAL="$(timeout 20 "$BIN" serve --host 0.0.0.0 --port "$PORT" \
  --jwt-secret 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef 2>&1)"
case "$REFUSAL" in
  *"Refusing to serve"*) printf 'PASS  refuses non-loopback without TLS\n'; PASS=$((PASS + 1)) ;;
  *) printf 'FAIL  non-loopback bind was not refused\n'; FAIL=$((FAIL + 1)) ;;
esac
case "$REFUSAL" in
  *"Listening on"*) printf 'FAIL  printed "Listening on" while refusing to serve\n'; FAIL=$((FAIL + 1)) ;;
  *) printf 'PASS  no false "Listening on" before the refusal\n'; PASS=$((PASS + 1)) ;;
esac

# Loopback should serve, and everything it reports about itself should agree
# with the crate version.
"$BIN" serve --host 127.0.0.1 --port "$PORT" \
  --jwt-secret 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef \
  >/tmp/smoke_serve.log 2>&1 &
SERVER=$!
trap 'kill "$SERVER" 2>/dev/null || true' EXIT

BASE="http://127.0.0.1:$PORT/api/v1"
for _ in $(seq 1 30); do
  [ "$(curl -s -o /dev/null -w '%{http_code}' "$BASE/health" 2>/dev/null)" = "200" ] && break
  sleep 1
done

check "GET /health" "$(curl -s -o /dev/null -w '%{http_code}' "$BASE/health")" "200"
check "health.version" \
  "$(curl -s "$BASE/health" | python -c 'import sys,json;print(json.load(sys.stdin).get("version",""))' 2>/dev/null)" \
  "$EXPECTED"
check "openapi.json info.version" \
  "$(curl -s "$BASE/openapi.json" | python -c 'import sys,json;print(json.load(sys.stdin)["info"]["version"])' 2>/dev/null)" \
  "$EXPECTED"
check "introspect version" \
  "$("$BIN" introspect --format json 2>/dev/null | python -c 'import sys,json;print(json.load(sys.stdin).get("version",""))' 2>/dev/null)" \
  "$EXPECTED"

# The served spec is what clients are generated from, so it should not be empty
# or truncated even when every version string agrees.
PATHS="$(curl -s "$BASE/openapi.json" | python -c 'import sys,json;print(len(json.load(sys.stdin).get("paths",{})))' 2>/dev/null)"
if [ "${PATHS:-0}" -ge 20 ]; then
  printf 'PASS  served spec declares %s paths\n' "$PATHS"; PASS=$((PASS + 1))
else
  printf 'FAIL  served spec declares only %s paths\n' "${PATHS:-0}"; FAIL=$((FAIL + 1))
fi


# The README's agent contract promises `--format json` on every read-style
# command. It was false for these five until 6.2.0 -- the first thing an agent
# integration would hit, and text-only output is not something `cargo test`
# would have noticed.
VAULTDIR="$(mktemp -d)"
export IRONVAULT_HOME="$VAULTDIR"
export IRONVAULT_PASSPHRASE="smoke-test-passphrase-not-a-real-secret"
"$BIN" init >/dev/null 2>&1
head -c 4096 /dev/urandom > "$VAULTDIR/m.safetensors" 2>/dev/null
"$BIN" store smoketest "$VAULTDIR/m.safetensors" >/dev/null 2>&1

for spec in "list" "versions smoketest" "stats" "compliance" "lineage smoketest 1"; do
  # shellcheck disable=SC2086
  if "$BIN" $spec --format json 2>/dev/null | python -c 'import sys,json; json.load(sys.stdin)' 2>/dev/null; then
    printf 'PASS  iv %s --format json\n' "$spec"; PASS=$((PASS + 1))
  else
    printf 'FAIL  iv %s --format json\n' "$spec"; FAIL=$((FAIL + 1))
  fi
done
rm -rf "$VAULTDIR"

# A wrong passphrase must not mint a token. Until 6.2.1 this returned HTTP 200
# with an admin JWT against a vault that had never been unlocked, because
# `Vault::unlock` derived a key without ever checking it. The token then read
# /models, /audit, /acl, /policies and /stats.
AUTHDIR="$(mktemp -d)"
IRONVAULT_HOME="$AUTHDIR" IRONVAULT_PASSPHRASE="the-real-one-2291" "$BIN" init >/dev/null 2>&1
head -c 2048 /dev/urandom > "$AUTHDIR/a.safetensors" 2>/dev/null
IRONVAULT_HOME="$AUTHDIR" IRONVAULT_PASSPHRASE="the-real-one-2291" \
  "$BIN" store authcheck "$AUTHDIR/a.safetensors" >/dev/null 2>&1

APORT=$((PORT + 1))
IRONVAULT_HOME="$AUTHDIR" "$BIN" serve --host 127.0.0.1 --port "$APORT" \
  --jwt-secret 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef \
  >/tmp/smoke_auth.log 2>&1 &
AUTHSRV=$!
for _ in $(seq 1 30); do
  [ "$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$APORT/api/v1/health")" = "200" ] && break
  sleep 1
done

ABASE="http://127.0.0.1:$APORT/api/v1"
BAD="$(curl -s -o /dev/null -w '%{http_code}' -X POST "$ABASE/auth/token" \
  -H 'content-type: application/json' -d '{"passphrase":"definitely-wrong"}')"
check "wrong passphrase is refused a token" "$BAD" "401"

GOOD="$(curl -s -o /dev/null -w '%{http_code}' -X POST "$ABASE/auth/token" \
  -H 'content-type: application/json' -d '{"passphrase":"the-real-one-2291"}')"
check "correct passphrase still gets a token" "$GOOD" "200"

# The CLI side of the same bug: listing succeeded with any passphrase.
if IRONVAULT_HOME="$AUTHDIR" IRONVAULT_PASSPHRASE="definitely-wrong" "$BIN" list >/dev/null 2>&1; then
  printf 'FAIL  iv list succeeded with a wrong passphrase\n'; FAIL=$((FAIL + 1))
else
  printf 'PASS  iv list refuses a wrong passphrase\n'; PASS=$((PASS + 1))
fi

kill "$AUTHSRV" 2>/dev/null || true
rm -rf "$AUTHDIR"
echo
printf '%s passed, %s failed\n' "$PASS" "$FAIL"
[ "$FAIL" -eq 0 ]
