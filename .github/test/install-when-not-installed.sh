#!/usr/bin/env bash
set -euo pipefail

./scripts/install.sh --non-interactive

# Should create ~/.cargo/bin/kernelle
test -f ~/.cargo/bin/kernelle

# Should create ~/.kernelle
test -d ~/.kernelle

# Check that binaries were installed (bentley is library-only, so exclude it)
ls -la ~/.cargo/bin/ | grep -E "(kernelle|jerrod|blizz|violet|adam|sentinel)"

# Check that .kernelle.source was created
test -f ~/.kernelle.source

# Check that .kernelle directory structure exists
test -d ~/.kernelle
test -d ~/.kernelle/.cursor

# Test that kernelle binary works
~/.cargo/bin/kernelle --help