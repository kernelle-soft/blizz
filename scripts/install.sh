#!/usr/bin/env bash
set -euo pipefail

# Kernelle Installation Script - Phase 1
# This script installs Kernelle and sets up the basic lifecycle infrastructure

# Show usage information
show_install_usage() {
    echo "Usage: $0 [--non-interactive]"
    echo ""
    echo "Options:"
    echo "  --non-interactive    Skip interactive prompts (for CI/automation)"
    echo "  --help, -h          Show this help message"
}

# Process a single command line option
process_install_option() {
    case $1 in
        --non-interactive)
            NON_INTERACTIVE=true
            ;;
        --help|-h)
            show_install_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_install_usage
            exit 1
            ;;
    esac
}

# Parse command line arguments  
parse_install_arguments() {
    NON_INTERACTIVE=false
    
    while [[ $# -gt 0 ]]; do
        process_install_option "$1"
        shift
    done
}

# Parse arguments
parse_install_arguments "$@"

echo "🚀 Installing Kernelle..."

# Configuration
KERNELLE_HOME="${KERNELLE_HOME:-$HOME/.kernelle}"

# Create directories
echo "📁 Creating directories..."
mkdir -p "$KERNELLE_HOME"

# For Phase 1, we'll assume we're running from the source directory
# In Phase 2+, this would clone from a repo
# Portable way to get script directory (works in bash and zsh)
if [ -n "${BASH_SOURCE[0]}" ]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
else
    # zsh and other shells
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
fi
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🔨 Installing Kernelle tools..."
echo "Script directory: $SCRIPT_DIR"
echo "Repository root: $REPO_ROOT"
echo "Current directory: $(pwd)"
echo "Looking for Cargo.toml at: $REPO_ROOT/Cargo.toml"

if [ ! -f "$REPO_ROOT/Cargo.toml" ]; then
    echo "❌ Error: Cargo.toml not found at $REPO_ROOT/Cargo.toml"
    echo "Contents of $REPO_ROOT:"
    ls -la "$REPO_ROOT"
    exit 1
fi

cd "$REPO_ROOT"

echo "📦 Installing binaries..."
# Install all binary crates using cargo install --path
for crate_dir in crates/*/; do
    if [ -d "$crate_dir" ]; then
        crate=$(basename "$crate_dir")
        # Check if this crate has binary targets by looking for [[bin]] in Cargo.toml
        if grep -q '\[\[bin\]\]' "$crate_dir/Cargo.toml"; then
            echo "  Installing: $crate"
            cargo install --path "$crate_dir" --force
        else
            echo "  Skipped: $crate (library only)"
        fi
    fi
done

echo "📋 Setting up workflows..."
# Copy .cursor rules to ~/.kernelle/.cursor
if [ -d "$REPO_ROOT/.cursor" ]; then
    cp -r "$REPO_ROOT/.cursor" "$KERNELLE_HOME/"
else
    echo "⚠️  No .cursor directory found - workflows will not be available"
fi
echo ""

# Copy kernelle.source template to ~/.kernelle/ only if it doesn't exist
if [[ ! -f "$HOME/.kernelle.source" ]]; then
    echo "🔗 Setting up shell source files..."
    cp "$SCRIPT_DIR/templates/kernelle.source.template" "$HOME/.kernelle.source"
else
    echo "~/.kernelle.source already exists - keeping existing file"
    if ! grep -q "kernelle.internal.source" "$HOME/.kernelle.source"; then
        echo "⚠️ If the line `source \"$KERNELLE_HOME/kernelle.internal.source\"` does not exist in this file already, please add it."
    fi
fi
echo ""

cp "$SCRIPT_DIR/templates/kernelle.internal.source.template" "$KERNELLE_HOME/kernelle.internal.source"

echo "✅ Kernelle installed successfully!"
echo ""
echo "📝 Next steps:"
echo "1. Add the following line to your shell configuration (~/.bashrc, ~/.zshrc, etc.):"
echo "   source ~/.kernelle.source"
echo ""
echo "2. Reload your shell or run: source ~/.kernelle.source"
echo ""
echo "3. Test the installation:"
echo "   kernelle --help"
echo "   kernelle add .  # (in a project directory)"
echo ""
echo "🎉 Enjoy your time using Kernelle!"
