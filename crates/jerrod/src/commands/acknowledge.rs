use anyhow::{anyhow, Result};
use crate::session::SessionManager;
use crate::platform::{ReactionType, GitPlatform, github::GitHubPlatform};

pub async fn handle() -> Result<()> {
  let session_manager = SessionManager::new()?;
  let session = session_manager.load_session()?
    .ok_or_else(|| anyhow!("No active review session found"))?;

  if session.platform != "github" {
    return Err(anyhow!("Reaction system currently only supported for GitHub"));
  }

  // Get the current thread
  let current_thread_id = session.thread_queue.front()
    .ok_or_else(|| anyhow!("No threads in queue"))?;

  // Create GitHub client with credential lookup
  let github = GitHubPlatform::new().await?;

  // Add eyes reaction
  let repo_parts: Vec<&str> = session.repository.full_name.split('/').collect();
  if repo_parts.len() != 2 {
    return Err(anyhow!("Invalid repository format"));
  }

  let success = github.add_reaction(
    repo_parts[0],
    repo_parts[1], 
    current_thread_id,
    ReactionType::Eyes
  ).await?;

  if success {
    bentley::success(&format!("Added {} reaction to thread", ReactionType::Eyes.emoji()));
  } else {
    bentley::warn("Failed to add reaction");
  }

  Ok(())
} 