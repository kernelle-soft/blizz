#!/usr/bin/env bash
# Test Isolation Framework for Blizz
# Source this script at the beginning of any test that needs isolated install/uninstall testing

# Ensure we exit on any error
set -euo pipefail

# Create isolated test environment
export TEST_HOME="$(mktemp -d)"
export TEST_CARGO_HOME="$TEST_HOME/.cargo"
mkdir -p "$TEST_CARGO_HOME/bin"

# Store original environment for restoration if needed
export ORIGINAL_HOME="$HOME"
# export ORIGINAL_CARGO_HOME="${CARGO_HOME:-}"
export ORIGINAL_RUSTUP_HOME="${RUSTUP_HOME:-}"
export ORIGINAL_PATH="$PATH"

# Override environment variables to point to our isolated environment
export HOME="$TEST_HOME"
# export CARGO_HOME="$TEST_CARGO_HOME"
# Keep the host's Rust toolchain to avoid re-downloading
export RUSTUP_HOME="${ORIGINAL_RUSTUP_HOME:-$ORIGINAL_HOME/.rustup}"
export PATH="$TEST_CARGO_HOME/bin:$PATH"

# Helper function to print error messages and exit
fail() { echo "❌ $1" >&2; exit 1; }

# Cleanup function
cleanup_test_isolation() {
    if [ -n "${TEST_HOME:-}" ] && [ -d "$TEST_HOME" ]; then
        echo "🧹 Cleaning up test isolation directory: $TEST_HOME"
        rm -rf "$TEST_HOME"
    fi
    
    # Restore original environment
    export HOME="$ORIGINAL_HOME"
    # export CARGO_HOME="$ORIGINAL_CARGO_HOME"
    export RUSTUP_HOME="$ORIGINAL_RUSTUP_HOME"
    export PATH="$ORIGINAL_PATH"
}

# Set up automatic cleanup on script exit
trap cleanup_test_isolation EXIT

# Helper function to show current test environment (useful for debugging)
show_test_environment() {
    echo "🔧 Test Environment:"
    echo "   TEST_HOME: $TEST_HOME"
    echo "   HOME: $HOME"
    echo "   CARGO_HOME: $CARGO_HOME"
    echo "   PATH: $PATH"
}

# Helper function to verify test isolation is working
verify_test_isolation() {
    if [ "$HOME" != "$TEST_HOME" ]; then
        echo "❌ Error: Test isolation failed - HOME is not set to TEST_HOME"
        exit 1
    fi
    
    # if [ "$CARGO_HOME" != "$TEST_CARGO_HOME" ]; then
    #     echo "❌ Error: Test isolation failed - CARGO_HOME is not set to TEST_CARGO_HOME"
    #     exit 1
    # fi
    
    echo "✅ Test isolation verified"
}

echo "🚀 Test isolation environment set up"
echo "   Isolated HOME: $HOME"
# echo "   Isolated CARGO_HOME: $CARGO_HOME"
verify_test_isolation
