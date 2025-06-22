use crate::platform::github::GitHubPlatform;
use crate::platform::{
  detection::{detect_platform, PlatformType},
  GitPlatform,
};
use crate::session::{ReviewSession, SessionManager};
use anyhow::{anyhow, Result};
use sentinel::Sentinel;
use std::process::Stdio;
use tokio::process::Command;

pub async fn handle(
  repository: String,
  mr_number: u64,
  platform_override: Option<String>,
  github_token: Option<String>,
  gitlab_token: Option<String>,
) -> Result<()> {
  bentley::announce("Jerrod - The Reliable Guardian of Code Quality");
  bentley::info("Starting new merge request review session");

  // Check if there's already an active session
  let session_manager = SessionManager::new()?;
  if session_manager.session_exists() {
    return Err(anyhow!("Active session already exists. Use 'jerrod finish' to complete it first, or 'jerrod refresh' to restart."));
  }

  // Detect platform and parse repository info
  let repo_info = detect_platform(&repository)?;

  // Override platform if specified
  let platform_type = if let Some(platform) = platform_override {
    match platform.to_lowercase().as_str() {
      "github" => PlatformType::GitHub,
      "gitlab" => PlatformType::GitLab,
      _ => return Err(anyhow!("Unsupported platform: {}. Use 'github' or 'gitlab'", platform)),
    }
  } else {
    repo_info.platform
  };

  bentley::info(&format!("Detected platform: {:?}", platform_type));
  bentley::info(&format!("Repository: {}/{}", repo_info.owner, repo_info.repo));
  bentley::info(&format!("MR/PR number: {}", mr_number));

  // Create platform client with automatic credential setup
  let platform: Box<dyn GitPlatform> = match platform_type {
    PlatformType::GitHub => {
      let token = get_or_setup_credential("github", "token", github_token).await?;
      Box::new(GitHubPlatform::new(Some(token))?)
    }
    PlatformType::GitLab => {
      let _token = get_or_setup_credential("gitlab", "token", gitlab_token).await?;
      return Err(anyhow!("GitLab support not yet implemented"));
    }
  };

  bentley::info("Fetching repository information...");
  let repository_info = platform.get_repository(&repo_info.owner, &repo_info.repo).await?;

  bentley::info("Fetching merge request details...");
  let merge_request =
    platform.get_merge_request(&repo_info.owner, &repo_info.repo, mr_number).await?;

  bentley::info("Fetching discussions and review comments...");
  let discussions = platform.get_discussions(&repo_info.owner, &repo_info.repo, mr_number).await?;

  bentley::info("Fetching pipeline/workflow information...");
  let pipelines = platform.get_pipelines(&repo_info.owner, &repo_info.repo, "HEAD").await?;

  // Create session
  let session = ReviewSession::new(
    repository_info,
    merge_request,
    format!("{:?}", platform_type).to_lowercase(),
    discussions,
    pipelines,
  );

  // Save session
  session_manager.save_session(&session)?;

  bentley::success(&format!(
    "Review session started for {}: {} ({:?})",
    session.merge_request.number, session.merge_request.title, session.merge_request.state
  ));

  bentley::info(&format!("Threads in queue: {}", session.threads_remaining()));
  bentley::info("Use 'jerrod status' to see session details");
  bentley::info("Use 'jerrod peek' to see the next thread");

  Ok(())
}

/// Get credential with automatic setup if missing
async fn get_or_setup_credential(
  service: &str,
  key: &str,
  cli_provided: Option<String>,
) -> Result<String> {
  // 1. Check CLI argument first
  if let Some(token) = cli_provided {
    return Ok(token);
  }

  // 2. Check Sentinel keychain
  let sentinel = Sentinel::new();
  if let Ok(token) = sentinel.get_credential(service, key) {
    return Ok(token);
  }

  // 3. Check environment variables
  let env_var = format!("{}_TOKEN", service.to_uppercase());
  if let Ok(token) = std::env::var(&env_var) {
    if !token.is_empty() {
      return Ok(token);
    }
  }

  // 4. No credential found - trigger Sentinel setup
  bentley::warn(&format!(
    "No {} token found in CLI args, keychain, or environment variables",
    service
  ));
  bentley::info(&format!("🔐 Setting up {} credentials...", service));

  // Spawn Sentinel setup command
  let mut child = Command::new("cargo")
    .args(["run", "--bin", "sentinel", "--", "setup", service])
    .stdin(Stdio::inherit())
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .spawn()?;

  let status = child.wait().await?;

  if !status.success() {
    anyhow::bail!("Credential setup failed or was cancelled");
  }

  // 5. Try to retrieve the credential again after setup
  match sentinel.get_credential(service, key) {
    Ok(token) => {
      bentley::success(&format!("✅ {} credentials configured successfully!", service));
      Ok(token)
    }
    Err(_) => {
      anyhow::bail!("Credential setup completed but token not found. Please try again.");
    }
  }
}
