#!/usr/bin/env bash

# -----------------------------------------------------------------------------
# This tests that we respect persisted data even after an update.
# User data and/or any other persisted state should not be lost after an 
# update.
# -----------------------------------------------------------------------------

set -euo pipefail
set -x

source "$(dirname "$0")/../isolate.sh"

# Install current version
./scripts/install.sh --non-interactive || fail "Install script failed"

# Seed persistent data
mkdir -p "$HOME/.kernelle/persistent"
echo "api_key=abc123" > "$HOME/.kernelle/persistent/config.env"
mkdir -p "$HOME/.kernelle/persistent/data"
echo "do-not-touch" > "$HOME/.kernelle/persistent/data/marker.txt"

# Prepare fake update server with a successful install.sh
WORKDIR=$(mktemp -d)
cd "$WORKDIR"
mkdir -p releases/tags artifacts

FAKE_VERSION="8.8.8"
TARBALL_URL_FILE="artifacts/kernelle.tar.gz"

cat > "releases/tags/v$FAKE_VERSION" <<JSON
{"tag_name":"v$FAKE_VERSION","tarball_url":"http://127.0.0.1:7778/$TARBALL_URL_FILE"}
JSON

# Build a tarball with a top-level dir and an install.sh that succeeds
ROOT_DIR="fake-kernelle-src"
mkdir -p "$ROOT_DIR/scripts" "$ROOT_DIR/kernelle_home/volatile"
cat > "$ROOT_DIR/scripts/install.sh" <<'BASH'
#!/usr/bin/env bash
set -euo pipefail
echo "Simulated install starting..."
sleep 0.1
# Create expected structure in staging kernelle_home to be copied over
KERNELLE_HOME="${KERNELLE_HOME:-$HOME/.kernelle}"
mkdir -p "$KERNELLE_HOME/volatile"
# Write a file to volatile to ensure copy happens
echo "ok" > "$KERNELLE_HOME/volatile/updated.txt"
# Pretend to install binaries is handled by cargo in real flow; here we do nothing
exit 0
BASH
chmod +x "$ROOT_DIR/scripts/install.sh"

tar -czf "$TARBALL_URL_FILE" "$ROOT_DIR"

# Start HTTP server
PORT=7778
python3 -m http.server "$PORT" >/dev/null 2>&1 &
HTTP_PID=$!
trap 'kill $HTTP_PID 2>/dev/null || true' EXIT
sleep 0.5

# Run the update pointing to our fake server
KERNELLE_UPDATES_API_BASE="http://127.0.0.1:$PORT" "$HOME/.cargo/bin/kernelle" update --version "$FAKE_VERSION"

# Validate persistent data preserved
[ -f "$HOME/.kernelle/persistent/config.env" ] || fail "Persistent config missing after update"
grep -q "api_key=abc123" "$HOME/.kernelle/persistent/config.env" || fail "Persistent config changed"
[ -f "$HOME/.kernelle/persistent/data/marker.txt" ] || fail "Persistent marker missing after update"

echo "✅ Persistence-respected-after-update verified"
