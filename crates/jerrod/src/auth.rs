use anyhow::{anyhow, Result};
use sentinel::Sentinel;

/// Get GitHub token with fallback priority: Sentinel -> Environment
pub async fn get_github_token() -> Result<String> {
  // 1. Check Sentinel daemon first, fall back to keychain
  let sentinel = Sentinel::new();
  if let Ok(token) = sentinel.get_credential_async("github", "token").await {
    return Ok(token);
  }

  // 2. Check environment variables
  if let Ok(token) = std::env::var("GITHUB_TOKEN") {
    if !token.is_empty() {
      return Ok(token);
    }
  }

  Err(anyhow!("No GitHub token found. Use 'sentinel setup github' to configure credentials"))
} 