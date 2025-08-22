#!/bin/bash

# Script to count source code lines for badge generation
# Focuses on Rust source files in the crates/ directory

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

usage() {
    echo "Usage: $0 [--badge-only]"
    echo "  --badge-only  Output only the badge markdown (for CI use)"
    echo "  --help       Show this help message"
}

# Parse arguments
badge_only=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --badge-only)
            badge_only=true
            shift
            ;;
        --help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Function to count lines in source files
count_source_lines() {
    # Count lines in Rust source files within crates/ directory
    # Exclude test files and generated files for a cleaner count
    find crates \
        -name "*.rs" \
        -not -path "*/target/*" \
        -not -path "*/.git/*" \
        -not -name "*.test.rs" \
        -exec wc -l {} + 2>/dev/null | \
        tail -1 | \
        awk '{print $1}'
}

# Function to determine badge color based on line count
get_badge_color() {
    local lines=$1
    if [ "$lines" -lt 5000 ]; then
        echo "green"
    elif [ "$lines" -lt 10000 ]; then
        echo "yellow"  
    elif [ "$lines" -lt 20000 ]; then
        echo "orange"
    else
        echo "red"
    fi
}

# Function to format number with commas for readability
format_number() {
    local num=$1
    echo "$num" | sed ':a;s/\B[0-9]\{3\}\>/,&/;ta'
}

# Main execution
main() {
    local line_count
    line_count=$(count_source_lines)
    
    if [ -z "$line_count" ] || [ "$line_count" -eq 0 ]; then
        echo -e "${RED}Error: Could not count source lines${NC}" >&2
        exit 1
    fi
    
    local formatted_count
    formatted_count=$(format_number "$line_count")
    
    local badge_color
    badge_color=$(get_badge_color "$line_count")
    
    # Generate the badge markdown
    local badge_text="Source%20Lines-${line_count}-${badge_color}"
    local badge_url="https://img.shields.io/badge/${badge_text}?style=flat"
    local badge_markdown="![Source Lines](${badge_url})"
    
    if [ "$badge_only" = true ]; then
        echo "$badge_markdown"
    else
        echo -e "${GREEN}✓ Source code analysis complete${NC}"
        echo "  Total source lines: $formatted_count"
        echo "  Badge color: $badge_color"
        echo "  Badge markdown: $badge_markdown"
    fi
}

main "$@"
