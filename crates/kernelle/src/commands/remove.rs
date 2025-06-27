use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub async fn execute(target_dir: &str) -> Result<()> {
    let target_path = Path::new(target_dir);
    let cursor_dir = target_path.join(".cursor");
    let kernelle_home = get_kernelle_home()?;

    if !cursor_dir.exists() {
        println!("No .cursor directory found in {}", target_path.display());
        return Ok(());
    }

    println!("Removing Kernelle cursor workflows from {}...", target_path.display());

    // Find and remove symlinks that point to ~/.kernelle/.cursor
    remove_kernelle_symlinks(&cursor_dir, &kernelle_home)?;

    // Remove empty directories
    remove_empty_dirs(&cursor_dir)?;

    // Remove .cursor directory if it's empty
    if is_dir_empty(&cursor_dir)? {
        fs::remove_dir(&cursor_dir)
            .with_context(|| format!("Failed to remove directory: {}", cursor_dir.display()))?;
        println!("  Removed empty .cursor directory");
    }

    println!("Cursor workflows removed successfully!");

    Ok(())
}

fn get_kernelle_home() -> Result<PathBuf> {
    if let Ok(home) = std::env::var("KERNELLE_HOME") {
        Ok(PathBuf::from(home))
    } else if let Some(user_home) = dirs::home_dir() {
        Ok(user_home.join(".kernelle"))
    } else {
        anyhow::bail!("Could not determine home directory")
    }
}

fn remove_kernelle_symlinks(cursor_dir: &Path, kernelle_home: &Path) -> Result<()> {
    let kernelle_cursor_path = kernelle_home.join(".cursor");
    
    for entry in walkdir::WalkDir::new(cursor_dir) {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_symlink() {
            if let Ok(target) = fs::read_link(path) {
                // Check if the symlink points to something under ~/.kernelle/.cursor
                if target.starts_with(&kernelle_cursor_path) {
                    fs::remove_file(path)
                        .with_context(|| format!("Failed to remove symlink: {}", path.display()))?;
                    
                    let relative_path = path.strip_prefix(cursor_dir.parent().unwrap_or(cursor_dir))
                        .unwrap_or(path);
                    println!("  Removed: {}", relative_path.display());
                }
            }
        }
    }
    
    Ok(())
}

fn remove_empty_dirs(dir: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(dir).contents_first(true) {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() && path != dir && is_dir_empty(path)? {
            fs::remove_dir(path)
                .with_context(|| format!("Failed to remove empty directory: {}", path.display()))?;
        }
    }
    
    Ok(())
}

fn is_dir_empty(dir: &Path) -> Result<bool> {
    let mut entries = fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;
    Ok(entries.next().is_none())
} 