#!/usr/bin/env bash
set -euo pipefail

# Test cleanup on clean system
./scripts/cleanup.sh --non-interactive --keep-insights

# Should not error even if nothing to clean up
echo "Cleanup completed successfully" 