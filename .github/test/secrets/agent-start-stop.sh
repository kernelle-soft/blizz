#!/usr/bin/env bash

# -----------------------------------------------------------------------------
# End-to-end test for secrets agent start and stop operations
# Tests basic daemon lifecycle management
# -----------------------------------------------------------------------------

set -euo pipefail
set -x

source "$(dirname "$0")/../isolate.sh"

echo "🚀 Testing secrets agent start and stop operations"
echo "=================================================="

# Install kernelle to get the secrets binary
./scripts/install.sh --non-interactive || fail "Install script failed"

# Set up environment variables for testing
export SECRETS_AUTH="test_password_123"
export SECRETS_QUIET="1"

# Verify secrets binary exists
test -f "$HOME/.cargo/bin/secrets" || fail "secrets binary not found after install"

echo "Testing agent start operations..."

# Initially, agent should not be running
OUTPUT=$("$HOME/.cargo/bin/secrets" agent status 2>&1 || true)
echo "$OUTPUT" | grep -q "not running\|No daemon" || fail "Expected agent to be stopped initially"
echo "✅ Confirmed agent is initially stopped"

# Start the agent
"$HOME/.cargo/bin/secrets" agent start || fail "Failed to start agent"
echo "✅ Started secrets agent"

# Give it a moment to fully start
sleep 2

# Check status after start
OUTPUT=$("$HOME/.cargo/bin/secrets" agent status)
echo "$OUTPUT" | grep -q "running\|active" || fail "Agent should be running after start"
echo "✅ Agent status shows running"

# Verify agent files exist
SOCKET_PATH="$HOME/.local/share/kernelle/secrets.sock"
PID_PATH="$HOME/.local/share/kernelle/secrets.pid"

test -S "$SOCKET_PATH" || fail "Socket file should exist when agent is running"
test -f "$PID_PATH" || fail "PID file should exist when agent is running"
echo "✅ Agent files exist when running"

# Try to start again (should handle gracefully)
OUTPUT=$("$HOME/.cargo/bin/secrets" agent start 2>&1 || true)
echo "$OUTPUT" | grep -q "already running\|already started" || fail "Starting already running agent should be handled gracefully"
echo "✅ Starting already running agent handled correctly"

echo "Testing agent stop operations..."

# Stop the agent
"$HOME/.cargo/bin/secrets" agent stop || fail "Failed to stop agent"
echo "✅ Stopped secrets agent"

# Give it a moment to fully stop
sleep 2

# Check status after stop
OUTPUT=$("$HOME/.cargo/bin/secrets" agent status 2>&1 || true)
echo "$OUTPUT" | grep -q "not running\|No daemon" || fail "Agent should be stopped"
echo "✅ Agent status shows stopped"

# Verify cleanup
test ! -S "$SOCKET_PATH" || fail "Socket file should be cleaned up after stop"
test ! -f "$PID_PATH" || fail "PID file should be cleaned up after stop"
echo "✅ Agent files cleaned up after stop"

# Try to stop again (should handle gracefully)
OUTPUT=$("$HOME/.cargo/bin/secrets" agent stop 2>&1 || true)
echo "$OUTPUT" | grep -q "not running\|already stopped" || fail "Stopping already stopped agent should be handled gracefully"
echo "✅ Stopping already stopped agent handled correctly"

echo "Testing multiple start/stop cycles..."

# Start again
"$HOME/.cargo/bin/secrets" agent start || fail "Failed to start agent on second cycle"
sleep 2

OUTPUT=$("$HOME/.cargo/bin/secrets" agent status)
echo "$OUTPUT" | grep -q "running\|active" || fail "Agent should be running on second start"
echo "✅ Second start cycle works"

# Stop again
"$HOME/.cargo/bin/secrets" agent stop || fail "Failed to stop agent on second cycle"
sleep 2

OUTPUT=$("$HOME/.cargo/bin/secrets" agent status 2>&1 || true)
echo "$OUTPUT" | grep -q "not running\|No daemon" || fail "Agent should be stopped on second stop"
echo "✅ Second stop cycle works"

# Third cycle
"$HOME/.cargo/bin/secrets" agent start || fail "Failed to start agent on third cycle"
sleep 2
"$HOME/.cargo/bin/secrets" agent stop || fail "Failed to stop agent on third cycle"
sleep 2
echo "✅ Multiple start/stop cycles work correctly"

echo "🎉 Agent start and stop operations test completed successfully!"
