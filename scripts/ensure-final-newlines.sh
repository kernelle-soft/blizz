#!/bin/bash

# ensure-final-newlines.sh - Ensure all project files end with a final newline
# This addresses the requirement for consistent file formatting across all file types

set -uo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to check and fix final newline
check_and_fix_file() {
    local file="$1"
    local fix_mode="${2:-false}"
    
    # Skip empty files
    if [[ ! -s "$file" ]]; then
        return 0
    fi
    
    # Check if file ends with newline
    if [[ "$(tail -c1 "$file" | wc -l)" -eq 0 ]]; then
        if [[ "$fix_mode" == "true" ]]; then
            echo "" >> "$file"
            echo -e "${GREEN}✓ Fixed${NC} $file (added final newline)"
            return 1  # Fixed
        else
            echo -e "${RED}✗ Missing${NC} $file (missing final newline)"
            return 1  # Error
        fi
    fi
    
    return 0  # OK
}

# Main function
main() {
    local fix_mode=false
    local exit_code=0
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --fix)
                fix_mode=true
                shift
                ;;
            --check)
                fix_mode=false
                shift
                ;;
            *)
                echo "Usage: $0 [--fix|--check]"
                echo "  --fix   Fix files by adding missing final newlines"
                echo "  --check Check files and report missing final newlines"
                exit 1
                ;;
        esac
    done
    
    echo "Checking final newlines for all project files..."
    
    # Find all relevant files, excluding build artifacts and dependencies
    mapfile -t files < <(find . \
        -type f \
        \( -name "*.rs" -o -name "*.yml" -o -name "*.yaml" -o -name "*.toml" -o -name "*.sh" -o -name "*.md" -o -name "*.json" -o -name "*.mdc" \) \
        -not -path "./target/*" \
        -not -path "./coverage/*" \
        -not -path "./.git/*" \
        -not -path "./tmp/*" \
        -not -path "./temp/*" \
        -not -path "./.cache/*" \
        -not -name "*.lock" \
        -not -name "*.log" \
        -not -name "Cargo.lock" \
        | sort)
    
    local total_files=${#files[@]}
    local issues=0
    
    echo "Found $total_files files to check..."
    
    for file in "${files[@]}"; do
        if ! check_and_fix_file "$file" "$fix_mode"; then
            ((issues++))
        fi
    done
    
    echo
    if [[ $issues -eq 0 ]]; then
        echo -e "${GREEN}✓ All $total_files files have proper final newlines!${NC}"
    else
        if [[ "$fix_mode" == "true" ]]; then
            echo -e "${YELLOW}✓ Fixed $issues files${NC}"
        else
            echo -e "${RED}✗ Found $issues files missing final newlines${NC}"
            echo -e "Run: ${YELLOW}bash scripts/ensure-final-newlines.sh --fix${NC} to fix them"
            exit_code=1
        fi
    fi
    
    exit $exit_code
}

main "$@" 
