#!/usr/bin/env bash
set -euo pipefail

# Test basic cleanup functionality
./scripts/cleanup.sh --non-interactive

# Verify only kernelle.internal.source remains in ~/.kernelle/
test -f ~/.kernelle/kernelle.internal.source
test $(find ~/.kernelle -type f | wc -l) -eq 1

# Verify kernelle.internal.source contains gone template contents
diff ~/.kernelle/kernelle.internal.source scripts/templates/kernelle.internal.source.gone.template

# Verify binaries were removed
test ! -f ~/.cargo/bin/kernelle
test ! -f ~/.cargo/bin/jerrod
test ! -f ~/.cargo/bin/blizz
test ! -f ~/.cargo/bin/violet
test ! -f ~/.cargo/bin/adam
test ! -f ~/.cargo/bin/sentinel