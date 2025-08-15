#!/usr/bin/env bash

# -----------------------------------------------------------------------------
# End-to-end test for secrets list operations
# Tests listing secrets with various filters and formats
# -----------------------------------------------------------------------------

set -euo pipefail
set -x

source "$(dirname "$0")/../isolate.sh"

echo "📋 Testing secrets list operations"
echo "=================================="

# Install kernelle to get the secrets binary
./scripts/install.sh --non-interactive || fail "Install script failed"

# Set up environment variables for testing
export SECRETS_AUTH="test_password_123"
export SECRETS_QUIET="1"

# Verify secrets binary exists
test -f "$HOME/.cargo/bin/secrets" || fail "secrets binary not found after install"

echo "Setting up test data..."

# Store test secrets in multiple groups
"$HOME/.cargo/bin/secrets" store api_key "secret_value_123" || fail "Failed to store general secret"
"$HOME/.cargo/bin/secrets" store database_url "db_url_456" || fail "Failed to store another general secret"
"$HOME/.cargo/bin/secrets" store -g github token "github_token_789" || fail "Failed to store GitHub secret"
"$HOME/.cargo/bin/secrets" store -g github webhook_secret "webhook_abc" || fail "Failed to store GitHub webhook"
"$HOME/.cargo/bin/secrets" store -g aws access_key "aws_key_def" || fail "Failed to store AWS secret"

echo "Testing list operations..."

# Test basic list functionality
OUTPUT=$("$HOME/.cargo/bin/secrets" list)
echo "$OUTPUT" | grep -q "general: 2 secrets" || fail "List output doesn't show correct general group count"
echo "$OUTPUT" | grep -q "github: 2 secrets" || fail "List output doesn't show correct github group count"
echo "$OUTPUT" | grep -q "aws: 1 secret" || fail "List output doesn't show correct aws group count"
echo "✅ Basic list shows correct group counts"

# Test list with keys
OUTPUT=$("$HOME/.cargo/bin/secrets" list --keys)
echo "$OUTPUT" | grep -q "general/api_key" || fail "List with keys doesn't show general/api_key"
echo "$OUTPUT" | grep -q "general/database_url" || fail "List with keys doesn't show general/database_url"
echo "$OUTPUT" | grep -q "github/token" || fail "List with keys doesn't show github/token"
echo "$OUTPUT" | grep -q "github/webhook_secret" || fail "List with keys doesn't show github/webhook_secret"
echo "$OUTPUT" | grep -q "aws/access_key" || fail "List with keys doesn't show aws/access_key"
echo "✅ List with keys shows individual secrets"

# Test list with group filter
OUTPUT=$("$HOME/.cargo/bin/secrets" list -g github)
echo "$OUTPUT" | grep -q "github: 2 secrets" || fail "Group filtered list doesn't work"
echo "$OUTPUT" | grep -v -q "general\|aws" || fail "Group filtered list shows other groups"
echo "✅ Group filtered list works correctly"

# Test list with group filter and keys
OUTPUT=$("$HOME/.cargo/bin/secrets" list -g aws --keys)
echo "$OUTPUT" | grep -q "aws/access_key" || fail "Group filtered list with keys doesn't show aws/access_key"
echo "$OUTPUT" | grep -v -q "general\|github" || fail "Group filtered list with keys shows other groups"
echo "✅ Group filtered list with keys works correctly"

# Test listing non-existent group
OUTPUT=$("$HOME/.cargo/bin/secrets" list -g nonexistent)
echo "$OUTPUT" | grep -q "no secrets found for group: nonexistent" || fail "Should report no secrets for non-existent group"
echo "✅ Listing non-existent group handled correctly"

echo "🎉 List operations test completed successfully!"
