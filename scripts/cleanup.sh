#!/usr/bin/env bash
set -euo pipefail

# Kernelle Cleanup Script - Phase 1
# This script safely removes Kernelle while preserving user data

echo "🧹 Kernelle Cleanup..."

# Configuration
KERNELLE_HOME="${KERNELLE_HOME:-$HOME/.kernelle}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Helper function to ask yes/no questions
ask_yes_no() {
    local question="$1"
    local default="${2:-no}"
    
    while true; do
        if [ "$default" = "yes" ]; then
            read -p "$question [Y/n]: " answer
            answer="${answer:-yes}"
        else
            read -p "$question [y/N]: " answer
            answer="${answer:-no}"
        fi
        
        case "$answer" in
            [Yy]|[Yy][Ee][Ss]) return 0 ;;
            [Nn]|[Nn][Oo]) return 1 ;;
            *) echo "Please answer yes or no." ;;
        esac
    done
}

# Triple check about insights as per requirement
echo "⚠️  IMPORTANT: Data Preservation Check"
echo ""
if ask_yes_no "Do you want to keep your Blizz insights? (FIRST CHECK)"; then
    keep_insights=true
    echo "✓ Insights will be preserved"
    
    if ask_yes_no "Are you SURE you want to keep your insights? (SECOND CHECK)" "yes"; then
        echo "✓ Double-confirmed: insights will be preserved"
        
        if ask_yes_no "FINAL CHECK: Keep insights safe from deletion?" "yes"; then
            echo "✅ Triple-confirmed: insights will be preserved"
        else
            keep_insights=false
            echo "❌ Insights will be deleted"
        fi
    else
        keep_insights=false
        echo "❌ Insights will be deleted"
    fi
else
    keep_insights=false
    echo "❌ Insights will be deleted"
fi

echo ""
echo "🔄 Replacing kernelle.source with soft-delete version..."
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cp "$SCRIPT_DIR/templates/kernelle.source.gone" "$HOME/.kernelle.source"

if [ "$keep_insights" = true ]; then
    echo "💾 Preserving insights..."
    if [ -d "$KERNELLE_HOME/insights" ]; then
        mv "$KERNELLE_HOME/insights" "$HOME/.kernelle-insights-backup"
        echo "✓ Insights backed up to ~/.kernelle-insights-backup"
    fi
fi

echo "🗑️  Removing global insights..."
rm -rf "$KERNELLE_HOME/global-insights" 2>/dev/null || true

echo "🔗 Removing cursor workflow symlinks..."
# Find all symlinks pointing to ~/.kernelle/.cursor and remove them
find . -name ".cursor" -type d 2>/dev/null | while read -r cursor_dir; do
    if [ -d "$cursor_dir" ]; then
        find "$cursor_dir" -type l | while read -r link; do
            if readlink "$link" 2>/dev/null | grep -q "^$KERNELLE_HOME/.cursor"; then
                rm -f "$link"
                echo "  Removed: $link"
            fi
        done
    fi
done

# Ask about preserving tweaks
if [ -d "$KERNELLE_HOME/.cursor/tweaks" ]; then
    echo ""
    echo "📁 Found custom tweaks directory: $KERNELLE_HOME/.cursor/tweaks"
    if ask_yes_no "Do you want to preserve your custom tweaks?"; then
        mv "$KERNELLE_HOME/.cursor/tweaks" "$HOME/.kernelle-tweaks-backup"
        echo "✓ Tweaks backed up to ~/.kernelle-tweaks-backup"
    fi
fi

echo "🗂️  Removing ~/.kernelle directory..."
rm -rf "$KERNELLE_HOME"

echo "🗑️  Removing binaries from $INSTALL_DIR..."
for tool in kernelle jerrod blizz bentley violet adam sentinel; do
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
if [ "$keep_insights" = true ]; then
    echo "💾 Your insights are safely backed up in ~/.kernelle-insights-backup"
fi
echo ""
echo "👋 Goodbye from Kernelle!" 