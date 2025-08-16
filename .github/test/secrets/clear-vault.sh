#!/usr/bin/env bash

# -----------------------------------------------------------------------------
# End-to-end test for secrets clear vault operations
# Tests clearing entire vault and verifying complete cleanup
# -----------------------------------------------------------------------------

set -euo pipefail
set -x

source "$(dirname "$0")/../isolate.sh"

echo "🧹 Testing secrets clear vault operations"
echo "========================================="

# Install kernelle to get the secrets binary
./scripts/install.sh --non-interactive || fail "Install script failed"

# Set up environment variables for testing
export SECRETS_AUTH="test_password_123"
export SECRETS_QUIET="1"
export KERNELLE_HOME="$HOME/.kernelle"  # Ensure consistent path for CLI and daemon

# Verify secrets binary exists
test -f "$HOME/.cargo/bin/secrets" || fail "secrets binary not found after install"

echo "Setting up test data..."

# Store multiple secrets across different groups
"$HOME/.cargo/bin/secrets" store api_key "api_value" || fail "Failed to store api_key"
"$HOME/.cargo/bin/secrets" store db_password "db_pass" || fail "Failed to store db_password"
"$HOME/.cargo/bin/secrets" store config_value "config_data" || fail "Failed to store config_value"
"$HOME/.cargo/bin/secrets" store -g github token "github_token" || fail "Failed to store GitHub token"
"$HOME/.cargo/bin/secrets" store -g github webhook "webhook_secret" || fail "Failed to store GitHub webhook"
"$HOME/.cargo/bin/secrets" store -g aws access_key "aws_key" || fail "Failed to store AWS key"
"$HOME/.cargo/bin/secrets" store -g aws secret_key "aws_secret" || fail "Failed to store AWS secret"
"$HOME/.cargo/bin/secrets" store -g production database_url "prod_db_url" || fail "Failed to store production secret"

echo "Testing clear vault operations..."

# Verify we have secrets before clear
OUTPUT=$("$HOME/.cargo/bin/secrets" list)
echo "$OUTPUT" | grep -q "general: 3 secrets" || fail "Should have 3 general secrets before clear"
echo "$OUTPUT" | grep -q "github: 2 secrets" || fail "Should have 2 github secrets before clear"
echo "$OUTPUT" | grep -q "aws: 2 secrets" || fail "Should have 2 aws secrets before clear"
echo "$OUTPUT" | grep -q "production: 1 secret" || fail "Should have 1 production secret before clear"
echo "✅ Verified vault has secrets before clear"

# Try to clear without force flag (should fail)
if "$HOME/.cargo/bin/secrets" clear 2>/dev/null; then
    fail "Clear without --force should fail"
fi
echo "✅ Clear without --force correctly fails"

# Clear all secrets with force
"$HOME/.cargo/bin/secrets" clear --force || fail "Failed to clear vault"
echo "✅ Cleared entire vault"

# Verify vault is empty
OUTPUT=$("$HOME/.cargo/bin/secrets" list)
echo "$OUTPUT" | grep -q "vault is empty" || fail "Vault should be empty after clear"
echo "✅ Verified vault is empty"

# Verify individual secrets are gone
if "$HOME/.cargo/bin/secrets" read api_key 2>/dev/null; then
    fail "api_key still accessible after vault clear"
fi
if "$HOME/.cargo/bin/secrets" read db_password 2>/dev/null; then
    fail "db_password still accessible after vault clear"
fi
if "$HOME/.cargo/bin/secrets" read -g github token 2>/dev/null; then
    fail "github token still accessible after vault clear"
fi
if "$HOME/.cargo/bin/secrets" read -g aws access_key 2>/dev/null; then
    fail "aws access_key still accessible after vault clear"
fi
if "$HOME/.cargo/bin/secrets" read -g production database_url 2>/dev/null; then
    fail "production secret still accessible after vault clear"
fi
echo "✅ Verified all secrets are gone after clear"

# Test that we can store new secrets after clear
"$HOME/.cargo/bin/secrets" store new_secret "new_value" || fail "Failed to store secret after clear"
OUTPUT=$("$HOME/.cargo/bin/secrets" read new_secret)
[ "$OUTPUT" = "new_value" ] || fail "New secret after clear doesn't work"
echo "✅ Can store new secrets after vault clear"

# Test clearing empty vault
"$HOME/.cargo/bin/secrets" clear --force || fail "Failed to clear empty vault"
OUTPUT=$("$HOME/.cargo/bin/secrets" list)
echo "$OUTPUT" | grep -q "vault is empty" || fail "Vault should still be empty"
echo "✅ Clearing empty vault works correctly"

echo "🎉 Clear vault operations test completed successfully!"
