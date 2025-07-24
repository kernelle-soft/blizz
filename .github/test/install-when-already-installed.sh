#!/usr/bin/env bash

# Isolate the test
source "$(dirname "$0")/isolate.sh"

# Create existing directories to simulate "already installed" state
mkdir -p ~/.cargo/bin
mkdir -p ~/.kernelle

echo "🔧 Simulated existing directories for 'already installed' test"

# Test install
./scripts/install.sh --non-interactive

# Should still work
test -f ~/.cargo/bin/kernelle

echo "✅ Install-when-already-installed test completed successfully"
