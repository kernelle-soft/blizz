#!/usr/bin/env bash

# -----------------------------------------------------------------------------
# End-to-end test for secrets error condition handling
# Tests various error scenarios and proper error reporting
# -----------------------------------------------------------------------------

set -euo pipefail
set -x

source "$(dirname "$0")/../isolate.sh"

echo "❌ Testing secrets error condition handling"
echo "==========================================="

# Install kernelle to get the secrets binary
./scripts/install.sh --non-interactive || fail "Install script failed"

# Set up environment variables for testing
export SECRETS_AUTH="test_password_123"
export SECRETS_QUIET="1"
export KERNELLE_HOME="$HOME/.kernelle"  # Ensure consistent path for CLI and daemon

# Verify secrets binary exists
test -f "$HOME/.cargo/bin/secrets" || fail "secrets binary not found after install"

echo "Testing error conditions..."

# Test reading non-existent secret
if "$HOME/.cargo/bin/secrets" read nonexistent_secret 2>/dev/null; then
    fail "Reading non-existent secret should fail"
fi
echo "✅ Reading non-existent secret correctly fails"

# Test reading from non-existent group
if "$HOME/.cargo/bin/secrets" read -g nonexistent_group some_key 2>/dev/null; then
    fail "Reading from non-existent group should fail"
fi
echo "✅ Reading from non-existent group correctly fails"

# Test deleting non-existent secret
if "$HOME/.cargo/bin/secrets" delete --force nonexistent_secret 2>/dev/null; then
    fail "Deleting non-existent secret should fail"
fi
echo "✅ Deleting non-existent secret correctly fails"

# Test deleting from non-existent group
if "$HOME/.cargo/bin/secrets" delete --force -g nonexistent_group some_key 2>/dev/null; then
    fail "Deleting from non-existent group should fail"
fi
echo "✅ Deleting from non-existent group correctly fails"

# Test invalid command
if "$HOME/.cargo/bin/secrets" invalid_command 2>/dev/null; then
    fail "Invalid command should fail"
fi
echo "✅ Invalid command correctly fails"

# Test missing arguments
if "$HOME/.cargo/bin/secrets" store 2>/dev/null; then
    fail "Store without arguments should fail"
fi
echo "✅ Store without arguments correctly fails"

if "$HOME/.cargo/bin/secrets" read 2>/dev/null; then
    fail "Read without arguments should fail"
fi
echo "✅ Read without arguments correctly fails"

if "$HOME/.cargo/bin/secrets" delete 2>/dev/null; then
    fail "Delete without arguments should fail"
fi
echo "✅ Delete without arguments correctly fails"

# Test operations with wrong password
export SECRETS_AUTH="wrong_password"

# Store a secret first with correct password
export SECRETS_AUTH="test_password_123"
"$HOME/.cargo/bin/secrets" store test_key "test_value" || fail "Failed to store test secret"

# Now try with wrong password
export SECRETS_AUTH="wrong_password"

if "$HOME/.cargo/bin/secrets" read test_key 2>/dev/null; then
    fail "Operations with wrong password should fail"
fi
echo "✅ Wrong password correctly fails"

if "$HOME/.cargo/bin/secrets" list 2>/dev/null; then
    fail "List with wrong password should fail"
fi
echo "✅ List with wrong password correctly fails"

if "$HOME/.cargo/bin/secrets" store another_key "value" 2>/dev/null; then
    fail "Store with wrong password should fail"
fi
echo "✅ Store with wrong password correctly fails"

# Test empty password
export SECRETS_AUTH=""

if "$HOME/.cargo/bin/secrets" read test_key 2>/dev/null; then
    fail "Operations with empty password should fail"
fi
echo "✅ Empty password correctly fails"

# Restore correct password for cleanup
export SECRETS_AUTH="test_password_123"

# Test invalid group names (if any restrictions exist)
# Note: This depends on implementation - some characters might be allowed
if "$HOME/.cargo/bin/secrets" store -g "group with spaces" key "value" 2>/dev/null; then
    echo "ℹ️ Group names with spaces are allowed"
else
    echo "✅ Group names with spaces correctly restricted"
fi

echo "🎉 Error condition handling test completed successfully!"
