#!/usr/bin/env bash

# -----------------------------------------------------------------------------
# This tests that we roll back to the previous version after a failed update.
# -----------------------------------------------------------------------------

set -euo pipefail
set -x

source "$(dirname "$0")/../isolate.sh"

# Helper to cleanup background server
HTTP_PID=""
cleanup_server() {
  if [ -n "$HTTP_PID" ] && kill -0 "$HTTP_PID" 2>/dev/null; then
    kill "$HTTP_PID" || true
    wait "$HTTP_PID" 2>/dev/null || true
  fi
}
trap cleanup_server EXIT

# 1) Install current version
./scripts/install.sh --non-interactive --from-source || fail "Install script failed"

# Record current version
BASE_VERSION=$("$HOME/.cargo/bin/kernelle" --version | grep -oE "[0-9]+\.[0-9]+\.[0-9]+" | head -n1)
[ -n "${BASE_VERSION:-}" ] || fail "Could not determine installed kernelle version"

# Seed persistent data we must never lose
mkdir -p "$HOME/.kernelle/persistent"
echo "secret=shhh" > "$HOME/.kernelle/persistent/keeper.env"

# 2) Prepare a fake update server that returns a tarball whose install.sh fails
WORKDIR=$(mktemp -d)
cd "$WORKDIR"
mkdir -p releases/tags artifacts

# Pick a fake version to request
FAKE_VERSION="999.9.9"
TARBALL_URL_FILE="artifacts/kernelle.tar.gz"

# Create a fake release JSON at /releases/tags/v9.9.9
cat > "releases/tags/v$FAKE_VERSION" <<JSON
{"tag_name":"v$FAKE_VERSION","tarball_url":"http://127.0.0.1:7777/$TARBALL_URL_FILE"}
JSON

# Build a tarball with a top-level dir containing "kernelle" and a failing install.sh
ROOT_DIR="fake-kernelle-src"
mkdir -p "$ROOT_DIR/scripts" "$ROOT_DIR/kernelle_home/volatile"
cat > "$ROOT_DIR/scripts/install.sh" <<'BASH'
#!/usr/bin/env bash
set -euo pipefail
echo "Simulated install starting..."
# Simulate some work
sleep 0.1
echo "Simulated install failing"
exit 1
BASH
chmod +x "$ROOT_DIR/scripts/install.sh"

# Package it
tar -czf "$TARBALL_URL_FILE" "$ROOT_DIR"

# Start simple HTTP server
PORT=7777
python3 -m http.server "$PORT" >/dev/null 2>&1 &
HTTP_PID=$!
sleep 0.5

# 3) Run the update pointing to our fake server
set +e
OUT=$({ KERNELLE_UPDATES_API_BASE="http://127.0.0.1:$PORT" "$HOME/.cargo/bin/kernelle" update --version "$FAKE_VERSION"; } 2>&1)
STATUS=$?
set -e

# Expect non-zero exit and rollback messages
[ "$STATUS" -ne 0 ] || fail "Update should have failed to trigger rollback"
echo "$OUT" | grep -qi "automatically rolling back" || fail "Expected rollback to start"
echo "$OUT" | grep -qi "rollback completed successfully" || fail "Expected rollback to complete successfully"

# 4) Validate state after rollback
# - Version unchanged
AFTER_VERSION=$("$HOME/.cargo/bin/kernelle" --version | grep -oE "[0-9]+\.[0-9]+\.[0-9]+" | head -n1)
[ "$AFTER_VERSION" = "$BASE_VERSION" ] || fail "Version changed after failed update (expected $BASE_VERSION, got $AFTER_VERSION)"

# - Persistent data preserved
[ -f "$HOME/.kernelle/persistent/keeper.env" ] || fail "Persistent file missing after rollback"
grep -q "secret=shhh" "$HOME/.kernelle/persistent/keeper.env" || fail "Persistent file contents changed"

# - A snapshot should have been created
[ -d "$HOME/.kernelle/snapshots" ] || fail "Snapshots directory missing"
SNAP_COUNT=$(ls -1 "$HOME/.kernelle/snapshots" | wc -l | tr -d ' ')
[ "$SNAP_COUNT" -ge 1 ] || fail "Expected at least one snapshot to exist"

echo "✅ Rolls-back-after-failed-update verified"
