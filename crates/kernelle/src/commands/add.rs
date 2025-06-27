use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub async fn execute(target_dir: &str) -> Result<()> {
    let target_path = Path::new(target_dir);
    let kernelle_home = get_kernelle_home()?;
    let cursor_source = kernelle_home.join(".cursor");

    if !cursor_source.exists() {
        anyhow::bail!(
            "Kernelle cursor workflows not found at {}/.cursor\nPlease run the Kernelle setup script first.",
            kernelle_home.display()
        );
    }

    // Create .cursor directory if it doesn't exist
    let cursor_target = target_path.join(".cursor");
    fs::create_dir_all(&cursor_target)
        .with_context(|| format!("Failed to create directory: {}", cursor_target.display()))?;

    println!("Adding Kernelle cursor workflows to {}...", target_path.display());

    // Recursively create symlinks for all files in ~/.kernelle/.cursor
    create_symlinks(&cursor_source, &cursor_target)?;

    println!("Cursor workflows added successfully!");
    println!("Open this project in Cursor to access Kernelle workflows.");

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

fn create_symlinks(source_dir: &Path, target_dir: &Path) -> Result<()> {
    for entry in fs::read_dir(source_dir)
        .with_context(|| format!("Failed to read directory: {}", source_dir.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let file_name = source_path.file_name().unwrap();
        let target_path = target_dir.join(file_name);

        if source_path.is_dir() {
            // Create target directory and recurse
            fs::create_dir_all(&target_path)
                .with_context(|| format!("Failed to create directory: {}", target_path.display()))?;
            create_symlinks(&source_path, &target_path)?;
        } else {
            // Remove existing file/link if it exists
            if target_path.exists() {
                fs::remove_file(&target_path)
                    .with_context(|| format!("Failed to remove existing file: {}", target_path.display()))?;
            }

            // Create symlink
            std::os::unix::fs::symlink(&source_path, &target_path)
                .with_context(|| format!("Failed to create symlink: {} -> {}", 
                    source_path.display(), target_path.display()))?;

            // Get relative path for nice output
            let relative_path = target_path.strip_prefix(target_dir.parent().unwrap_or(target_dir))
                .unwrap_or(&target_path);
            println!("  Linked: {}", relative_path.display());
        }
    }
    Ok(())
} 