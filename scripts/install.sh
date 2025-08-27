#!/usr/bin/env bash
set -euo pipefail

# Show usage information
show_install_usage() {
	echo "Usage: $0 [--non-interactive] [--from-source]"
	echo ""
	echo "This script installs Blizz using pre-built binaries when available,"
	echo "or falls back to building from source if needed."
	echo ""
	echo "Options:"
	echo "  --non-interactive    Install dependencies automatically without prompts"
	echo "                       (suitable for CI/automation)"
	echo "  --from-source        Force building from source instead of using pre-built binaries"
	echo "  --help, -h          Show this help message"
	echo ""
	echo "System Requirements:"
	echo "  - curl or wget (for downloading pre-built binaries)"
	echo "  - tar (for extracting archives)"
	echo "  - For source builds: Rust toolchain, OpenSSL dev libraries, pkg-config"
}

# Handle help and unknown options
handle_install_help_and_errors() {
	local option="$1"

	if [ "$option" = "--help" ] || [ "$option" = "-h" ]; then
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
	--from-source)
		FORCE_SOURCE_BUILD=true
		;;
	--help | -h | *)
		handle_install_help_and_errors "$1"
		;;
	esac
}

# Parse command line arguments
parse_install_arguments() {
	NON_INTERACTIVE=false
	FORCE_SOURCE_BUILD=false

	while [ $# -gt 0 ]; do
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

# Detect the current platform and return the appropriate binary archive name
detect_platform() {
	local os
	local arch
	
	os=$(uname -s | tr '[:upper:]' '[:lower:]')
	arch=$(uname -m)
	
	case "$os-$arch" in
	linux-x86_64)
		echo "blizz-x86_64-unknown-linux-gnu.tar.gz"
		return 0
		;;
	darwin-arm64)
		echo "blizz-aarch64-apple-darwin.tar.gz"
		return 0
		;;
	*)
		echo "Unsupported platform: $os-$arch" >&2
		return 1
		;;
	esac
}

# Get the latest release version from GitHub
get_latest_version() {
	local version
	
	if command -v curl >/dev/null 2>&1; then
		version=$(curl -s https://api.github.com/repos/kernelle-soft/blizz/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
	elif command -v wget >/dev/null 2>&1; then
		version=$(wget -qO- https://api.github.com/repos/kernelle-soft/blizz/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
	else
		echo "❌ Neither curl nor wget found. Cannot download pre-built binaries." >&2
		return 1
	fi
	
	if [ -z "$version" ]; then
		echo "❌ Failed to get latest version from GitHub API" >&2
		return 1
	fi
	
	echo "$version"
}

# Download and extract pre-built binaries
download_prebuilt_binaries() {
	local platform_archive
	local version
	local download_url
	local temp_dir
	
	echo "🔍 Detecting platform..."
	platform_archive=$(detect_platform) || return 1
	echo "✅ Detected platform archive: $platform_archive"
	
	echo "🔍 Getting latest release version..."
	version=$(get_latest_version) || return 1
	echo "✅ Latest version: $version"
	
	download_url="https://github.com/kernelle-soft/blizz/releases/download/$version/$platform_archive"
	echo "📥 Downloading: $download_url"
	
	temp_dir=$(mktemp -d)
	trap "rm -rf '$temp_dir'" EXIT
	
	if command -v curl >/dev/null 2>&1; then
		curl -L "$download_url" -o "$temp_dir/$platform_archive" || {
			echo "❌ Failed to download $download_url" >&2
			return 1
		}
	elif command -v wget >/dev/null 2>&1; then
		wget "$download_url" -O "$temp_dir/$platform_archive" || {
			echo "❌ Failed to download $download_url" >&2
			return 1
		}
	else
		echo "❌ Neither curl nor wget found. Cannot download pre-built binaries." >&2
		return 1
	fi
	
	echo "📦 Extracting binaries to $INSTALL_DIR/bin..."
	mkdir -p "$INSTALL_DIR/bin"
	tar -xzf "$temp_dir/$platform_archive" -C "$INSTALL_DIR/bin" || {
		echo "❌ Failed to extract $platform_archive" >&2
		return 1
	}
	
	echo "✅ Pre-built binaries installed successfully"
	return 0
}

# Build from source using cargo
build_from_source() {
	echo "🔨 Building from source..."
	
	# Check system dependencies for source build
	check_system_dependencies
	
	# For source builds, we need the repo
	if [ ! -f "$REPO_ROOT/Cargo.toml" ]; then
		echo "❌ Error: Cargo.toml not found at $REPO_ROOT/Cargo.toml"
		echo "Contents of $REPO_ROOT:"
		ls -la "$REPO_ROOT"
		return 1
	fi
	
	cd "$REPO_ROOT"
	
	echo "📦 Installing binaries from source..."
	
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
	
	echo "✅ Source build completed successfully"
}

# Install binaries using the best available method
install_binaries() {
	if [ "$FORCE_SOURCE_BUILD" = true ]; then
		echo "🔧 Forced source build requested"
		build_from_source
		return $?
	fi
	
	echo "🚀 Attempting to install pre-built binaries..."
	if download_prebuilt_binaries; then
		echo "✅ Pre-built binaries installed successfully"
		return 0
	else
		echo "⚠️  Pre-built binaries not available, falling back to source build"
		build_from_source
		return $?
	fi
}

# Parse arguments
parse_install_arguments "$@"

echo "🚀 Installing Blizz..."

# Configuration
KERNELLE_HOME="${KERNELLE_HOME:-$HOME/.kernelle}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.cargo}"

# Create directories
echo "📁 Creating directories..."
mkdir -p "$KERNELLE_HOME/persistent/keeper"
mkdir -p "$KERNELLE_HOME/volatile"

# Get script and repo directory info (needed for source builds and templates)
# Portable way to get script directory (works in bash and zsh)
if [ -n "${BASH_SOURCE[0]}" ]; then
	SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
else
	# zsh and other shells
	SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
fi
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🔨 Installing Blizz tools..."
echo "Script directory: $SCRIPT_DIR"
echo "Repository root: $REPO_ROOT"
echo "Current directory: $(pwd)"

# Install binaries (pre-built or from source)
install_binaries || {
	echo "❌ Failed to install binaries"
	exit 1
}

echo "📋 Setting up workflows..."
# Copy .cursor rules to ~/.kernelle/volatile/.cursor
if [ -d "$REPO_ROOT/.cursor" ]; then
	cp -r "$REPO_ROOT/.cursor" "$KERNELLE_HOME/volatile/"
else
	echo "⚠️  No .cursor directory found - workflows will not be available"
fi
echo ""

# Copy kernelle.source template to ~/.kernelle/ only if it doesn't exist
if [ ! -f "$HOME/.kernelle.source" ]; then
	echo "🔗 Setting up shell source files..."
	cp "$SCRIPT_DIR/templates/kernelle.source.template" "$HOME/.kernelle.source"
else
	echo "~/.kernelle.source already exists - keeping existing file"
	if ! grep -q "kernelle.internal.source" "$HOME/.kernelle.source"; then
		echo "⚠️ If the line $(source \"$KERNELLE_HOME/kernelle.internal.source\") does not exist in this file already, please add it."
	fi
fi
echo ""

# Copy the internal source file to the KERNELLE_HOME and source it
cp "$SCRIPT_DIR/templates/kernelle.internal.source.template" "$KERNELLE_HOME/kernelle.internal.source"
source "$KERNELLE_HOME/kernelle.internal.source"

echo "🎯 Configuring GPU acceleration dependencies..."
# Run CUDA dependency checker if the binary was installed
if command -v install_insights_cuda_dependencies >/dev/null 2>&1; then
	install_insights_cuda_dependencies || echo "⚠️  GPU setup encountered issues - CPU inference will be used"
else
	echo "⚠️  CUDA dependency checker not found - skipping GPU setup"
fi
echo ""

echo "📝 Setting up uninstaller..."
# Copy uninstaller script to KERNELLE_HOME

# Copy uninstaller script to KERNELLE_HOME only if it doesn't exist
if [ ! -f "$KERNELLE_HOME/uninstall.sh" ]; then
	cp "$SCRIPT_DIR/uninstall.sh" "$KERNELLE_HOME/uninstall.sh"
	chmod +x "$KERNELLE_HOME/uninstall.sh"
else
	echo "$KERNELLE_HOME/uninstall.sh already exists - keeping existing file"
fi

# Copy required template for uninstaller to volatile only if it doesn't exist
mkdir -p "$KERNELLE_HOME/volatile"
if [ ! -f "$KERNELLE_HOME/volatile/kernelle.internal.source.gone.template" ]; then
	cp "$SCRIPT_DIR/templates/kernelle.internal.source.gone.template" "$KERNELLE_HOME/volatile/kernelle.internal.source.gone.template"
else
	echo "$KERNELLE_HOME/volatile/kernelle.internal.source.gone.template already exists - keeping existing file"
fi

echo "✅ Blizz installed successfully!"
echo ""
echo "📝 Next steps:"
echo "1. Add the following line to your shell configuration (~/.bashrc, ~/.zshrc, etc.):"
echo "   source ~/.kernelle.source"
echo ""
echo "2. Reload your shell or run: source ~/.kernelle.source"
echo ""
echo "3. Test the installation:"
echo "   kernelle --help"
echo "   insights --help"
echo "   violet --help"
echo ""
echo "4. To uninstall later, run:"
echo "   ~/.kernelle/uninstall.sh"
echo ""
echo "Let's get some stuff done!"
