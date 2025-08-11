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
# Replace internal source with gone template from volatile, keep directory structure
if [ -d "$KERNELLE_HOME" ]; then
	if [ -f "$KERNELLE_HOME/volatile/kernelle.internal.source.gone.template" ]; then
		cp "$KERNELLE_HOME/volatile/kernelle.internal.source.gone.template" "$KERNELLE_HOME/kernelle.internal.source" || true
	else
		echo "⚠️  Could not find gone template in volatile - kernelle.internal.source not updated"
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
	# Clean up uninstaller files (but keep persistent data)
	rm -f "$KERNELLE_HOME/uninstall.sh" 2>/dev/null || true
	rm -f "$KERNELLE_HOME/volatile/kernelle.internal.source.gone.template" 2>/dev/null || true
	# Optionally remove volatile if empty
	if [ -d "$KERNELLE_HOME/volatile" ] && [ -z "$(ls -A "$KERNELLE_HOME/volatile" 2>/dev/null)" ]; then
		rmdir "$KERNELLE_HOME/volatile" 2>/dev/null || true
	fi
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
echo "Your insights and other customizations remain safely stored in ~/.kernelle/persistent/"
echo ""
echo "👋 Goodbye!"

# Ensure successful exit
exit 0
