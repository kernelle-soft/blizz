#!/usr/bin/env bash
set -euo pipefail

# Blizz Cleanup Script
# This script safely removes Blizz while preserving user data

# Show usage information
show_cleanup_usage() {
	echo "Usage: $0 [--help]"
	echo ""
	echo "This script safely removes Blizz while preserving user data."
	echo ""
	echo "Options:"
	echo "  --help, -h          Show this help message"
	echo ""
	echo "Note: insights and credentials are preserved by default."
}

# Handle help and unknown options
handle_cleanup_help_and_errors() {
	local option="$1"

	if [ "$option" = "--help" ] || [ "$option" = "-h" ]; then
		show_cleanup_usage
		exit 0
	else
		echo "Unknown option: $option"
		show_cleanup_usage
		exit 1
	fi
}

# Process a single command line option
process_cleanup_option() {
	case $1 in
	--help | -h | *)
		handle_cleanup_help_and_errors "$1"
		;;
	esac
}

# Parse command line arguments
parse_cleanup_arguments() {
	while [ $# -gt 0 ]; do
		process_cleanup_option "$1"
		shift
	done
}

# Setup configuration variables
setup_cleanup_configuration() {
	BLIZZ_HOME="${BLIZZ_HOME:-$HOME/.blizz}"
	INSTALL_DIR="${INSTALL_DIR:-$HOME/.cargo/bin}"
}

# Clean up shell source files
cleanup_shell_source_files() {
	echo "Soft deleting blizz shell source files..."
	# Replace internal source with gone template from volatile, keep directory structure
	if [ -d "$BLIZZ_HOME" ]; then
		if [ -f "$BLIZZ_HOME/volatile/blizz.internal.source.gone.template" ]; then
			cp "$BLIZZ_HOME/volatile/blizz.internal.source.gone.template" "$BLIZZ_HOME/blizz.internal.source" || true
		else
			echo "⚠️  Could not find gone template in volatile - blizz.internal.source not updated"
		fi
	fi
}

# Remove cursor workflow symlinks
remove_cursor_symlinks() {
	echo "🔗 Removing cursor workflow symlinks..."
	# Find all symlinks that point to ~/.blizz/volatile/.cursor
	find . -type l -lname "$BLIZZ_HOME/volatile/.cursor" 2>/dev/null | while read -r link; do
		rm -f "$link"
		echo "  Removed: $link"

		# Remove empty .cursor directory if it only contained our symlink
		cursor_dir="$(dirname "$link")"
		if [ -d "$cursor_dir" ] && [ -z "$(ls -A "$cursor_dir" 2>/dev/null)" ]; then
			rmdir "$cursor_dir" 2>/dev/null && echo "  Removed empty: $cursor_dir" || true
		fi
	done || true
}

# Clean up the blizz directory
cleanup_blizz_directory() {
	echo "Cleaning ~/.blizz directory..."
	if [ -d "$BLIZZ_HOME" ]; then
		# Clean up uninstaller files (but keep persistent data)
		rm -f "$BLIZZ_HOME/uninstall.sh" 2>/dev/null || true
		
		# Remove volatile directory - it contains no user data
		if [ -d "$BLIZZ_HOME/volatile" ]; then
			rm -rf "$BLIZZ_HOME/volatile" 2>/dev/null || true
		fi
	fi
}

# Remove installed binaries
remove_binaries() {
	echo "Removing binaries from $INSTALL_DIR..."
	for tool in blizz insights insights_daemon install_insights_cuda_dependencies secrets keeper violet; do
		if [ -f "$INSTALL_DIR/$tool" ]; then
			rm -f "$INSTALL_DIR/$tool"
			echo "  Removed: $tool"
		fi
	done
}

# Show cleanup completion message
show_cleanup_completion() {
	echo ""
	echo "Blizz cleanup completed!"
	echo ""
	echo "📝 Don't forget to:"
	echo "1. Remove 'source ~/.blizz.source' from your shell configuration"
	echo "2. Reload your shell to stop seeing the warning message"
	echo ""
	echo "Your insights and other customizations are still stored in ~/.blizz/persistent/ for safe keeping"
	echo ""
	echo "👋 Goodbye!"
}

# Main uninstall function
main() {
	echo "Uninstalling Blizz..."
	echo ""
	
	setup_cleanup_configuration
	cleanup_shell_source_files
	remove_cursor_symlinks
	cleanup_blizz_directory
	remove_binaries
	show_cleanup_completion
}

# Parse arguments and run main function
parse_cleanup_arguments "$@"
main
