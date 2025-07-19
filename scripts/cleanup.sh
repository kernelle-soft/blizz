#!/usr/bin/env bash
set -euo pipefail

# Kernelle Cleanup Script - Phase 1
# This script safely removes Kernelle while preserving user data

# Show usage information
show_cleanup_usage() {
    echo "Usage: $0 [--non-interactive] [--keep-insights|--delete-insights]"
    echo ""
    echo "Options:"
    echo "  --non-interactive    Skip interactive prompts (for CI/automation)"
    echo "  --keep-insights      Keep insights when running non-interactively"
    echo "  --delete-insights    Delete insights when running non-interactively"  
    echo "  --help, -h          Show this help message"
    echo ""
    echo "Note: In interactive mode, you'll be prompted about insights preservation."
}

# Process a single command line option
process_cleanup_option() {
    case $1 in
        --non-interactive)
            NON_INTERACTIVE=true
            ;;
        --keep-insights)
            KEEP_INSIGHTS="yes"
            ;;
        --delete-insights)
            KEEP_INSIGHTS="no"
            ;;
        --help|-h)
            show_cleanup_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_cleanup_usage
            exit 1
            ;;
    esac
}

# Parse command line arguments
parse_cleanup_arguments() {
    NON_INTERACTIVE=false
    KEEP_INSIGHTS=""
    
    while [[ $# -gt 0 ]]; do
        process_cleanup_option "$1"
        shift
    done
}

# Parse arguments
parse_cleanup_arguments "$@"

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

# Handle insights preservation decision
handle_insights_preservation() {
    if [ "$NON_INTERACTIVE" = true ]; then
        handle_non_interactive_insights
    else
        handle_interactive_insights
    fi
}

# Handle insights decision in non-interactive mode
handle_non_interactive_insights() {
    if [ "$KEEP_INSIGHTS" = "no" ]; then
        keep_insights=false
        echo "🤖 Non-interactive mode: Insights will be deleted (--delete-insights)"
    else
        keep_insights=true
        echo "🤖 Non-interactive mode: Insights will be preserved (default)"
    fi
}

# Handle insights decision in interactive mode with triple-check
handle_interactive_insights() {
    show_insights_warning
    
    if ask_yes_no "Do you want to keep your Blizz insights?" "yes"; then
        keep_insights=true
        echo "✅ Insights will be preserved"
    else
        perform_triple_check_deletion
    fi
}

# Show warning about insights importance
show_insights_warning() {
    echo "⚠️  IMPORTANT: Data Preservation Check"
    echo ""
    echo "Your Blizz insights contain hundreds of files unique to your experiences and needs."
    echo "These help Kernelle work the way you want it to. Deleting them cannot be undone."
    echo ""
}

# Perform the triple-check process for insights deletion
perform_triple_check_deletion() {
    echo "⚠️  You chose to delete your insights. This will permanently remove all your"
    echo "    accumulated knowledge, patterns, and customizations."
    echo ""
    
    if ask_yes_no "Are you SURE you want to delete your ENTIRELY IRREPLACEABLE insights? (FIRST CONFIRMATION)"; then
        echo "⚠️  Still planning to delete insights..."
        
        if ask_yes_no "FINAL CHECK: Really DELETE all your valuable insights forever?" "no"; then
            keep_insights=false
            echo "❌ Insights will be permanently deleted. I really hope you backed those up."
        else
            keep_insights=true
            echo "✅ Insights will be preserved"
        fi
    else
        keep_insights=true
        echo "✅ Insights will be preserved"
    fi
}

# Handle insights preservation
handle_insights_preservation

echo ""
echo "Soft deleting kernelle shell source files..."
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cp "$SCRIPT_DIR/templates/kernelle.internal.source.gone.template" "$KERNELLE_HOME/kernelle.internal.source"

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
# Find all symlinks that point to ~/.kernelle/.cursor (much more efficient!)
find . -type l -lname "$KERNELLE_HOME/.cursor" 2>/dev/null | while read -r link; do
    rm -f "$link"
    echo "  Removed: $link"
    
    # Remove empty .cursor directory if it only contained our symlink
    cursor_dir="$(dirname "$link")"
    if [ -d "$cursor_dir" ] && [ -z "$(ls -A "$cursor_dir" 2>/dev/null)" ]; then
        rmdir "$cursor_dir" 2>/dev/null && echo "  Removed empty: $cursor_dir"
    fi
done

# Ask about preserving tweaks
if [ -d "$KERNELLE_HOME/.cursor/tweaks" ]; then
    echo ""
    echo "📁 Found custom tweaks directory: $KERNELLE_HOME/.cursor/tweaks"
    if [ "$NON_INTERACTIVE" = true ]; then
        # Non-interactive: always preserve tweaks (safer default)
        mv "$KERNELLE_HOME/.cursor/tweaks" "$HOME/.kernelle-tweaks-backup"
        echo "🤖 Non-interactive mode: Tweaks backed up to ~/.kernelle-tweaks-backup"
    else
        if ask_yes_no "Do you want to preserve your custom tweaks?"; then
            mv "$KERNELLE_HOME/.cursor/tweaks" "$HOME/.kernelle-tweaks-backup"
            echo "✓ Tweaks backed up to ~/.kernelle-tweaks-backup"
        fi
    fi
fi

echo "🗂️  Removing ~/.kernelle directory..."
rm -rf "$KERNELLE_HOME"

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
if [ "$keep_insights" = true ]; then
    echo "💾 Your insights are safely backed up in ~/.kernelle-insights-backup"
fi
echo ""
echo "👋 Goodbye from Kernelle!" 
