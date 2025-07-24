#!/usr/bin/env bash
set -euo pipefail

# Create existing directories
mkdir -p ~/.cargo/bin
mkdir -p ~/.kernelle

# Test install
./scripts/install.sh --non-interactive

# Should still work
test -f ~/.cargo/bin/kernelle