#!/usr/bin/env bash


set -euo pipefail
set -x

# Isolate the test
source "$(dirname "$0")/isolate.sh"

# Create existing directories to simulate "already installed" state
mkdir -p ~/.cargo/bin
mkdir -p ~/.blizz

echo "🔧 Simulated existing directories for 'already installed' test"

# Test install
./scripts/install.sh --non-interactive --from-source || fail "Install script failed"

# Verify uninstaller and template were installed
test -f ~/.blizz/uninstall.sh || fail "~/.blizz/uninstall.sh not found after install"
test -f ~/.blizz/volatile/blizz.internal.source.gone.template || fail "~/.blizz/volatile/blizz.internal.source.gone.template not found after install"


# Debug: show possible binary locations
echo "ls -l $HOME/.cargo/bin:"
test -f "$HOME/.cargo/bin/blizz" || fail "blizz binary not found after install"

echo "✅ Install-when-already-installed test completed successfully"
