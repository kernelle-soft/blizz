#!/bin/sh

# Kernelle Installation Script
# A comprehensive setup script for the Kernelle AI development toolshed
# 
# Usage: curl -sSL https://raw.githubusercontent.com/TravelSizedLions/kernelle/main/setup.sh | sh
#    or: curl -sSL https://raw.githubusercontent.com/TravelSizedLions/kernelle/main/setup.sh | bash
#    or: curl -sSL https://raw.githubusercontent.com/TravelSizedLions/kernelle/main/setup.sh | zsh

set -euo pipefail

# ANSI color codes for pretty output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Global variables
TMP_DIR=""
KERNELLE_HOME="$HOME/.kernelle"
INSTALL_DIR="$HOME/.cargo/bin"
REPO_URL="https://github.com/TravelSizedLions/kernelle.git"

# Banner function
show_banner() {
    printf "${PURPLE}${BOLD}"
    cat << 'EOF'
╦╔═╔═╗╦═╗╔╗╔╔═╗╦  ╦  ╔═╗
╠╩╗║╣ ╠╦╝║║║║╣ ║  ║  ║╣ 
╩ ╩╚═╝╩╚═╝╚╝╚═╝╩═╝╩═╝╚═╝
EOF
    printf "${NC}\n"
    printf "${CYAN}${BOLD}AI Development Toolshed${NC}\n"
    printf "${BLUE}Rust-powered investigation-to-merge workflow acceleration${NC}\n\n"
}

# Logging functions with colors and timestamps
log_info() {
    printf "${GREEN}[INFO]${NC} %s: %s\n" "$(date '+%H:%M:%S')" "$*"
}

log_warn() {
    printf "${YELLOW}[WARN]${NC} %s: %s\n" "$(date '+%H:%M:%S')" "$*" 62
}

log_error() {
    printf "${RED}[ERROR]${NC} %s: %s\n" "$(date '+%H:%M:%S')" "$*" 62
}

log_success() {
    printf "${GREEN}[SUCCESS]${NC} %s: %s\n" "$(date '+%H:%M:%S')" "$*"
}

# Function to exit with error message
die() {
    log_error "$*"
    exit 1
}

# Function to check if a command exists
cmd_exists() {
    command -v "$1" /dev/null 261
}

# Cleanup function
cleanup() {
    if [ -n "${TMP_DIR:-}" ] && [ -d "$TMP_DIR" ]; then
        log_info "Cleaning up temporary directory: $TMP_DIR"
        rm -rf "$TMP_DIR" || log_warn "Failed to clean up temporary directory"
    fi
}

# Register cleanup on exit
trap 'cleanup' EXIT INT TERM

# Check system requirements
check_requirements() {
    log_info "Checking system requirements..."
    
    # Check for git
    if ! cmd_exists "git"; then
        die "Git is required but not installed. Please install git and try again."
    fi
    
    # Check for Rust/Cargo
    if ! cmd_exists "cargo"; then
        log_warn "Rust/Cargo not found. Installing rustup..."
        install_rust
    else
        log_info "Found Rust/Cargo: $(cargo --version)"
    fi
    
    # Check cargo bin directory is in PATH
    if [ ! -d "$INSTALL_DIR" ]; then
        mkdir -p "$INSTALL_DIR"
    fi
    
    case ":$PATH:" in
        *":$INSTALL_DIR:")
            log_info "Cargo bin directory is in PATH"
            ;;
        *)
            log_warn "$INSTALL_DIR is not in PATH. You may need to add it manually."
            ;;
    esac
}

# Install Rust if not present
install_rust() {
    log_info "Installing Rust via rustup..."
    if cmd_exists "curl"; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    elif cmd_exists "wget"; then
        wget -qO- https://sh.rustup.rs | sh -s -- -y
    else
        die "Neither curl nor wget found. Please install Rust manually: https://rustup.rs/"
    fi
    
    # Source the cargo environment
    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck source=/dev/null
        . "$HOME/.cargo/env"
    fi
    
    if ! cmd_exists "cargo"; then
        die "Rust installation failed. Please install manually: https://rustup.rs/"
    fi
    
    log_success "Rust installed successfully: $(cargo --version)"
}

# Setup kernelle home directory
setup_kernelle_home() {
    log_info "Setting up Kernelle home directory at $KERNELLE_HOME"
    
    mkdir -p "$KERNELLE_HOME"
    mkdir -p "$KERNELLE_HOME/insights"
    mkdir -p "$KERNELLE_HOME/config"
    
    # Create a basic insights README if it doesn't exist
    if [ ! -f "$KERNELLE_HOME/insights/README.md" ]; then
        cat > "$KERNELLE_HOME/insights/README.md" << 'EOF'
# Kernelle Insights

This directory contains insights gathered by Blizz and curated by Adam.

## Structure
- `projects/` - Project-specific insights
- `general/` - General development insights
- `teams/` - Team or organization insights (if shared)

## Usage
Insights are stored as markdown files that can be:
- Searched with `grep` or `rg`
- Viewed with any text editor
- Shared via git repositories
- Curated automatically by Adam

For more information, run `blizz --help` after installation.
EOF
        log_info "Created insights directory structure"
    fi
}

# Clone the repository
clone_repository() {
    log_info "Creating temporary directory for build..."
    TMP_DIR=$(mktemp -d)
    log_info "Using temporary directory: $TMP_DIR"
    
    log_info "Cloning Kernelle repository..."
    git clone --depth 1 "$REPO_URL" "$TMP_DIR/kernelle" || die "Failed to clone repository"
    
    cd "$TMP_DIR/kernelle" || die "Failed to enter repository directory"
    log_success "Repository cloned successfully"
    
    # Copy Cursor workflows
    if [ -d ".cursor" ]; then
        mkdir -p "$KERNELLE_HOME"
        rm -rf "$KERNELLE_HOME/.cursor"
        cp -R ".cursor" "$KERNELLE_HOME/.cursor"
        log_success "Copied Cursor workflows to $KERNELLE_HOME/.cursor"
    else
        log_warn "No .cursor directory found in repo; skipping Cursor workflow copy"
    fi
    
    # Copy add-workflows.sh script
    if [ -f "add-workflows.sh" ]; then
        cp "add-workflows.sh" "$KERNELLE_HOME/add-workflows.sh"
        chmod +x "$KERNELLE_HOME/add-workflows.sh"
        log_success "Copied add-workflows.sh to $KERNELLE_HOME"
    else
        log_warn "No add-workflows.sh script found in repo; skipping workflow script copy"
    fi
}

# Build and install the tools
build_and_install() {
    log_info "Building Kernelle tools (this may take a few minutes)..."
    
    # Build the workspace
    cargo build --release --workspace || die "Build failed"
    
    log_info "Installing Kernelle tools..."
    
    # Install each tool
    local tools="jerrod blizz adam violet bentley sentinel"
    local installed_count=0
    
    for tool in $tools; do
        if [ -d "crates/$tool" ]; then
            log_info "Installing $tool..."
            if cargo install --path "crates/$tool" --force; then
                installed_count=$((installed_count + 1))
                log_success "$tool installed successfully"
            else
                log_warn "Failed to install $tool"
            fi
        else
            log_warn "Tool $tool not found in crates directory"
        fi
    done
    
    if [ $installed_count -eq 0 ]; then
        die "No tools were installed successfully"
    fi
    
    log_success "Installed $installed_count Kernelle tools"
}

# Verify installation
verify_installation() {
    log_info "Verifying installation..."
    
    local tools="jerrod blizz adam violet bentley sentinel"
    local verified_count=0
    
    for tool in $tools; do
        if cmd_exists "$tool"; then
            log_success "✓ $tool is available"
            verified_count=$((verified_count + 1))
        else
            log_warn "✗ $tool is not available in PATH"
        fi
    done
    
    if [ $verified_count -eq 0 ]; then
        die "No tools are available in PATH. Installation may have failed."
    fi
    
    # Check for Cursor workflows
    [ -d "$KERNELLE_HOME/.cursor" ] && log_success "Cursor workflows installed"
    
    # Check for add-workflows command availability
    command -v add-workflows >/dev/null && log_success "add-workflows available via shell"
    
    log_success "$verified_count tools verified and ready to use"
}

# Integrate shell profile to auto-source add-workflows.sh
integrate_shell() {
  local profile
  if [ -f "$HOME/.zshrc" ]; then
    profile="$HOME/.zshrc"
  else
    profile="$HOME/.bash_profile"
  fi
  grep -Fq 'source ~/.kernelle/add-workflows.sh' "$profile" 2>/dev/null || {
    printf "\n# Kernelle Cursor workflows\n[ -f \"$HOME/.kernelle/add-workflows.sh\" ] && source \"$HOME/.kernelle/add-workflows.sh\"\n" >> "$profile"
    log_success "Added Cursor workflow integration to $profile"
  }
}

# Show getting started guide
show_getting_started() {
    printf "\n${GREEN}${BOLD}🎉 Kernelle Installation Complete!${NC}\n\n"
    
    printf "${CYAN}${BOLD}Meet Your New AI Development Team:${NC}\n\n"
    
    printf "${PURPLE}🔧 Jerrod${NC} - GitLab/GitHub MR Review Specialist\n"
    printf "   Start reviewing: ${BOLD}jerrod start <repo> <mr-number>${NC}\n\n"
    
    printf "${BLUE}❄️  Blizz${NC} - Lightning-Fast Knowledge Acquisition\n"
    printf "   Gather insights: ${BOLD}blizz investigate <topic>${NC}\n\n"
    
    
    printf "${PURPLE}🎨 Violet${NC} - Code Complexity Artisan\n"
    printf "   Check code quality: ${BOLD}violet analyze <file>${NC}\n\n"
    
    
    printf "${RED}🛡️  Sentinel${NC} - Security & Encryption Guardian\n"
    printf "   Secure your data: ${BOLD}sentinel encrypt <file>${NC}\n\n"
    
    printf "${CYAN}🪄  add-workflows${NC} - Cursor IDE Workflow Helper\n"
    printf "   Add workflows to current project: ${BOLD}add-workflows${NC}\n"
    printf "   Add workflows to specific path:   ${BOLD}add-workflows path/to/project${NC}\n"
    printf "   Remove workflows from project:   ${BOLD}rm-workflows${NC}\n"
    printf "   This creates symlinks from ~/.kernelle/.cursor to your project's .cursor directory.\n\n"
    
    printf "${CYAN}${BOLD}Next Steps:${NC}\n"
    printf "1. Try ${BOLD}jerrod --help${NC} to see available commands\n"
    printf "2. Explore ${BOLD}$KERNELLE_HOME${NC} for configuration and insights\n"
    printf "3. Visit the repository for full documentation and examples\n"
    printf "4. Consider setting up MCP integrations for Blizz\n\n"
    
    if ! echo ":$PATH:" | grep -q ":$INSTALL_DIR:"; then
        printf "${YELLOW}${BOLD}⚠️  PATH Notice:${NC}\n"
        printf "Add ${BOLD}$INSTALL_DIR${NC} to your PATH if tools aren't found:\n"
        printf "${BOLD}export PATH=\"\$PATH:$INSTALL_DIR\"${NC}\n\n"
    fi
    
    printf "${GREEN}Happy coding with your new AI development team! 🚀${NC}\n"
}

# Main installation function
main() {
    show_banner
    
    log_info "Starting Kernelle installation..."
    
    # Perform installation steps
    check_requirements
    setup_kernelle_home
    clone_repository
    build_and_install
    verify_installation
    integrate_shell
    
    # Show success message and getting started guide
    show_getting_started
    
    log_success "Kernelle installation completed successfully!"
}

# Run main function if script is executed directly
if [ "${0##*/}" = "setup.sh" ] || [ "$0" = "sh" ] || [ "$0" = "bash" ] || [ "$0" = "zsh" ]; then
    main "$@"
fi
