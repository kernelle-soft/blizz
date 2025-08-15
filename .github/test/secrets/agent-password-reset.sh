#!/usr/bin/env bash

# -----------------------------------------------------------------------------
# End-to-end test for secrets agent password reset operations
# Tests changing authentication password while agent is running
# -----------------------------------------------------------------------------

set -euo pipefail
set -x

source "$(dirname "$0")/../isolate.sh"

echo "🔒 Testing secrets agent password reset operations"
echo "=================================================="

# Install kernelle to get the secrets binary
./scripts/install.sh --non-interactive || fail "Install script failed"

# Set up environment variables for testing
export SECRETS_AUTH="original_password_123"
export SECRETS_QUIET="1"

# Verify secrets binary exists
test -f "$HOME/.cargo/bin/secrets" || fail "secrets binary not found after install"

echo "Setting up test data with original password..."

# Start agent with original password
"$HOME/.cargo/bin/secrets" agent start || fail "Failed to start agent with original password"
sleep 2

# Store some test secrets with original password
"$HOME/.cargo/bin/secrets" store original_secret "original_value" || fail "Failed to store secret with original password"
"$HOME/.cargo/bin/secrets" store -g github original_token "github_original" || fail "Failed to store GitHub secret with original password"

# Verify secrets work with original password
OUTPUT=$("$HOME/.cargo/bin/secrets" read original_secret)
[ "$OUTPUT" = "original_value" ] || fail "Original secret not accessible with original password"
echo "✅ Test data stored with original password"

echo "Testing password reset with agent running..."

# Prepare new password
export NEW_SECRETS_AUTH="new_password_456"

# Attempt password reset while agent is running
"$HOME/.cargo/bin/secrets" reset-password || fail "Failed to reset password"
echo "✅ Password reset command completed"

# Update environment to use new password
export SECRETS_AUTH="$NEW_SECRETS_AUTH"

# Give the agent a moment to process the password change
sleep 3

# Verify we can still access secrets with new password
OUTPUT=$("$HOME/.cargo/bin/secrets" read original_secret)
[ "$OUTPUT" = "original_value" ] || fail "Original secret not accessible after password reset"
echo "✅ Original secret accessible with new password"

OUTPUT=$("$HOME/.cargo/bin/secrets" read -g github original_token)
[ "$OUTPUT" = "github_original" ] || fail "GitHub secret not accessible after password reset"
echo "✅ GitHub secret accessible with new password"

# Store new secret with new password
"$HOME/.cargo/bin/secrets" store new_secret "new_value" || fail "Failed to store secret with new password"
OUTPUT=$("$HOME/.cargo/bin/secrets" read new_secret)
[ "$OUTPUT" = "new_value" ] || fail "New secret not accessible with new password"
echo "✅ Can store new secrets with new password"

echo "Testing that old password no longer works..."

# Try to use old password (should fail)
export SECRETS_AUTH="original_password_123"

if "$HOME/.cargo/bin/secrets" read original_secret 2>/dev/null; then
    fail "Old password should not work after reset"
fi
echo "✅ Old password correctly rejected after reset"

if "$HOME/.cargo/bin/secrets" store test_with_old "value" 2>/dev/null; then
    fail "Store with old password should fail after reset"
fi
echo "✅ Store operations fail with old password"

echo "Testing password reset error conditions..."

# Test reset with empty password
export SECRETS_AUTH=""
if "$HOME/.cargo/bin/secrets" agent reset-password 2>/dev/null; then
    fail "Password reset with empty password should fail"
fi
echo "✅ Password reset with empty password correctly fails"

# Test reset with wrong current password
export SECRETS_AUTH="wrong_password"
if "$HOME/.cargo/bin/secrets" agent reset-password 2>/dev/null; then
    fail "Password reset with wrong current password should fail"
fi
echo "✅ Password reset with wrong password correctly fails"

echo "🎉 Agent password reset operations test completed successfully!"
