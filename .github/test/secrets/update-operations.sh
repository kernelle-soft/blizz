#!/usr/bin/env bash

# -----------------------------------------------------------------------------
# End-to-end test for secrets update operations
# Tests updating existing secrets with --force flag
# -----------------------------------------------------------------------------

set -euo pipefail
set -x

source "$(dirname "$0")/../isolate.sh"

echo "🔄 Testing secrets update operations"
echo "===================================="

# Install kernelle to get the secrets binary
./scripts/install.sh --non-interactive || fail "Install script failed"

# Set up environment variables for testing
export SECRETS_AUTH="test_password_123"
export SECRETS_QUIET="1"
export KERNELLE_HOME="$HOME/.kernelle"  # Ensure consistent path for CLI and daemon

# Verify secrets binary exists
test -f "$HOME/.cargo/bin/secrets" || fail "secrets binary not found after install"

echo "Setting up test data..."

# Store initial secrets
"$HOME/.cargo/bin/secrets" store api_key "original_value" || fail "Failed to store original secret"
"$HOME/.cargo/bin/secrets" store -g github token "original_token" || fail "Failed to store original GitHub token"

echo "Testing update operations..."

# Try to store existing secret without force (should fail)
if "$HOME/.cargo/bin/secrets" store api_key "new_value" 2>/dev/null; then
    fail "Storing existing secret without --force should fail"
fi
echo "✅ Storing existing secret without --force correctly fails"

# Update secret with force flag
"$HOME/.cargo/bin/secrets" store --force api_key "updated_value" || fail "Failed to update secret with --force"
echo "✅ Updated secret with --force flag"

# Verify the value was updated
OUTPUT=$("$HOME/.cargo/bin/secrets" read api_key)
[ "$OUTPUT" = "updated_value" ] || fail "Updated value doesn't match. Got: '$OUTPUT'"
echo "✅ Verified secret was updated"

# Update secret in specific group with force flag
"$HOME/.cargo/bin/secrets" store --force -g github token "updated_token" || fail "Failed to update GitHub token with --force"
echo "✅ Updated secret in specific group with --force"

# Verify the group secret was updated
OUTPUT=$("$HOME/.cargo/bin/secrets" read -g github token)
[ "$OUTPUT" = "updated_token" ] || fail "Updated GitHub token doesn't match. Got: '$OUTPUT'"
echo "✅ Verified group secret was updated"

# Test force flag in different position
"$HOME/.cargo/bin/secrets" store api_key "final_value" --force || fail "Failed to update with --force at end"
OUTPUT=$("$HOME/.cargo/bin/secrets" read api_key)
[ "$OUTPUT" = "final_value" ] || fail "Final updated value doesn't match. Got: '$OUTPUT'"
echo "✅ Force flag works in different position"

# Test updating non-existent secret with force (should work like normal store)
"$HOME/.cargo/bin/secrets" store --force new_secret "new_value" || fail "Failed to store new secret with --force"
OUTPUT=$("$HOME/.cargo/bin/secrets" read new_secret)
[ "$OUTPUT" = "new_value" ] || fail "New secret with --force doesn't match. Got: '$OUTPUT'"
echo "✅ Force flag works for new secrets"

echo "🎉 Update operations test completed successfully!"
