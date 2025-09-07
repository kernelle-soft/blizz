#!/usr/bin/env bash
set -euo pipefail

# Show usage information
show_install_usage() {
	echo "Usage: $0 [--non-interactive] [--from-source]"
	echo ""
	echo "This script installs Blizz using pre-built binaries from GitHub releases."
	echo ""
	echo "Options:"
	echo "  --non-interactive    Install dependencies automatically without prompts"
	echo "                       (suitable for CI/automation)"
	echo "  --from-source        Build from source (for CI/development environments)"
	echo "  --help, -h          Show this help message"
	echo ""
	echo "System Requirements:"
	echo "  - curl or wget (for downloading pre-built binaries)"
	echo "  - tar (for extracting archives)"
	echo "  - For --from-source: Rust toolchain, OpenSSL dev libraries, pkg-config"
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
	local json_response
	
  latest_url="https://api.github.com/repos/kernelle-soft/blizz/releases/latest"
	if command -v curl >/dev/null 2>&1; then
		json_response=$(curl -s $latest_url)
	elif command -v wget >/dev/null 2>&1; then
		json_response=$(wget -qO- $latest_url)
	else
		echo "❌ Neither curl nor wget found. Cannot download pre-built binaries." >&2
		return 1
	fi
	
	if [ -z "$json_response" ]; then
		echo "❌ Failed to fetch release data from GitHub API" >&2
		return 1
	fi
	
	# More robust parsing - look for tag_name as the first field and extract only the value
	# This pattern ensures we only get the tag_name field value, not content from other fields
	version=$(echo "$json_response" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | sed -E 's/.*"([^"]+)".*/\1/')
	
	if [ -z "$version" ]; then
		echo "❌ Failed to parse version from GitHub API response" >&2
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
	
	echo "Detecting platform..."
	platform_archive=$(detect_platform) || return 1
	echo "Detected platform archive: $platform_archive"
	
	echo "Getting latest release version..."
	version=$(get_latest_version) || return 1
	echo "Latest version: $version"
	
	download_url="https://github.com/kernelle-soft/blizz/releases/download/$version/$platform_archive"
	echo "Downloading: $download_url"
	
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
	
	echo "Extracting binaries to $INSTALL_DIR/bin..."
	mkdir -p "$INSTALL_DIR/bin"
	tar -xzf "$temp_dir/$platform_archive" -C "$INSTALL_DIR/bin" || {
		echo "❌ Failed to extract $platform_archive" >&2
		return 1
	}
	
	echo "Pre-built binaries downdloaded"
	return 0
}



# Build from source using cargo (minimal version for CI)
build_from_source() {
	echo "Building from source..."
	
	cd "$REPO_ROOT"
	
	echo "Installing binaries from source..."
	# Install all binary crates using cargo install --path
	for crate_dir in crates/*/; do
		if [ -d "$crate_dir" ]; then
			crate=$(basename "$crate_dir")
			# Check if this crate has binary targets
			if grep -q '\[\[bin\]\]' "$crate_dir/Cargo.toml"; then
				echo "  Installing: $crate"
				cargo install --path "$crate_dir" --force --root "$INSTALL_DIR"
			else
				echo "  Skipped: $crate (library only)"
			fi
		fi
	done
	
	echo "Source build completed successfully"
}

# Install binaries using pre-built binaries or source build
install_binaries() {
	if [ "$FORCE_SOURCE_BUILD" = true ]; then
		echo "Building from source (requested via --from-source)"
		build_from_source
		return $?
	fi
	
	echo "Installing pre-built binaries..."
	if download_prebuilt_binaries; then
		echo "Pre-built binaries installed successfully"
		return 0
	else
		echo "❌ Failed to install pre-built binaries"
		echo ""
		echo "Please ensure:"
		echo "  - Your platform is supported (Linux x86_64 or macOS ARM64)"
		echo "  - You have internet connectivity"
		echo "  - curl or wget is installed"
		echo ""
		echo "If you continue to have issues, please visit:"
		echo "  https://github.com/kernelle-soft/blizz/releases"
		echo ""
		echo "For CI/development environments, try: $0 --from-source"
		return 1
	fi
}

# Setup configuration variables
setup_configuration() {
	BLIZZ_HOME="${BLIZZ_HOME:-$HOME/.blizz}"
	INSTALL_DIR="${INSTALL_DIR:-$HOME/.cargo}"
}

# Create necessary directories
create_directories() {
	echo "Creating directories..."
	mkdir -p "$BLIZZ_HOME/persistent/keeper"
	mkdir -p "$BLIZZ_HOME/volatile"
}

# Get script and repo directory paths
get_script_paths() {
	# Portable way to get script directory (works in bash and zsh)
	if [ -n "${BASH_SOURCE[0]}" ]; then
		SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
	else
		# zsh and other shells
		SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
	fi
	REPO_ROOT="$(dirname "$SCRIPT_DIR")"
}

# Setup workflow configuration files
setup_workflows() {
	echo "Setting up workflows..."
	# Copy .cursor rules to ~/.blizz/volatile/.cursor
	if [ -d "$REPO_ROOT/.cursor" ]; then
		cp -r "$REPO_ROOT/.cursor" "$BLIZZ_HOME/volatile/"
	else
		echo "⚠️  No .cursor directory found - workflows will not be available"
	fi
	echo ""
}

# Setup shell integration files
setup_shell_integration() {
	# Copy blizz.source template to ~/.blizz/ only if it doesn't exist
	if [ ! -f "$HOME/.blizz.source" ]; then
		echo "Setting up shell source files..."
		cp "$SCRIPT_DIR/templates/blizz.source.template" "$HOME/.blizz.source"
	else
		echo "~/.blizz.source already exists - keeping existing file"
		if ! grep -q "blizz.internal.source" "$HOME/.blizz.source"; then
			echo "⚠️ If the line $(source \"$BLIZZ_HOME/blizz.internal.source\") does not exist in this file already, please add it."
		fi
	fi
	echo ""

	# Copy the internal source file to the BLIZZ_HOME and source it
	cp "$SCRIPT_DIR/templates/blizz.internal.source.template" "$BLIZZ_HOME/blizz.internal.source"
	source "$BLIZZ_HOME/blizz.internal.source"
}

# Configure GPU acceleration dependencies
configure_gpu_acceleration() {
	echo "Configuring GPU acceleration dependencies..."
	# Run CUDA dependency checker if the binary was installed
	if command -v install_insights_cuda_dependencies >/dev/null 2>&1; then
		install_insights_cuda_dependencies || echo "⚠️  GPU setup encountered issues - CPU inference will be used"
	else
		echo "⚠️  CUDA dependency checker not found - skipping GPU setup"
	fi
	echo ""
}

# Setup uninstaller and related templates
setup_uninstaller() {
	echo "Setting up uninstaller..."
	
	# Copy uninstaller script to BLIZZ_HOME only if it doesn't exist
	if [ ! -f "$BLIZZ_HOME/uninstall.sh" ]; then
		cp "$SCRIPT_DIR/uninstall.sh" "$BLIZZ_HOME/uninstall.sh"
		chmod +x "$BLIZZ_HOME/uninstall.sh"
	else
		echo "$BLIZZ_HOME/uninstall.sh already exists - keeping existing file"
	fi

	# Copy required template for uninstaller to volatile only if it doesn't exist
	mkdir -p "$BLIZZ_HOME/volatile"
	if [ ! -f "$BLIZZ_HOME/volatile/blizz.internal.source.gone.template" ]; then
		cp "$SCRIPT_DIR/templates/blizz.internal.source.gone.template" "$BLIZZ_HOME/volatile/blizz.internal.source.gone.template"
	else
		echo "$BLIZZ_HOME/volatile/blizz.internal.source.gone.template already exists - keeping existing file"
	fi
}

# Show installation success message
show_success_message() {
	echo "Blizz installed successfully!"
	echo ""
	echo "📝 Next steps:"
	echo "1. Add the following line to your shell configuration (~/.bashrc, ~/.zshrc, etc.):"
	echo "   source ~/.blizz.source"
	echo ""
	echo "2. Reload your shell or run: source ~/.blizz.source"
	echo ""
	echo "3. Test the installation:"
	echo "   blizz --help"
	echo "   insights --help"
	echo "   violet --help"
	echo ""
	echo "4. To uninstall later, run:"
	echo "   ~/.blizz/uninstall.sh"
	echo ""
	echo "Let's get some stuff done!"
}

# Main installation function
main() {
	echo "Installing Blizz..."
	
	setup_configuration
	create_directories
	get_script_paths
	
	echo "Installing Blizz tools..."
	
	# Install pre-built binaries
	install_binaries || {
		echo "❌ Failed to install binaries"
		exit 1
	}
	
	setup_workflows
	setup_shell_integration
	configure_gpu_acceleration
	setup_uninstaller
	show_success_message
}

# Parse arguments and run main function
parse_install_arguments "$@"
main
