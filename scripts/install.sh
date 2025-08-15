#!/usr/bin/env bash
set -euo pipefail

# Kernelle Installation Script - Phase 1
# This script installs Kernelle and sets up the basic lifecycle infrastructure

# Show usage information
show_install_usage() {
	echo "Usage: $0 [--non-interactive]"
	echo ""
	echo "This script installs Kernelle and its dependencies. It will check for"
	echo "required system packages (OpenSSL development libraries, pkg-config)"
	echo "and install them automatically."
	echo ""
	echo "Options:"
	echo "  --non-interactive    Install dependencies automatically without prompts"
	echo "                       (suitable for CI/automation)"
	echo "  --help, -h          Show this help message"
	echo ""
	echo "System Requirements:"
	echo "  - Rust toolchain (cargo)"
	echo "  - OpenSSL development libraries (installed automatically)"
	echo "  - pkg-config (installed automatically)"
}

# Handle help and unknown options
handle_install_help_and_errors() {
	local option="$1"

	if [[ "$option" == "--help" || "$option" == "-h" ]]; then
		show_install_usage
		exit 0
	else
		echo "Unknown option: $option"
		show_install_usage
		exit 1
	fi
}

# Process a single command line option
process_install_option() {
	case $1 in
	--non-interactive)
		NON_INTERACTIVE=true
		;;
	--help | -h | *)
		handle_install_help_and_errors "$1"
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

# Check for required system dependencies
check_system_dependencies() {
	echo "🔍 Checking system dependencies..."

	local missing_deps=()

	# Check for pkg-config
	if ! command -v pkg-config >/dev/null 2>&1; then
		missing_deps+=("pkg-config")
	fi

	# Check for OpenSSL development libraries
	if ! pkg-config --exists openssl 2>/dev/null; then
		local openssl_pkg
		openssl_pkg=$(get_openssl_package_name)
		if [ -n "$openssl_pkg" ]; then
			missing_deps+=("$openssl_pkg")
		fi
	fi

	if [ ${#missing_deps[@]} -gt 0 ]; then
		handle_missing_dependencies "${missing_deps[@]}"
	else
		echo "✅ All system dependencies are satisfied"
	fi
}

# Get the appropriate OpenSSL package name for the current system
get_openssl_package_name() {
	if command -v apt-get >/dev/null 2>&1; then
		echo "libssl-dev"
	elif command -v yum >/dev/null 2>&1; then
		echo "openssl-devel"
	elif command -v dnf >/dev/null 2>&1; then
		echo "openssl-devel"
	elif command -v pacman >/dev/null 2>&1; then
		echo "openssl"
	elif command -v brew >/dev/null 2>&1; then
		echo "openssl"
	else
		echo "⚠️  Could not determine package manager. Please install OpenSSL development libraries manually."
		return 1
	fi
}

# Handle missing dependencies based on interactive/non-interactive mode
handle_missing_dependencies() {
	local deps=("$@")
	echo "❌ Missing required dependencies: ${deps[*]}"
	echo ""

	if [ "$NON_INTERACTIVE" = true ]; then
		echo "Running in non-interactive mode. Installing dependencies automatically..."
		install_system_dependencies "${deps[@]}"
	else
		prompt_for_dependency_installation "${deps[@]}"
	fi
}

# Prompt user for dependency installation in interactive mode
prompt_for_dependency_installation() {
	local deps=("$@")
	echo "Would you like to install these dependencies automatically? (y/N)"
	read -r response
	case "$response" in
	[yY][eE][sS] | [yY])
		install_system_dependencies "${deps[@]}"
		;;
	*)
		show_manual_installation_commands "${deps[@]}"
		exit 1
		;;
	esac
}

# Show manual installation commands for dependencies
show_manual_installation_commands() {
	local deps=("$@")
	echo "Please install the dependencies manually and run the install script again."
	echo ""
	echo "Manual installation commands:"

	if command -v apt-get >/dev/null 2>&1; then
		echo "  sudo apt update && sudo apt install -y ${deps[*]}"
	elif command -v yum >/dev/null 2>&1; then
		echo "  sudo yum install -y ${deps[*]}"
	elif command -v dnf >/dev/null 2>&1; then
		echo "  sudo dnf install -y ${deps[*]}"
	elif command -v pacman >/dev/null 2>&1; then
		echo "  sudo pacman -S ${deps[*]}"
	elif command -v brew >/dev/null 2>&1; then
		echo "  brew install ${deps[*]}"
	fi
}

# Install system dependencies based on the detected package manager
install_system_dependencies() {
	local deps=("$@")
	echo "📦 Installing system dependencies: ${deps[*]}"

	# Log for CI/debugging purposes
	if [ "${CI:-}" = "true" ] || [ "${GITHUB_ACTIONS:-}" = "true" ]; then
		echo "🔍 CI environment detected - logging package manager and OS info"
		echo "OS: $(uname -a)"
		if command -v lsb_release >/dev/null 2>&1; then
			echo "Distribution: $(lsb_release -d)"
		fi
	fi

	if command -v apt-get >/dev/null 2>&1; then
		sudo apt update && sudo apt install -y "${deps[@]}"
	elif command -v yum >/dev/null 2>&1; then
		sudo yum install -y "${deps[@]}"
	elif command -v dnf >/dev/null 2>&1; then
		sudo dnf install -y "${deps[@]}"
	elif command -v pacman >/dev/null 2>&1; then
		sudo pacman -S "${deps[@]}"
	elif command -v brew >/dev/null 2>&1; then
		brew install "${deps[@]}"
	else
		echo "❌ Could not determine package manager. Please install dependencies manually."
		exit 1
	fi

	echo "✅ System dependencies installed successfully"
}

# Parse arguments
parse_install_arguments "$@"

echo "🚀 Installing Kernelle..."

# Check system dependencies first
check_system_dependencies

# Configuration
KERNELLE_HOME="${KERNELLE_HOME:-$HOME/.kernelle}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.cargo}"

# Create directories
echo "📁 Creating directories..."
mkdir -p "$KERNELLE_HOME/persistent/keeper"
mkdir -p "$KERNELLE_HOME/volatile"

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
			cargo install --path "$crate_dir" --force --root "$INSTALL_DIR"
		else
			echo "  Skipped: $crate (library only)"
		fi
	fi
done

echo "📋 Setting up workflows..."
# Copy .cursor rules to ~/.kernelle/volatile/.cursor
if [ -d "$REPO_ROOT/.cursor" ]; then
	cp -r "$REPO_ROOT/.cursor" "$KERNELLE_HOME/volatile/"
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
		echo "⚠️ If the line $(source \"$KERNELLE_HOME/kernelle.internal.source\") does not exist in this file already, please add it."
	fi
fi
echo ""

cp "$SCRIPT_DIR/templates/kernelle.internal.source.template" "$KERNELLE_HOME/kernelle.internal.source"

echo "📝 Setting up uninstaller..."
# Copy uninstaller script to KERNELLE_HOME

# Copy uninstaller script to KERNELLE_HOME only if it doesn't exist
if [[ ! -f "$KERNELLE_HOME/uninstall.sh" ]]; then
	cp "$SCRIPT_DIR/uninstall.sh" "$KERNELLE_HOME/uninstall.sh"
	chmod +x "$KERNELLE_HOME/uninstall.sh"
else
	echo "$KERNELLE_HOME/uninstall.sh already exists - keeping existing file"
fi

# Copy required template for uninstaller to volatile only if it doesn't exist
mkdir -p "$KERNELLE_HOME/volatile"
if [[ ! -f "$KERNELLE_HOME/volatile/kernelle.internal.source.gone.template" ]]; then
	cp "$SCRIPT_DIR/templates/kernelle.internal.source.gone.template" "$KERNELLE_HOME/volatile/kernelle.internal.source.gone.template"
else
	echo "$KERNELLE_HOME/volatile/kernelle.internal.source.gone.template already exists - keeping existing file"
fi

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
echo "4. To uninstall later, run:"
echo "   ~/.kernelle/uninstall.sh"
echo ""
echo "🎉 Enjoy your time using Kernelle!"
