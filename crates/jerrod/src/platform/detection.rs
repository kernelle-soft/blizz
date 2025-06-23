use anyhow::{anyhow, Result};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlatformType {
  GitHub,
  GitLab,
}

#[derive(Debug, Clone)]
pub struct RepositoryInfo {
  pub platform: PlatformType,
  pub owner: String,
  pub repo: String,
  #[allow(dead_code)]
  pub host: Option<String>, // For self-hosted instances
}

/// Detect the platform type and parse repository information from various input formats
pub fn detect_platform(input: &str) -> Result<RepositoryInfo> {
  // First, try to parse as a URL
  if let Ok(url) = Url::parse(input) {
    return parse_url(url);
  }

  // If not a URL, try to parse as "owner/repo" format
  if let Some((owner, repo)) = input.split_once('/') {
    if !owner.is_empty() && !repo.is_empty() {
      // Default to GitHub for owner/repo format
      return Ok(RepositoryInfo {
        platform: PlatformType::GitHub,
        owner: owner.to_string(),
        repo: repo.to_string(),
        host: None,
      });
    }
  }

  Err(anyhow!("Invalid repository format. Use 'owner/repo' or a full URL"))
}

fn parse_url(url: Url) -> Result<RepositoryInfo> {
  let host = url.host_str().ok_or_else(|| anyhow!("Invalid URL: no host"))?;

  let platform = detect_platform_from_host(host)?;

  let path = url.path().trim_start_matches('/').trim_end_matches('/');
  let path_parts: Vec<&str> = path.split('/').collect();

  if path_parts.len() < 2 {
    return Err(anyhow!("Invalid repository URL: path should contain owner/repo"));
  }

  let owner = path_parts[0];
  let repo = path_parts[1];

  if owner.is_empty() || repo.is_empty() {
    return Err(anyhow!("Invalid repository URL: empty owner or repo name"));
  }

  Ok(RepositoryInfo {
    platform,
    owner: owner.to_string(),
    repo: repo.to_string(),
    host: if (matches!(platform, PlatformType::GitHub) && host == "github.com")
      || (matches!(platform, PlatformType::GitLab) && host == "gitlab.com")
    {
      None // Use default host
    } else {
      Some(host.to_string()) // Custom host
    },
  })
}

fn detect_platform_from_host(host: &str) -> Result<PlatformType> {
  match host {
    "github.com" => Ok(PlatformType::GitHub),
    "gitlab.com" => Ok(PlatformType::GitLab),
    // For self-hosted instances, we'll need more sophisticated detection
    // For now, default to GitLab for unknown hosts (common for self-hosted GitLab)
    _ => {
      if host.contains("gitlab") {
        Ok(PlatformType::GitLab)
      } else {
        // Could be GitHub Enterprise, but default to GitHub
        Ok(PlatformType::GitHub)
      }
    }
  }
}



#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_github_url() {
    let info = detect_platform("https://github.com/owner/repo").unwrap();
    assert_eq!(info.platform, PlatformType::GitHub);
    assert_eq!(info.owner, "owner");
    assert_eq!(info.repo, "repo");
    assert_eq!(info.host, None);
  }

  #[test]
  fn test_gitlab_url() {
    let info = detect_platform("https://gitlab.com/owner/repo").unwrap();
    assert_eq!(info.platform, PlatformType::GitLab);
    assert_eq!(info.owner, "owner");
    assert_eq!(info.repo, "repo");
    assert_eq!(info.host, None);
  }

  #[test]
  fn test_owner_repo_format() {
    let info = detect_platform("owner/repo").unwrap();
    assert_eq!(info.platform, PlatformType::GitHub); // Defaults to GitHub
    assert_eq!(info.owner, "owner");
    assert_eq!(info.repo, "repo");
    assert_eq!(info.host, None);
  }


}
