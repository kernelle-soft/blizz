use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub async fn execute(target_dir: &str) -> Result<()> {
  let target_path = Path::new(target_dir);
  let cursor_dir = target_path.join(".cursor");
  let rules_dir = cursor_dir.join("rules");
  let blizz_home = get_blizz_home()?;

  if !cursor_dir.exists() {
    println!("No .cursor directory found in {}", target_path.display());
    return Ok(());
  }

  println!("Removing Blizz cursor workflows from {}...", target_path.display());

  // Check for the blizz symlink in .cursor/rules/
  let blizz_link = rules_dir.join("blizz");
  let blizz_cursor_path = blizz_home.join("volatile").join(".cursor").join("rules").join("blizz");

  if blizz_link.exists() && blizz_link.is_symlink() {
    // Check if it points to ~/.blizz/volatile/.cursor/rules/blizz
    if let Ok(target) = fs::read_link(&blizz_link) {
      if target == blizz_cursor_path {
        fs::remove_file(&blizz_link)
          .with_context(|| format!("Failed to remove symlink: {}", blizz_link.display()))?;
        println!("  Removed: .cursor/rules/blizz/");
      } else {
        println!("  Skipped: .cursor/rules/blizz/ points to {}, not Blizz", target.display());
      }
    }
  } else if blizz_link.exists() {
    println!("  Skipped: .cursor/rules/blizz/ exists but is not a symlink");
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

fn get_blizz_home() -> Result<PathBuf> {
  if let Ok(home) = std::env::var("BLIZZ_HOME") {
    Ok(PathBuf::from(home))
  } else if let Some(user_home) = dirs::home_dir() {
    Ok(user_home.join(".blizz"))
  } else {
    anyhow::bail!("Could not determine home directory")
  }
}

fn is_dir_empty(dir: &Path) -> Result<bool> {
  let mut entries =
    fs::read_dir(dir).with_context(|| format!("Failed to read directory: {}", dir.display()))?;
  Ok(entries.next().is_none())
}
