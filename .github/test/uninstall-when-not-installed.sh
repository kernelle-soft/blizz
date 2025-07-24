#!/usr/bin/env bash

# Isolate the test
source "$(dirname "$0")/isolate.sh"

# Test cleanup on clean system (nothing should be installed)
echo "🧹 Testing cleanup on clean system..."
./scripts/uninstall.sh --non-interactive --keep-insights

# Should not error even if nothing to clean up
echo "✅ Cleanup completed successfully on clean system" 