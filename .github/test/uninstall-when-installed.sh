#!/usr/bin/env bash

# Isolate the test
source "$(dirname "$0")/isolate.sh"

# First, install kernelle so we can test uninstalling it
echo "🔧 Setting up pre-installed state for uninstall test..."
./scripts/install.sh --non-interactive

# Verify it was installed
test -f ~/.cargo/bin/kernelle
echo "✅ Pre-installed state verified"

# Now test basic cleanup functionality
echo "🧹 Testing cleanup functionality..."
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

echo "✅ Uninstall test completed successfully"