#!/usr/bin/env bash


set -euo pipefail
set -x

fail() { echo "❌ $1" >&2; exit 1; }

# Isolate the test
source "$(dirname "$0")/isolate.sh"

# Create existing directories to simulate "already installed" state
mkdir -p ~/.cargo/bin
mkdir -p ~/.kernelle

echo "🔧 Simulated existing directories for 'already installed' test"

# Test install
./scripts/install.sh --non-interactive || fail "Install script failed"



# Debug: show possible binary locations
echo "ls -l $HOME/.cargo/bin:"
test -f "$HOME/.cargo/bin/kernelle" || fail "kernelle binary not found after install"

echo "✅ Install-when-already-installed test completed successfully"
