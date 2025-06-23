use crate::platform::{
  detection::{detect_platform, PlatformType},
  GitPlatform, create_platform,
};
use crate::session::{ReviewSession, SessionManager, SessionDiscovery};
use anyhow::{anyhow, Result};

pub async fn handle(
  repository: String,
  mr_number: u64,
  platform_override: Option<String>,
  _github_token: Option<String>,
  _gitlab_token: Option<String>,
) -> Result<()> {
  bentley::announce("Jerrod - The Reliable Guardian of Code Quality");
  bentley::info("Starting new merge request review session");

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

  // Check if there's already an active session
  let discovery = SessionDiscovery::new()?;
  if discovery.find_any_session()?.is_some() {
    return Err(anyhow!("Active session already exists. Use 'jerrod finish' to complete it first, or 'jerrod refresh' to restart."));
  }

  // Set up session manager for the new session
  let mut session_manager = SessionManager::new()?;
  let platform_name = format!("{:?}", platform_type).to_lowercase();
  let repository_path = format!("{}/{}", repo_info.owner, repo_info.repo);
  session_manager.with_session_context(&platform_name, &repository_path, mr_number)?;

  bentley::info(&format!("Detected platform: {:?}", platform_type));
  bentley::info(&format!("Repository: {}/{}", repo_info.owner, repo_info.repo));
  bentley::info(&format!("MR/PR number: {}", mr_number));

  // Create platform client with automatic credential setup using strategy pattern
  let platform_name = format!("{:?}", platform_type).to_lowercase();
  let platform = create_platform(&platform_name).await?;

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
    platform_name,
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
