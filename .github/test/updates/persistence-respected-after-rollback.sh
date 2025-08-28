#!/usr/bin/env bash

# -----------------------------------------------------------------------------
# This tests that we respect persisted data even after a failed update.
# User data and/or any other persisted state should not be lost after a 
# rollback.
# -----------------------------------------------------------------------------

set -euo pipefail
set -x

source "$(dirname "$0")/../isolate.sh"

./scripts/install.sh --non-interactive --from-source || fail "Install script failed"

# Seed persistent data
mkdir -p "$HOME/.blizz/persistent"
echo "token=very-secret" > "$HOME/.blizz/persistent/keeper.env"
mkdir -p "$HOME/.blizz/persistent/keepme"
echo "keep" > "$HOME/.blizz/persistent/keepme/file.txt"

# Prepare a failing update via local server
WORKDIR=$(mktemp -d)
cd "$WORKDIR"
mkdir -p releases/tags artifacts

FAKE_VERSION="7.7.7"
TARBALL_URL_FILE="artifacts/blizz.tar.gz"

cat > "releases/tags/v$FAKE_VERSION" <<JSON
{"tag_name":"v$FAKE_VERSION","tarball_url":"http://127.0.0.1:7779/$TARBALL_URL_FILE"}
JSON

ROOT_DIR="fake-blizz-src"
mkdir -p "$ROOT_DIR/scripts" "$ROOT_DIR/blizz_home/volatile"
cat > "$ROOT_DIR/scripts/install.sh" <<'BASH'
#!/usr/bin/env bash
set -euo pipefail
echo "Simulated install starting..."
sleep 0.1
echo "boom"
exit 1
BASH
chmod +x "$ROOT_DIR/scripts/install.sh"

tar -czf "$TARBALL_URL_FILE" "$ROOT_DIR"

PORT=7779
python3 -m http.server "$PORT" >/dev/null 2>&1 &
HTTP_PID=$!
trap 'kill $HTTP_PID 2>/dev/null || true' EXIT
sleep 0.5

set +e
OUT=$({ BLIZZ_UPDATES_API_BASE="http://127.0.0.1:$PORT" "$HOME/.cargo/bin/blizz" update --version "$FAKE_VERSION"; } 2>&1)
STATUS=$?
set -e

[ "$STATUS" -ne 0 ] || fail "Update should fail"
echo "$OUT" | grep -qi "rollback completed successfully" || fail "Rollback did not complete"

# Verify persistent data remains
[ -f "$HOME/.blizz/persistent/keeper.env" ] || fail "keeper.env missing after rollback"
[ -f "$HOME/.blizz/persistent/keepme/file.txt" ] || fail "nested persistent file missing after rollback"
grep -q "token=very-secret" "$HOME/.blizz/persistent/keeper.env" || fail "keeper.env changed after rollback"

echo "✅ Persistence-respected-after-rollback verified"
