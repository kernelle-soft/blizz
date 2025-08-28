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
  let blizz_home = get_blizz_home()?;
  let cursor_source = blizz_home.join("volatile").join(".cursor").join("rules").join("blizz");

  if !cursor_source.exists() {
    anyhow::bail!(
            "Blizz cursor workflows not found at {}/volatile/.cursor/rules/blizz\nPlease run the Blizz setup script first.",
            blizz_home.display()
        );
  }

  // Create .cursor/rules directory if it doesn't exist
  let cursor_target = target_path.join(".cursor");
  let rules_target = cursor_target.join("rules");
  fs::create_dir_all(&rules_target)
    .with_context(|| format!("Failed to create directory: {}", rules_target.display()))?;

  println!("Adding Blizz cursor workflows to {}...", target_path.display());

  // Create single symlink: .cursor/rules/blizz/ -> ~/.blizz/volatile/.cursor/rules/blizz/
  let blizz_link = rules_target.join("blizz");

  // Remove existing blizz symlink/directory if it exists
  // Use symlink_metadata to detect symlinks even if they're broken
  if let Ok(metadata) = fs::symlink_metadata(&blizz_link) {
    if metadata.is_symlink() {
      fs::remove_file(&blizz_link)
        .with_context(|| format!("Failed to remove existing symlink: {}", blizz_link.display()))?;
    } else if metadata.is_dir() {
      anyhow::bail!("Directory .cursor/rules/blizz/ already exists and is not a symlink. Please remove it manually.");
    } else {
      anyhow::bail!(
        "File .cursor/rules/blizz already exists and is not a symlink. Please remove it manually."
      );
    }
  }

  // Create the symlink (cross-platform)
  create_cross_platform_symlink(&cursor_source, &blizz_link).with_context(|| {
    format!("Failed to create symlink: {} -> {}", cursor_source.display(), blizz_link.display())
  })?;

  println!("  Linked: .cursor/rules/blizz/ -> {}", cursor_source.display());
  println!("Cursor workflows added successfully!");
  println!("Open this project in Cursor to access Blizz rules and workflows.");

  Ok(())
}

fn get_blizz_home() -> Result<PathBuf> {
  if let Ok(home) = std::env::var("BLIZZ_HOME") {
    Ok(PathBuf::from(home))
  } else if let Some(user_home) = dirs::home_dir() {
    Ok(user_home.join(".blizz"))
  } else {
    anyhow::bail!("Could not determine home directory")
  }
}
