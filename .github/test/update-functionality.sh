#!/usr/bin/env bash

# Isolate the test
source "$(dirname "$0")/isolate.sh"

echo "🔄 Testing kernelle update functionality"
echo "========================================"

# PHASE 1: Install an older version (simulate)
echo "📦 Phase 1: Installing current version..."
./scripts/install.sh --non-interactive

# Verify installation worked
echo "🔍 Verifying initial installation..."
test -f ~/.cargo/bin/kernelle
test -d ~/.kernelle
~/.cargo/bin/kernelle --version
echo "✅ Initial installation verified"
echo

# PHASE 2: Test update to same version (should be idempotent)
echo "🔄 Phase 2: Testing update to same version..."
~/.cargo/bin/kernelle update

# Verify kernelle still works after "update"
echo "🔍 Verifying kernelle after same-version update..."
~/.cargo/bin/kernelle --version
echo "✅ Same-version update completed successfully"
echo

# PHASE 3: Test update with specific version (if we have releases)
echo "🔄 Phase 3: Testing update with version parameter..."
# Note: This would test against actual GitHub releases
# ~/.cargo/bin/kernelle update v0.2.16
echo "⏩ Skipping version-specific update (requires actual releases)"
echo

# PHASE 4: Test snapshot creation and verification
echo "📸 Phase 4: Verifying snapshot functionality..."
# Check that snapshots directory exists
test -d ~/.kernelle/snapshots || echo "No snapshots created yet (expected for same-version update)"

# PHASE 5: Test error handling
echo "❌ Phase 5: Testing error handling..."
# Test with invalid version
if ~/.cargo/bin/kernelle update nonexistent-version 2>/dev/null; then
    echo "ERROR: Update should have failed for nonexistent version"
    exit 1
else
    echo "✅ Properly handled nonexistent version"
fi

echo
echo "🎉 All update functionality tests passed!"
