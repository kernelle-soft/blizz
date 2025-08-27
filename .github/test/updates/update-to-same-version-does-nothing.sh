#!/usr/bin/env bash

# -----------------------------------------------------------------------------
# This tests that we do nothing when updating to the same version.
# 1. with the --version flag
# 2. without the --version flag
# -----------------------------------------------------------------------------

set -euo pipefail
set -x

source "$(dirname "$0")/../isolate.sh"

# Install once to provision the isolated env
./scripts/install.sh --non-interactive --from-source || fail "Install script failed"

# Get the installed version (normalize to plain semver)
INSTALLED_VERSION=$("$HOME/.cargo/bin/kernelle" --version | grep -oE "[0-9]+\.[0-9]+\.[0-9]+" | head -n1)
[ -n "${INSTALLED_VERSION:-}" ] || fail "Could not determine installed kernelle version"

# Sanity: no snapshots directory yet
[ ! -d "$HOME/.kernelle/snapshots" ] || fail "Snapshots directory should not exist before update"

# Case 1: update to same version with v-prefix
OUT1=$("$HOME/.cargo/bin/kernelle" update --version "v$INSTALLED_VERSION" 2>&1 || true)
echo "$OUT1" | grep -qi "already up to date" || fail "Expected no-op update when using v$INSTALLED_VERSION"

# Case 2: update to same version without v-prefix
OUT2=$("$HOME/.cargo/bin/kernelle" update --version "$INSTALLED_VERSION" 2>&1 || true)
echo "$OUT2" | grep -qi "already up to date" || fail "Expected no-op update when using $INSTALLED_VERSION"

# Ensure still no snapshots were created by the no-op path
[ ! -d "$HOME/.kernelle/snapshots" ] || fail "No-op update should not create snapshots"

# Ensure binary still works after no-op updates
"$HOME/.cargo/bin/kernelle" --help >/dev/null || fail "kernelle should still work after no-op update"

echo "✅ Update-to-same-version-does-nothing verified"
