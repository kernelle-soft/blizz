#!/usr/bin/env bash

# -----------------------------------------------------------------------------
# End-to-end test for secrets delete operations
# Tests deleting individual secrets and verifying cleanup
# -----------------------------------------------------------------------------

set -euo pipefail
set -x

source "$(dirname "$0")/../isolate.sh"

echo "🗑️ Testing secrets delete operations"
echo "===================================="

# Install kernelle to get the secrets binary
./scripts/install.sh --non-interactive || fail "Install script failed"

# Set up environment variables for testing
export SECRETS_AUTH="test_password_123"
export SECRETS_QUIET="1"

# Verify secrets binary exists
test -f "$HOME/.cargo/bin/secrets" || fail "secrets binary not found after install"

echo "Setting up test data..."

# Store test secrets for deletion
"$HOME/.cargo/bin/secrets" store api_key "api_value" || fail "Failed to store api_key"
"$HOME/.cargo/bin/secrets" store db_password "db_pass" || fail "Failed to store db_password"
"$HOME/.cargo/bin/secrets" store -g github token "github_token" || fail "Failed to store GitHub token"
"$HOME/.cargo/bin/secrets" store -g aws access_key "aws_key" || fail "Failed to store AWS key"
"$HOME/.cargo/bin/secrets" store -g aws secret_key "aws_secret" || fail "Failed to store AWS secret"

echo "Testing delete operations..."

# Try to delete without force flag (should fail)
if "$HOME/.cargo/bin/secrets" delete api_key 2>/dev/null; then
    fail "Delete without --force should fail"
fi
echo "✅ Delete without --force correctly fails"

# Delete specific secret with force
"$HOME/.cargo/bin/secrets" delete --force api_key || fail "Failed to delete specific secret"
echo "✅ Deleted specific secret"

# Verify secret is gone
if "$HOME/.cargo/bin/secrets" read api_key 2>/dev/null; then
    fail "Deleted secret still accessible"
fi
echo "✅ Verified deleted secret is inaccessible"

# Verify other secrets still exist
OUTPUT=$("$HOME/.cargo/bin/secrets" read db_password)
[ "$OUTPUT" = "db_pass" ] || fail "Other secret was incorrectly affected by deletion"
echo "✅ Other secrets unaffected by deletion"

# Delete secret from specific group
"$HOME/.cargo/bin/secrets" delete --force -g github token || fail "Failed to delete secret from specific group"
echo "✅ Deleted secret from specific group"

# Verify github secret is gone
if "$HOME/.cargo/bin/secrets" read -g github token 2>/dev/null; then
    fail "Deleted github secret still accessible"
fi
echo "✅ Verified github secret is deleted"

# Verify AWS secrets still exist
OUTPUT=$("$HOME/.cargo/bin/secrets" read -g aws access_key)
[ "$OUTPUT" = "aws_key" ] || fail "AWS access_key was incorrectly affected"
OUTPUT=$("$HOME/.cargo/bin/secrets" read -g aws secret_key)
[ "$OUTPUT" = "aws_secret" ] || fail "AWS secret_key was incorrectly affected"
echo "✅ AWS secrets unaffected by GitHub deletion"

# Test force flag in different position
"$HOME/.cargo/bin/secrets" delete db_password --force || fail "Failed to delete with --force at end"
if "$HOME/.cargo/bin/secrets" read db_password 2>/dev/null; then
    fail "Secret with --force at end still accessible"
fi
echo "✅ Force flag works in different position"

# Test deleting non-existent secret
if "$HOME/.cargo/bin/secrets" delete --force nonexistent 2>/dev/null; then
    fail "Deleting non-existent secret should fail"
fi
echo "✅ Deleting non-existent secret correctly fails"

echo "🎉 Delete operations test completed successfully!"
