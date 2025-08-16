#!/usr/bin/env bash

# -----------------------------------------------------------------------------
# End-to-end test for secrets agent file cleanup operations
# Tests that agent properly creates and cleans up socket and PID files
# -----------------------------------------------------------------------------

set -euo pipefail
set -x

source "$(dirname "$0")/../isolate.sh"

echo "🧹 Testing secrets agent file cleanup operations"
echo "================================================"

# Install kernelle to get the secrets binary
./scripts/install.sh --non-interactive || fail "Install script failed"

# Set up environment variables for testing
export SECRETS_AUTH="test_password_123"
export SECRETS_QUIET="1"
export KERNELLE_HOME="$HOME/.kernelle"  # Ensure consistent path for CLI and daemon

# Verify secrets binary exists
test -f "$HOME/.cargo/bin/secrets" || fail "secrets binary not found after install"

# Define expected file paths (using the isolated HOME from test environment)
SOCKET_PATH="$HOME/.kernelle/persistent/keeper/keeper.sock"
PID_PATH="$HOME/.kernelle/persistent/keeper/keeper.pid"
DATA_DIR="$HOME/.kernelle/persistent/keeper"

echo "🔧 Test Environment Paths:"
echo "   HOME: $HOME"
echo "   Expected socket: $SOCKET_PATH"
echo "   Expected PID: $PID_PATH"
echo "   Data directory: $DATA_DIR"

echo "Testing initial state (no files should exist)..."

# Ensure agent is stopped initially
"$HOME/.cargo/bin/secrets" agent stop >/dev/null 2>&1 || true
sleep 1

# Verify no agent files exist initially
test ! -S "$SOCKET_PATH" || fail "Socket file should not exist initially"
test ! -f "$PID_PATH" || fail "PID file should not exist initially"
echo "✅ No agent files exist initially"

echo "Testing file creation on agent start..."

# Start the agent
"$HOME/.cargo/bin/secrets" agent start || fail "Failed to start agent"
sleep 2

# Verify agent files are created
test -S "$SOCKET_PATH" || fail "Socket file should exist when agent is running"
test -f "$PID_PATH" || fail "PID file should exist when agent is running"
echo "✅ Agent files created on start"

# Verify socket is actually a socket
if [ ! -S "$SOCKET_PATH" ]; then
    fail "Socket path exists but is not a socket"
fi
echo "✅ Socket file is correct type"

# Verify PID file contains valid PID
if [ -f "$PID_PATH" ]; then
    PID_CONTENT=$(cat "$PID_PATH")
    if ! echo "$PID_CONTENT" | grep -E '^[0-9]+$' >/dev/null; then
        fail "PID file should contain numeric PID"
    fi
    echo "✅ PID file contains valid PID: $PID_CONTENT"
fi

# Verify the PID is actually running
if [ -f "$PID_PATH" ]; then
    PID_CONTENT=$(cat "$PID_PATH")
    if ! ps -p "$PID_CONTENT" >/dev/null 2>&1; then
        fail "PID from file is not running"
    fi
    echo "✅ PID from file corresponds to running process"
fi

echo "Testing file permissions..."

# Check socket permissions
SOCKET_PERMS=$(stat -c "%a" "$SOCKET_PATH" 2>/dev/null || stat -f "%Lp" "$SOCKET_PATH" 2>/dev/null || echo "600")
if [ "$SOCKET_PERMS" != "600" ] && [ "$SOCKET_PERMS" != "700" ]; then
    echo "ℹ️ Socket permissions: $SOCKET_PERMS (expected 600 or 700)"
else
    echo "✅ Socket has appropriate permissions: $SOCKET_PERMS"
fi

# Check PID file permissions
PID_PERMS=$(stat -c "%a" "$PID_PATH" 2>/dev/null || stat -f "%Lp" "$PID_PATH" 2>/dev/null || echo "600")
if [ "$PID_PERMS" != "600" ] && [ "$PID_PERMS" != "644" ]; then
    echo "ℹ️ PID file permissions: $PID_PERMS (expected 600 or 644)"
else
    echo "✅ PID file has appropriate permissions: $PID_PERMS"
fi

echo "Testing files persist during operations..."

# Perform some operations
"$HOME/.cargo/bin/secrets" store test_key "test_value" || fail "Failed to store secret"
"$HOME/.cargo/bin/secrets" list >/dev/null || fail "Failed to list secrets"

# Verify files still exist
test -S "$SOCKET_PATH" || fail "Socket file should persist during operations"
test -f "$PID_PATH" || fail "PID file should persist during operations"
echo "✅ Agent files persist during operations"

echo "Testing file cleanup on agent stop..."

# Stop the agent
"$HOME/.cargo/bin/secrets" agent stop || fail "Failed to stop agent"
sleep 2

# Verify files are cleaned up
test ! -S "$SOCKET_PATH" || fail "Socket file should be cleaned up after stop"
test ! -f "$PID_PATH" || fail "PID file should be cleaned up after stop"
echo "✅ Agent files cleaned up on stop"

echo "Testing cleanup after restart..."

# Start and restart cycle
"$HOME/.cargo/bin/secrets" agent start || fail "Failed to start for restart test"
sleep 2

# Verify files exist
test -S "$SOCKET_PATH" || fail "Socket should exist after start"
test -f "$PID_PATH" || fail "PID should exist after start"

# Restart
"$HOME/.cargo/bin/secrets" agent restart || fail "Failed to restart agent"
sleep 3

# Verify files still exist after restart
test -S "$SOCKET_PATH" || fail "Socket should exist after restart"
test -f "$PID_PATH" || fail "PID should exist after restart"
echo "✅ Agent files handled correctly during restart"

# Final stop
"$HOME/.cargo/bin/secrets" agent stop || fail "Failed to stop after restart test"
sleep 2

# Verify cleanup after restart cycle
test ! -S "$SOCKET_PATH" || fail "Socket should be cleaned up after final stop"
test ! -f "$PID_PATH" || fail "PID should be cleaned up after final stop"
echo "✅ Agent files cleaned up after restart cycle"

echo "Testing cleanup after crash simulation..."

# Start agent
"$HOME/.cargo/bin/secrets" agent start || fail "Failed to start for crash simulation"
sleep 2

# Get the PID
if [ -f "$PID_PATH" ]; then
    AGENT_PID=$(cat "$PID_PATH")
    echo "Agent PID: $AGENT_PID"
    
    # Simulate crash by killing the process directly
    kill -9 "$AGENT_PID" 2>/dev/null || echo "Process already gone"
    sleep 2
    
    # Files might still exist after crash
    if [ -S "$SOCKET_PATH" ] || [ -f "$PID_PATH" ]; then
        echo "ℹ️ Files exist after crash (normal behavior)"
        
        # Try to start agent again (should handle stale files)
        "$HOME/.cargo/bin/secrets" agent start || fail "Failed to start after crash"
        sleep 2
        
        # Verify new files are created
        test -S "$SOCKET_PATH" || fail "Socket should exist after restart from crash"
        test -f "$PID_PATH" || fail "PID should exist after restart from crash"
        echo "✅ Agent handles stale files correctly"
        
        # Clean stop
        "$HOME/.cargo/bin/secrets" agent stop || fail "Failed to clean stop after crash test"
        sleep 2
    else
        echo "✅ Files cleaned up immediately after crash"
    fi
fi

echo "Testing directory creation..."

# Remove the data directory and test recreation
if [ -d "$DATA_DIR" ]; then
    rm -rf "$DATA_DIR" || fail "Failed to remove data directory for test"
fi

# Start agent (should recreate directory)
"$HOME/.cargo/bin/secrets" agent start || fail "Failed to start agent after removing directory"
sleep 2

# Verify directory and files are created
test -d "$DATA_DIR" || fail "Data directory should be recreated"
test -S "$SOCKET_PATH" || fail "Socket should be created in new directory"
test -f "$PID_PATH" || fail "PID should be created in new directory"
echo "✅ Agent recreates necessary directories"

# Final cleanup
"$HOME/.cargo/bin/secrets" agent stop || fail "Failed to final stop"

echo "🎉 Agent file cleanup operations test completed successfully!"
