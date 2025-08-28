use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub async fn execute(target_dir: &str) -> Result<()> {
  let target_path = Path::new(target_dir);
  let cursor_dir = target_path.join(".cursor");
  let rules_dir = cursor_dir.join("rules");
  let kernelle_home = get_kernelle_home()?;

  if !cursor_dir.exists() {
    println!("No .cursor directory found in {}", target_path.display());
    return Ok(());
  }

  println!("Removing Kernelle cursor workflows from {}...", target_path.display());

  // Check for the kernelle symlink in .cursor/rules/
  let kernelle_link = rules_dir.join("kernelle");
  let kernelle_cursor_path =
    kernelle_home.join("volatile").join(".cursor").join("rules").join("kernelle");

  if kernelle_link.exists() && kernelle_link.is_symlink() {
    // Check if it points to ~/.kernelle/volatile/.cursor/rules/kernelle
    if let Ok(target) = fs::read_link(&kernelle_link) {
      if target == kernelle_cursor_path {
        fs::remove_file(&kernelle_link)
          .with_context(|| format!("Failed to remove symlink: {}", kernelle_link.display()))?;
        println!("  Removed: .cursor/rules/kernelle/");
      } else {
        println!("  Skipped: .cursor/rules/kernelle/ points to {}, not Kernelle", target.display());
      }
    }
  } else if kernelle_link.exists() {
    println!("  Skipped: .cursor/rules/kernelle/ exists but is not a symlink");
  }

  // Remove .cursor/rules directory if it's empty
  if rules_dir.exists() && is_dir_empty(&rules_dir)? {
    fs::remove_dir(&rules_dir)
      .with_context(|| format!("Failed to remove directory: {}", rules_dir.display()))?;
    println!("  Removed empty .cursor/rules directory");
  }

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

fn is_dir_empty(dir: &Path) -> Result<bool> {
  let mut entries =
    fs::read_dir(dir).with_context(|| format!("Failed to read directory: {}", dir.display()))?;
  Ok(entries.next().is_none())
}
