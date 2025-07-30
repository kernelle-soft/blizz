use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::io::Write;

/// GitHub release information
#[derive(Debug, Deserialize)]
struct GitHubRelease {
  tag_name: String,
  prerelease: bool,
  draft: bool,
}

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

  // Fetch releases from GitHub API
  match fetch_github_releases().await {
    Ok(releases) => {
      if releases.is_empty() {
        writeln!(writer, "No releases found on GitHub")?;
        return Ok(());
      }

      writeln!(writer, "Available versions:")?;
      
      // Filter out drafts and prereleases, then sort by version
      let mut stable_releases: Vec<_> = releases
        .into_iter()
        .filter(|r| !r.draft && !r.prerelease)
        .collect();
      
      // Sort releases by version (newest first)
      // Note: This is a simple string sort, which works for semantic versions
      stable_releases.sort_by(|a, b| {
        // Extract version number from tag_name (remove 'v' prefix if present)
        let version_a = a.tag_name.strip_prefix('v').unwrap_or(&a.tag_name);
        let version_b = b.tag_name.strip_prefix('v').unwrap_or(&b.tag_name);
        
        // Sort in descending order (newest first)
        version_compare(version_b, version_a)
      });

      for release in stable_releases {
        let version = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
        let is_current = version == current_version;
        
        if is_current {
          writeln!(writer, "  {} (current)", version)?;
        } else {
          writeln!(writer, "  {}", version)?;
        }
      }
    }
    Err(e) => {
      writeln!(writer, "Failed to fetch releases from GitHub: {}", e)?;
      writeln!(writer)?;
      writeln!(writer, "Available versions:")?;
      writeln!(writer, "  {} (current)", current_version)?;
      writeln!(writer)?;
      writeln!(writer, "Note: Unable to fetch remote release information")?;
    }
  }

  Ok(())
}

/// Fetch releases from GitHub API
async fn fetch_github_releases() -> Result<Vec<GitHubRelease>> {
  let client = reqwest::Client::new();
  let url = "https://api.github.com/repos/TravelSizedLions/kernelle/releases";
  
  let response = client
    .get(url)
    .header("User-Agent", "kernelle")
    .header("Accept", "application/vnd.github.v3+json")
    .send()
    .await
    .map_err(|e| anyhow!("Failed to fetch releases: {}", e))?;

  if !response.status().is_success() {
    return Err(anyhow!("GitHub API request failed with status: {}", response.status()));
  }

  let releases: Vec<GitHubRelease> = response
    .json()
    .await
    .map_err(|e| anyhow!("Failed to parse GitHub API response: {}", e))?;

  Ok(releases)
}

/// Simple version comparison for semantic versions
/// Returns Ordering for use in sort_by
fn version_compare(a: &str, b: &str) -> std::cmp::Ordering {
  // Split versions into parts
  let parts_a: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
  let parts_b: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();
  
  // Compare each part
  let max_len = parts_a.len().max(parts_b.len());
  for i in 0..max_len {
    let part_a = parts_a.get(i).unwrap_or(&0);
    let part_b = parts_b.get(i).unwrap_or(&0);
    
    match part_a.cmp(part_b) {
      std::cmp::Ordering::Equal => continue,
      other => return other,
    }
  }
  
  std::cmp::Ordering::Equal
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

  #[test]
  fn test_version_compare() {
    use std::cmp::Ordering;
    
    // Test basic version comparison
    assert_eq!(version_compare("1.0.0", "1.0.0"), Ordering::Equal);
    assert_eq!(version_compare("1.0.1", "1.0.0"), Ordering::Greater);
    assert_eq!(version_compare("1.0.0", "1.0.1"), Ordering::Less);
    assert_eq!(version_compare("1.1.0", "1.0.9"), Ordering::Greater);
    assert_eq!(version_compare("2.0.0", "1.9.9"), Ordering::Greater);
  }

  #[tokio::test]
  async fn test_fetch_github_releases_handles_errors() {
    // This test verifies that the function handles errors gracefully
    // We can't easily mock the HTTP client in this simple test, but we can
    // verify that the function signature is correct and returns a Result
    let result = fetch_github_releases().await;
    // Result should be Ok (if network available) or Err (if no network)
    // Either is fine for this test - we just want to ensure it compiles and runs
    match result {
      Ok(releases) => {
        // If successful, releases should be a vector
        assert!(releases.iter().all(|r| !r.tag_name.is_empty()));
      }
      Err(_) => {
        // If it fails (e.g., no network), that's also acceptable for this test
        // The important thing is that it doesn't panic
      }
    }
  }
}
