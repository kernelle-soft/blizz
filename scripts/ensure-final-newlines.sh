#!/bin/bash

# ensure-final-newlines.sh - Ensure all project files end with a final newline
# This addresses the requirement for consistent file formatting across all file types

set -uo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if file needs a final newline
file_needs_newline() {
    local file="$1"
    [[ -s "$file" && "$(tail -c1 "$file" | wc -l)" -eq 0 ]]
}

# Fix missing newline in file
fix_file_newline() {
    local file="$1"
    echo "" >> "$file"
    echo -e "${GREEN}✓ Fixed${NC} $file (added final newline)"
}

# Report missing newline
report_missing_newline() {
    local file="$1"
    echo -e "${RED}✗ Missing${NC} $file (missing final newline)"
}

# Function to check and fix final newline
check_and_fix_file() {
    local file="$1"
    local fix_mode="${2:-false}"
    
    if ! file_needs_newline "$file"; then
        return 0  # OK
    fi
    
    if [[ "$fix_mode" == "true" ]]; then
        fix_file_newline "$file"
    else
        report_missing_newline "$file"
    fi
    
    return 1  # Fixed or Error
}

# Process a single command line option
process_newlines_option() {
    case $1 in
        --fix)
            echo "true"
            ;;
        --check)
            echo "false"
            ;;
        *)
            show_usage
            exit 1
            ;;
    esac
}

# Parse command line arguments
parse_arguments() {
    local fix_mode=false
    
    while [[ $# -gt 0 ]]; do
        fix_mode=$(process_newlines_option "$1")
        shift
    done
    
    echo "$fix_mode"
}

# Show usage information
show_usage() {
    echo "Usage: $0 [--fix|--check]"
    echo "  --fix   Fix files by adding missing final newlines"
    echo "  --check Check files and report missing final newlines"
}

# violet ignore chunk - Long find command with many file patterns creates artificial complexity from bash syntax
# Find all relevant project files
find_project_files() {
    find . \
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
        | sort
}

# Process all files and count issues
process_files() {
    local fix_mode="$1"
    local -a files
    mapfile -t files < <(find_project_files)
    
    local total_files=${#files[@]}
    local issues=0
    
    echo "Found $total_files files to check..." >&2
    
    for file in "${files[@]}"; do
        if ! check_and_fix_file "$file" "$fix_mode"; then
            ((issues++))
        fi
    done
    
    echo "$issues:$total_files"
}

# Report final results
report_results() {
    local issues="$1"
    local total_files="$2"
    local fix_mode="$3"
    
    if [[ $issues -eq 0 ]]; then
        echo -e "${GREEN}✓ All $total_files files have proper final newlines!${NC}"
        return 0
    else
        if [[ "$fix_mode" == "true" ]]; then
            echo -e "${YELLOW}✓ Fixed $issues files${NC}"
            return 0
        else
            echo -e "${RED}✗ Found $issues files missing final newlines${NC}"
            echo -e "Run: ${YELLOW}bash scripts/ensure-final-newlines.sh --fix${NC} to fix them"
            return 1
        fi
    fi
}

# Main function
main() {
    local fix_mode
    fix_mode=$(parse_arguments "$@")
    
    echo "Checking final newlines for all project files..."
    
    local result
    result=$(process_files "$fix_mode")
    local issues="${result%:*}"
    local total_files="${result#*:}"
    
    echo
    if report_results "$issues" "$total_files" "$fix_mode"; then
        exit 0
    else
        exit 1
    fi
}

main "$@" 
