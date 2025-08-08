#!/usr/bin/env bash


set -euo pipefail
set -x

# Isolate the test
source "$(dirname "$0")/isolate.sh"

# Test cleanup on clean system (nothing should be installed)
echo "🧹 Testing cleanup on clean system..."
./scripts/uninstall.sh || fail "Uninstall script failed on clean system"

# Should not error even if nothing to clean up
echo "✅ Cleanup completed successfully on clean system" 
