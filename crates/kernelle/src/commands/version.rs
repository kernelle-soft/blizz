use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::io::Write;

const GITHUB_RELEASE_API: &str = "https://api.github.com/repos/TravelSizedLions/kernelle/releases";

#[derive(Debug, Deserialize)]
struct Release {
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
        writeln!(writer, "No releases found")?;
        return Ok(());
      }

      writeln!(writer, "Available versions:")?;

      // Filter out drafts and prereleases, then sort by version
      let mut stable_releases: Vec<_> =
        releases.into_iter().filter(|r| !r.draft && !r.prerelease).collect();

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
          writeln!(writer, "  {version} (current)")?;
        } else {
          writeln!(writer, "  {version}")?;
        }
      }
    }
    Err(e) => {
      writeln!(writer, "Failed to fetch releases: {e}")?;
      writeln!(writer)?;
      writeln!(writer, "Available versions:")?;
      writeln!(writer, "  {current_version} (current)")?;
      writeln!(writer)?;
      writeln!(writer, "Note: Unable to fetch remote release information")?;
    }
  }

  Ok(())
}

/// Fetch releases from GitHub API
async fn fetch_github_releases() -> Result<Vec<Release>> {
  fetch_releases_from_url(GITHUB_RELEASE_API).await
}

/// Fetch releases from a given URL (for testing)
async fn fetch_releases_from_url(url: &str) -> Result<Vec<Release>> {
  let client = reqwest::Client::new();
  let response = client
    .get(url)
    .header("User-Agent", "kernelle")
    .header("Accept", "application/vnd.github.v3+json")
    .send()
    .await
    .map_err(|e| anyhow!("Failed to fetch releases: {}", e))?;

  if !response.status().is_success() {
    return Err(anyhow!("API request failed with status: {}", response.status()));
  }

  let releases: Vec<Release> =
    response.json().await.map_err(|e| anyhow!("Failed to parse API response: {}", e))?;

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
  async fn test_fetch_releases_with_mock_success() {
    use mockito::Server;

    let mut server = Server::new_async().await;
    let mock_response = r#"[
      {
        "tag_name": "v1.2.0",
        "prerelease": false,
        "draft": false
      },
      {
        "tag_name": "v1.1.0",
        "prerelease": false,
        "draft": false
      },
      {
        "tag_name": "v1.0.0-beta.1",
        "prerelease": true,
        "draft": false
      }
    ]"#;

    let _mock = server
      .mock("GET", "/releases")
      .with_status(200)
      .with_header("content-type", "application/json")
      .with_body(mock_response)
      .create_async()
      .await;

    let mock_url = format!("{}/releases", server.url());
    let result = fetch_releases_from_url(&mock_url).await;

    assert!(result.is_ok());
    let releases = result.unwrap();
    assert_eq!(releases.len(), 3);
    assert_eq!(releases[0].tag_name, "v1.2.0");
    assert!(!releases[0].prerelease);
    assert!(!releases[0].draft);
    assert!(releases[2].prerelease); // The beta version
  }

  #[tokio::test]
  async fn test_fetch_releases_with_mock_error() {
    use mockito::Server;

    let mut server = Server::new_async().await;
    let _mock = server.mock("GET", "/releases").with_status(404).create_async().await;

    let mock_url = format!("{}/releases", server.url());
    let result = fetch_releases_from_url(&mock_url).await;

    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_show_available_versions_with_mock() {
    use mockito::Server;

    let mut server = Server::new_async().await;
    let mock_response = r#"[
      {
        "tag_name": "v0.3.0",
        "prerelease": false,
        "draft": false
      },
      {
        "tag_name": "v0.2.9",
        "prerelease": false,
        "draft": false
      }
    ]"#;

    let _mock = server
      .mock("GET", "/releases")
      .with_status(200)
      .with_header("content-type", "application/json")
      .with_body(mock_response)
      .create_async()
      .await;

    // Test the version display logic without hitting real GitHub
    let mock_url = format!("{}/releases", server.url());

    let result = fetch_releases_from_url(&mock_url).await;
    assert!(result.is_ok());

    let releases = result.unwrap();
    let stable_releases: Vec<_> =
      releases.into_iter().filter(|r| !r.draft && !r.prerelease).collect();

    assert_eq!(stable_releases.len(), 2);
    assert_eq!(stable_releases[0].tag_name, "v0.3.0");
  }
}
