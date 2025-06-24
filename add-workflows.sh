#!/bin/sh

# Kernelle Cursor Workflows Integration
# This script provides functions to add/remove Cursor workflows to/from projects

# Check if Kernelle home exists
KERNELLE_HOME="${KERNELLE_HOME:-$HOME/.kernelle}"

# Function to add Cursor workflows to current project
add-workflows() {
    local target_dir="${1:-.}"
    
    if [ ! -d "$KERNELLE_HOME/.cursor" ]; then
        echo "Error: Kernelle Cursor workflows not found at $KERNELLE_HOME/.cursor"
        echo "Please run the Kernelle setup script first."
        return 1
    fi
    
    # Create .cursor directory if it doesn't exist
    mkdir -p "$target_dir/.cursor"
    
    echo "Adding Kernelle Cursor workflows to $target_dir..."
    
    # Recursively create symlinks for all files in ~/.kernelle/.cursor
    find "$KERNELLE_HOME/.cursor" -type f | while read -r source_file; do
        # Get relative path from ~/.kernelle/.cursor
        rel_path="${source_file#$KERNELLE_HOME/.cursor/}"
        target_file="$target_dir/.cursor/$rel_path"
        target_parent="$(dirname "$target_file")"
        
        # Create parent directory if needed
        mkdir -p "$target_parent"
        
        # Create symlink (remove existing file/link first)
        rm -f "$target_file"
        ln -s "$source_file" "$target_file"
        echo "  Linked: .cursor/$rel_path"
    done
    
    echo "Cursor workflows added successfully!"
    echo "Open this project in Cursor to access Kernelle workflows."
}

# Function to remove Cursor workflows from current project
rm-workflows() {
    local target_dir="${1:-.}"
    
    if [ ! -d "$target_dir/.cursor" ]; then
        echo "No .cursor directory found in $target_dir"
        return 0
    fi
    
    echo "Removing Kernelle Cursor workflows from $target_dir..."
    
    # Find and remove symlinks that point to ~/.kernelle/.cursor
    find "$target_dir/.cursor" -type l | while read -r link; do
        if readlink "$link" | grep -q "^$KERNELLE_HOME/.cursor"; then
            rm -f "$link"
            echo "  Removed: ${link#$target_dir/}"
        fi
    done
    
    # Remove empty directories
    find "$target_dir/.cursor" -type d -empty -delete 2>/dev/null || true
    
    # Remove .cursor directory if it's empty
    rmdir "$target_dir/.cursor" 2>/dev/null && echo "  Removed empty .cursor directory" || true
    
    echo "Cursor workflows removed successfully!"
}

# Export functions so they're available in the shell
export -f add-workflows 2>/dev/null || true
export -f rm-workflows 2>/dev/null || true
