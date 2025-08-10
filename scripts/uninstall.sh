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
# Replace internal source with gone template, keep directory structure
if [ -d "$KERNELLE_HOME" ]; then
    # Try to use gone template from KERNELLE_HOME first (new installs)
    if [ -f "$KERNELLE_HOME/kernelle.internal.source.gone.template" ]; then
        cp "$KERNELLE_HOME/kernelle.internal.source.gone.template" "$KERNELLE_HOME/kernelle.internal.source" || true
    else
        # Fall back to original location for backwards compatibility
        # Look for the template in common possible locations
        template_found=false
        for template_path in \
            "$(dirname "$PWD")/scripts/templates/kernelle.internal.source.gone.template" \
            "./scripts/templates/kernelle.internal.source.gone.template" \
            "../scripts/templates/kernelle.internal.source.gone.template" \
            "../../scripts/templates/kernelle.internal.source.gone.template"; do
            if [ -f "$template_path" ]; then
                cp "$template_path" "$KERNELLE_HOME/kernelle.internal.source" || true
                template_found=true
                break
            fi
        done
        if [ "$template_found" = false ]; then
            echo "⚠️  Could not find gone template - kernelle.internal.source not updated"
        fi
    fi
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
    # Clean up uninstaller files (but keep persistent data)
    rm -f "$KERNELLE_HOME/uninstall.sh" 2>/dev/null || true
    rm -f "$KERNELLE_HOME/kernelle.internal.source.gone.template" 2>/dev/null || true
fi

echo "🗑️  Removing binaries from $INSTALL_DIR..."
for tool in kernelle blizz insights violet adam sentinel; do
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
