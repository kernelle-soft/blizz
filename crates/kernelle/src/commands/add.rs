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

  // Create single symlink: .cursor/kernelle/ -> ~/.kernelle/.cursor/
  let kernelle_link = cursor_target.join("kernelle");

  // Remove existing kernelle symlink/directory if it exists
  // Use symlink_metadata to detect symlinks even if they're broken
  if let Ok(metadata) = fs::symlink_metadata(&kernelle_link) {
    if metadata.is_symlink() {
      fs::remove_file(&kernelle_link).with_context(|| {
        format!("Failed to remove existing symlink: {}", kernelle_link.display())
      })?;
    } else if metadata.is_dir() {
      anyhow::bail!("Directory .cursor/kernelle/ already exists and is not a symlink. Please remove it manually.");
    } else {
      anyhow::bail!(
        "File .cursor/kernelle already exists and is not a symlink. Please remove it manually."
      );
    }
  }

  // Create the symlink
  std::os::unix::fs::symlink(&cursor_source, &kernelle_link).with_context(|| {
    format!("Failed to create symlink: {} -> {}", cursor_source.display(), kernelle_link.display())
  })?;

  println!("  Linked: .cursor/kernelle/ -> {}", cursor_source.display());
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
