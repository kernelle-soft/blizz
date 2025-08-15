#!/usr/bin/env bash

# -----------------------------------------------------------------------------
# End-to-end test for secrets agent status operations
# Tests status reporting in various agent states
# -----------------------------------------------------------------------------

set -euo pipefail
set -x

source "$(dirname "$0")/../isolate.sh"

echo "🔍 Testing secrets agent status operations"
echo "=========================================="

# Install kernelle to get the secrets binary
./scripts/install.sh --non-interactive || fail "Install script failed"

# Set up environment variables for testing
export SECRETS_AUTH="test_password_123"
export SECRETS_QUIET="1"

# Verify secrets binary exists
test -f "$HOME/.cargo/bin/secrets" || fail "secrets binary not found after install"

echo "Testing status when agent is stopped..."

# Initially, agent should not be running
OUTPUT=$("$HOME/.cargo/bin/secrets" agent status 2>&1 || true)
echo "$OUTPUT" | grep -q "not running" || fail "Expected agent to be stopped initially"
echo "✅ Status correctly reports stopped agent"

echo "Testing status when agent is running..."

# Start the agent
"$HOME/.cargo/bin/secrets" agent start || fail "Failed to start agent"
sleep 2

# Check status when running
OUTPUT=$("$HOME/.cargo/bin/secrets" agent status 2>&1 || true)
echo "$OUTPUT" | grep -q "running" || fail "Status should show running when agent is active"
echo "✅ Status correctly reports running agent"

# Check that status command exits successfully when running
"$HOME/.cargo/bin/secrets" agent status >/dev/null || fail "Status command should return success when running"
echo "✅ Status command returns success when running"

echo "Testing status during agent operations..."

# Store a secret while checking status
"$HOME/.cargo/bin/secrets" store test_key "test_value" || fail "Failed to store secret"

# Status should still show running
OUTPUT=$("$HOME/.cargo/bin/secrets" agent status 2>&1 || true)
echo "$OUTPUT" | grep -q "running" || fail "Status should show running during operations"
echo "✅ Status correctly reports running during operations"

# Multiple rapid status checks
for i in {1..5}; do
    "$HOME/.cargo/bin/secrets" agent status >/dev/null || fail "Status check $i failed"
done
echo "✅ Multiple rapid status checks work"

echo "Testing status during shutdown..."

# Stop the agent
"$HOME/.cargo/bin/secrets" agent stop || fail "Failed to stop agent"

# Give a short moment for shutdown to begin
sleep 1

# Check status during/after shutdown
OUTPUT=$("$HOME/.cargo/bin/secrets" agent status 2>&1 || true)
echo "$OUTPUT" | grep -q "not running" || fail "Status should show stopped after shutdown"
echo "✅ Status correctly reports stopped after shutdown"

echo "Testing status consistency..."

# Start agent again
"$HOME/.cargo/bin/secrets" agent start || fail "Failed to start agent again"
sleep 2

# Check status multiple times - should be consistent
for i in {1..3}; do
    OUTPUT=$("$HOME/.cargo/bin/secrets" agent status 2>&1 || true)
    echo "$OUTPUT" | grep -q "running" || fail "Status check $i inconsistent"
done
echo "✅ Status reports are consistent"

# Stop and check multiple times
"$HOME/.cargo/bin/secrets" agent stop || fail "Failed to stop agent"
sleep 2

for i in {1..3}; do
    OUTPUT=$("$HOME/.cargo/bin/secrets" agent status 2>&1 || true)
    echo "$OUTPUT" | grep -q "not running" || fail "Stopped status check $i inconsistent"
done
echo "✅ Stopped status reports are consistent"

echo "Testing status without authentication..."

# Status should work without authentication
unset SECRETS_AUTH

OUTPUT=$("$HOME/.cargo/bin/secrets" agent status 2>&1 || true)
echo "$OUTPUT" | grep -q "not running" || fail "Status should work without auth"
echo "✅ Status works without authentication"

# Restore auth for cleanup
export SECRETS_AUTH="test_password_123"

echo "🎉 Agent status operations test completed successfully!"
