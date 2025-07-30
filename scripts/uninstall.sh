#!/usr/bin/env bash
set -euo pipefail

# Kernelle Cleanup Script - Phase 1
# This script safely removes Kernelle while preserving user data

# Show usage information
show_cleanup_usage() {
    echo "Usage: $0"
    echo ""
    echo "Options:"
    echo "  --help, -h          Show this help message"
    echo ""
    echo "Note: insights and credentials are preserved by default."
}

echo "🧹 Kernelle Cleanup..."

# Configuration
KERNELLE_HOME="${KERNELLE_HOME:-$HOME/.kernelle}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.cargo/bin}"

echo ""
echo "Soft deleting kernelle shell source files..."
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Replace internal source with gone template, keep directory structure
if [ -d "$KERNELLE_HOME" ]; then
    # Copy gone template to internal source location
    cp "$SCRIPT_DIR/templates/kernelle.internal.source.gone.template" "$KERNELLE_HOME/kernelle.internal.source" || true
fi

echo "🔗 Removing cursor workflow symlinks..."
# Find all symlinks that point to ~/.kernelle/volatile/.cursor (updated path)
find . -type l -lname "$KERNELLE_HOME/volatile/.cursor" 2>/dev/null | while read -r link; do
    rm -f "$link"
    echo "  Removed: $link"
    
    # Remove empty .cursor directory if it only contained our symlink
    cursor_dir="$(dirname "$link")"
    if [ -d "$cursor_dir" ] && [ -z "$(ls -A "$cursor_dir" 2>/dev/null)" ]; then
        rmdir "$cursor_dir" 2>/dev/null && echo "  Removed empty: $cursor_dir" || true
    fi
done || true

echo "🗂️  Cleaning ~/.kernelle directory..."
if [ -d "$KERNELLE_HOME" ]; then
    rm -rf "$KERNELLE_HOME/volatile" 2>/dev/null || true
fi

echo "🗑️  Removing binaries from $INSTALL_DIR..."
for tool in kernelle jerrod blizz violet adam sentinel; do
    if [ -f "$INSTALL_DIR/$tool" ]; then
        rm -f "$INSTALL_DIR/$tool"
        echo "  Removed: $tool"
    fi
done

echo ""
echo "✅ Kernelle cleanup completed!"
echo ""
echo "📝 Don't forget to:"
echo "1. Remove 'source ~/.kernelle.source' from your shell configuration"
echo "2. Reload your shell to stop seeing the warning message"
echo ""
echo "💾 Your insights and other customizations remain safely stored in ~/.kernelle/persistent/"
echo ""
echo "👋 Goodbye from Kernelle!"

# Ensure successful exit
exit 0 
