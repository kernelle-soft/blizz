use anyhow::Result;
use std::io::Write;

/// Execute version command
pub async fn execute(list: bool) -> Result<()> {
  let mut stdout = std::io::stdout();
  if list {
    show_available_versions(&mut stdout).await
  } else {
    show_current_version(&mut stdout).await
  }
}

/// Show the current version
async fn show_current_version<W: Write>(writer: &mut W) -> Result<()> {
  let version = env!("CARGO_PKG_VERSION");
  writeln!(writer, "kernelle {version}")?;
  Ok(())
}

/// Show all available versions from GitHub releases
async fn show_available_versions<W: Write>(writer: &mut W) -> Result<()> {
  let current_version = env!("CARGO_PKG_VERSION");
  writeln!(writer, "Current version: kernelle {current_version}")?;
  writeln!(writer)?;

  // TODO: Implement GitHub API call to fetch available releases
  writeln!(writer, "Available versions:")?;
  writeln!(writer, "  {current_version} (current)")?;
  writeln!(writer)?;
  writeln!(writer, "Note: Release listing not yet implemented")?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_show_current_version() -> Result<()> {
    // Test that show_current_version outputs the correct version
    let mut output = Vec::new();
    let result = show_current_version(&mut output).await;

    assert!(result.is_ok());
    let output_str = String::from_utf8(output)?;
    let expected_version = env!("CARGO_PKG_VERSION");
    assert!(output_str.contains(&format!("kernelle {expected_version}")));
    Ok(())
  }

  #[tokio::test]
  async fn test_show_available_versions() -> Result<()> {
    // Test that show_available_versions outputs the current version
    let mut output = Vec::new();
    let result = show_available_versions(&mut output).await;

    assert!(result.is_ok());
    let output_str = String::from_utf8(output)?;
    let expected_version = env!("CARGO_PKG_VERSION");
    assert!(output_str.contains(&format!("Current version: kernelle {expected_version}")));
    assert!(output_str.contains("Available versions:"));
    Ok(())
  }

  #[tokio::test]
  async fn test_execute_basic_version() -> Result<()> {
    // Test version command without list flag
    let result = execute(false).await;
    assert!(result.is_ok());
    Ok(())
  }

  #[tokio::test]
  async fn test_execute_version_list() -> Result<()> {
    // Test version command with list flag
    let result = execute(true).await;
    assert!(result.is_ok());
    Ok(())
  }
}
