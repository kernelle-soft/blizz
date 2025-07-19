use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Creates a cross-platform symlink/junction
fn create_cross_platform_symlink(src: &Path, dst: &Path) -> std::io::Result<()> {
  #[cfg(unix)]
  {
    std::os::unix::fs::symlink(src, dst)
  }

  #[cfg(windows)]
  {
    // On Windows, try symlink_dir first, fall back to copying if it fails
    // (symlinks require admin privileges on Windows)
    match std::os::windows::fs::symlink_dir(src, dst) {
      Ok(()) => Ok(()),
      Err(_) => {
        // Fall back to creating a junction using the junction crate if available,
        // or just copy the directory structure
        if src.is_dir() {
          copy_dir_recursive(src, dst)
        } else {
          std::fs::copy(src, dst).map(|_| ())
        }
      }
    }
  }
}

#[cfg(windows)]
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
  std::fs::create_dir_all(dst)?;
  for entry in std::fs::read_dir(src)? {
    let entry = entry?;
    let src_path = entry.path();
    let dst_path = dst.join(entry.file_name());

    if src_path.is_dir() {
      copy_dir_recursive(&src_path, &dst_path)?;
    } else {
      std::fs::copy(&src_path, &dst_path)?;
    }
  }
  Ok(())
}

pub async fn execute(target_dir: &str) -> Result<()> {
  let target_path = Path::new(target_dir);
  let kernelle_home = get_kernelle_home()?;
  let cursor_source = kernelle_home.join(".cursor").join("rules").join("kernelle");

  if !cursor_source.exists() {
    anyhow::bail!(
            "Kernelle cursor workflows not found at {}/.cursor/rules/kernelle\nPlease run the Kernelle setup script first.",
            kernelle_home.display()
        );
  }

  // Create .cursor/rules directory if it doesn't exist
  let cursor_target = target_path.join(".cursor");
  let rules_target = cursor_target.join("rules");
  fs::create_dir_all(&rules_target)
    .with_context(|| format!("Failed to create directory: {}", rules_target.display()))?;

  println!("Adding Kernelle cursor workflows to {}...", target_path.display());

  // Create single symlink: .cursor/rules/kernelle/ -> ~/.kernelle/.cursor/rules/kernelle/
  let kernelle_link = rules_target.join("kernelle");

  // Remove existing kernelle symlink/directory if it exists
  // Use symlink_metadata to detect symlinks even if they're broken
  if let Ok(metadata) = fs::symlink_metadata(&kernelle_link) {
    if metadata.is_symlink() {
      fs::remove_file(&kernelle_link).with_context(|| {
        format!("Failed to remove existing symlink: {}", kernelle_link.display())
      })?;
    } else if metadata.is_dir() {
      anyhow::bail!("Directory .cursor/rules/kernelle/ already exists and is not a symlink. Please remove it manually.");
    } else {
      anyhow::bail!(
        "File .cursor/rules/kernelle already exists and is not a symlink. Please remove it manually."
      );
    }
  }

  // Create the symlink (cross-platform)
  create_cross_platform_symlink(&cursor_source, &kernelle_link).with_context(|| {
    format!("Failed to create symlink: {} -> {}", cursor_source.display(), kernelle_link.display())
  })?;

  println!("  Linked: .cursor/rules/kernelle/ -> {}", cursor_source.display());
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
