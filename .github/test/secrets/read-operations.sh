#!/usr/bin/env bash

# -----------------------------------------------------------------------------
# End-to-end test for secrets read operations
# Tests reading secrets from default and specific groups
# -----------------------------------------------------------------------------

set -euo pipefail
set -x

source "$(dirname "$0")/../isolate.sh"

echo "📖 Testing secrets read operations"
echo "=================================="

# Install kernelle to get the secrets binary
./scripts/install.sh --non-interactive || fail "Install script failed"

# Set up environment variables for testing
export SECRETS_AUTH="test_password_123"
export SECRETS_QUIET="1"

# Verify secrets binary exists
test -f "$HOME/.cargo/bin/secrets" || fail "secrets binary not found after install"

echo "Setting up test data..."

# Store test secrets
"$HOME/.cargo/bin/secrets" store api_key "secret_value_123" || fail "Failed to store test secret"
"$HOME/.cargo/bin/secrets" store -g github token "github_token_456" || fail "Failed to store GitHub secret"
"$HOME/.cargo/bin/secrets" store -g aws access_key "aws_key_789" || fail "Failed to store AWS secret"

echo "Testing read operations..."

# Read secret from default group
OUTPUT=$("$HOME/.cargo/bin/secrets" read api_key)
[ "$OUTPUT" = "secret_value_123" ] || fail "Read from default group failed. Got: '$OUTPUT'"
echo "✅ Read secret from default group"

# Read secret from specific group
OUTPUT=$("$HOME/.cargo/bin/secrets" read -g github token)
[ "$OUTPUT" = "github_token_456" ] || fail "Read from github group failed. Got: '$OUTPUT'"
echo "✅ Read secret from specific group"

# Read secret from another group
OUTPUT=$("$HOME/.cargo/bin/secrets" read -g aws access_key)
[ "$OUTPUT" = "aws_key_789" ] || fail "Read from aws group failed. Got: '$OUTPUT'"
echo "✅ Read secret from another group"

# Test reading with group flag in different position
OUTPUT=$("$HOME/.cargo/bin/secrets" read access_key -g aws)
[ "$OUTPUT" = "aws_key_789" ] || fail "Read with group flag after key failed. Got: '$OUTPUT'"
echo "✅ Read with group flag in different position"

echo "🎉 Read operations test completed successfully!"
