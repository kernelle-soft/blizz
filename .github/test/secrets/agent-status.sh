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
export KERNELLE_HOME="$HOME/.kernelle"  # Ensure consistent path for CLI and daemon

# Verify secrets binary exists
test -f "$HOME/.cargo/bin/secrets" || fail "secrets binary not found after install"

echo "🔧 CI Environment Debugging"
echo "==========================="

# Debug environment and filesystem
echo "🔧 Environment Info:"
echo "   KERNELLE_HOME: $KERNELLE_HOME"
echo "   USER: $USER"
echo "   HOME: $HOME"
echo "   PWD: $PWD"
echo "   TMPDIR: ${TMPDIR:-/tmp}"
echo "   Expected socket: $KERNELLE_HOME/persistent/keeper/keeper.sock"
echo "   Expected PID: $KERNELLE_HOME/persistent/keeper/keeper.pid"

# Show filesystem info
echo "🔧 Filesystem Info:"
df -T "$HOME" || echo "Failed to get filesystem info for HOME"
stat "$KERNELLE_HOME" 2>/dev/null || echo "KERNELLE_HOME doesn't exist yet"

# Ensure the directory structure exists
mkdir -p "$KERNELLE_HOME/persistent/keeper"
echo "   Created directory structure"

# Check permissions
echo "🔧 Directory Permissions:"
ls -la "$KERNELLE_HOME"
ls -la "$KERNELLE_HOME/persistent"
ls -la "$KERNELLE_HOME/persistent/keeper"

# Check for any existing keeper processes
echo "🔧 Existing processes:"
ps aux | grep keeper || echo "No keeper processes found"

# Test socket creation capability (basic sanity check)
echo "🔧 Testing socket creation capability:"
TEST_SOCKET="$KERNELLE_HOME/persistent/keeper/test.sock"
if command -v nc >/dev/null 2>&1; then
    timeout 5 nc -U -l "$TEST_SOCKET" &
    NC_PID=$!
    sleep 1
    if [ -S "$TEST_SOCKET" ]; then
        echo "✅ Socket creation works"
        kill $NC_PID 2>/dev/null || true
        rm -f "$TEST_SOCKET"
    else
        echo "❌ Socket creation failed"
        kill $NC_PID 2>/dev/null || true
    fi
else
    echo "nc not available for socket test"
fi

echo "Testing status when agent is stopped..."

# Initially, agent should not be running
OUTPUT=$("$HOME/.cargo/bin/secrets" agent status 2>&1 || true)
echo "$OUTPUT" | grep -q "not running" || fail "Expected agent to be stopped initially"
echo "✅ Status correctly reports stopped agent"

echo "Testing status when agent is running..."

# Add debugging before starting the agent
echo "🔧 About to start agent with full debugging..."
echo "   Command: $HOME/.cargo/bin/secrets agent start"
echo "   Environment:"
echo "     KERNELLE_HOME=$KERNELLE_HOME"
echo "     SECRETS_AUTH=$SECRETS_AUTH"
echo "     SECRETS_QUIET=$SECRETS_QUIET"

# Check if socket exists before operation
if [ -S "$KERNELLE_HOME/persistent/keeper/keeper.sock" ]; then
    echo "   ⚠️  Socket already exists before starting agent"
    ls -la "$KERNELLE_HOME/persistent/keeper/keeper.sock"
else
    echo "   ✅ No socket exists before starting agent (expected)"
fi

# Start the agent with debugging
echo "🔧 Starting agent with strace debugging..."
if command -v strace >/dev/null 2>&1; then
    strace -f -e trace=socket,connect,bind,listen -o /tmp/agent-start-trace.log "$HOME/.cargo/bin/secrets" agent start 2>&1 || {
        START_EXIT_CODE=$?
        echo "❌ Agent start failed with exit code: $START_EXIT_CODE"
        echo "🔧 Debugging agent start failure:"
        echo "   Last 30 lines of strace output:"
        tail -30 /tmp/agent-start-trace.log || echo "No strace output"
        echo "   Socket status after failure:"
        ls -la "$KERNELLE_HOME/persistent/keeper/" || echo "Directory doesn't exist"
        echo "   Process status after failure:"
        ps aux | grep -E "(secrets|keeper)" || echo "No related processes"
        fail "Failed to start agent"
    }
else
    "$HOME/.cargo/bin/secrets" agent start || {
        START_EXIT_CODE=$?
        echo "❌ Agent start failed with exit code: $START_EXIT_CODE"
        echo "🔧 Debugging agent start failure:"
        echo "   Socket status after failure:"
        ls -la "$KERNELLE_HOME/persistent/keeper/" || echo "Directory doesn't exist"
        echo "   Process status after failure:"
        ps aux | grep -E "(secrets|keeper)" || echo "No related processes"
        fail "Failed to start agent"
    }
fi

# Wait and verify socket creation
echo "🔧 Verifying socket creation..."
for i in {1..10}; do
    if [ -S "$KERNELLE_HOME/persistent/keeper/keeper.sock" ]; then
        echo "✅ Socket created successfully on attempt $i"
        ls -la "$KERNELLE_HOME/persistent/keeper/keeper.sock"
        break
    else
        echo "   Attempt $i: Socket not found, waiting..."
        ls -la "$KERNELLE_HOME/persistent/keeper/" || echo "Directory doesn't exist"
        sleep 1
    fi
    if [ $i -eq 10 ]; then
        echo "❌ Socket never appeared after agent start"
        fail "Socket not created after agent start"
    fi
done

sleep 2

# Check status when running
OUTPUT=$("$HOME/.cargo/bin/secrets" agent status 2>&1 || true)
echo "$OUTPUT" | grep -q "running" || fail "Status should show running when agent is active"
echo "✅ Status correctly reports running agent"

# Check that status command exits successfully when running
"$HOME/.cargo/bin/secrets" agent status >/dev/null || fail "Status command should return success when running"
echo "✅ Status command returns success when running"

echo "Testing status during agent operations..."

# Debug before storing secret
echo "🔧 About to store secret with socket debugging..."
echo "   Verifying socket exists before store operation:"
if [ -S "$KERNELLE_HOME/persistent/keeper/keeper.sock" ]; then
    echo "   ✅ Socket exists"
    ls -la "$KERNELLE_HOME/persistent/keeper/keeper.sock"
    # Test socket connectivity
    echo "   🔧 Testing socket connectivity..."
    if timeout 5 bash -c "echo 'test' | nc -U '$KERNELLE_HOME/persistent/keeper/keeper.sock'" 2>/dev/null; then
        echo "   ✅ Socket appears to be responsive"
    else
        echo "   ⚠️  Socket exists but may not be responsive"
    fi
else
    echo "   ❌ Socket doesn't exist before store operation"
    ls -la "$KERNELLE_HOME/persistent/keeper/" || echo "Directory doesn't exist"
    fail "Socket missing before store operation"
fi

# Store a secret while checking status with enhanced debugging
echo "🔧 Storing secret with detailed monitoring..."
if command -v strace >/dev/null 2>&1; then
    strace -f -e trace=socket,connect,bind -o /tmp/store-trace.log "$HOME/.cargo/bin/secrets" store test_key "test_value" 2>&1 || {
        STORE_EXIT_CODE=$?
        echo "❌ Store operation failed with exit code: $STORE_EXIT_CODE"
        echo "🔧 Store failure debugging:"
        echo "   Last 20 lines of strace output:"
        tail -20 /tmp/store-trace.log || echo "No strace output"
        echo "   Socket status after store failure:"
        ls -la "$KERNELLE_HOME/persistent/keeper/" || echo "Directory doesn't exist"
        echo "   Process status after store failure:"
        ps aux | grep -E "(secrets|keeper)" || echo "No related processes"
        fail "Failed to store secret"
    }
else
    "$HOME/.cargo/bin/secrets" store test_key "test_value" || {
        STORE_EXIT_CODE=$?
        echo "❌ Store operation failed with exit code: $STORE_EXIT_CODE"
        echo "🔧 Store failure debugging:"
        echo "   Socket status after store failure:"
        ls -la "$KERNELLE_HOME/persistent/keeper/" || echo "Directory doesn't exist"
        echo "   Process status after store failure:"
        ps aux | grep -E "(secrets|keeper)" || echo "No related processes"
        fail "Failed to store secret"
    }
fi

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
