use anyhow::{anyhow, Result};
use crate::session::SessionManager;
use crate::platform::{ReactionType, GitPlatform, github::GitHubPlatform};

pub async fn handle(comment: Option<String>) -> Result<()> {
  let session_manager = SessionManager::new()?;
  let session = session_manager.load_session()?
    .ok_or_else(|| anyhow!("No active review session found"))?;

  if session.platform != "github" {
    return Err(anyhow!("Reaction system currently only supported for GitHub"));
  }

  // Get the current thread
  let current_thread_id = session.thread_queue.front()
    .ok_or_else(|| anyhow!("No threads in queue"))?;

  // Create GitHub client - for now, use environment variable
  let token = std::env::var("GITHUB_TOKEN")
    .map_err(|_| anyhow!("GITHUB_TOKEN environment variable not set"))?;
  let github = GitHubPlatform::new(Some(token))?;

  let repo_parts: Vec<&str> = session.repository.full_name.split('/').collect();
  if repo_parts.len() != 2 {
    return Err(anyhow!("Invalid repository format"));
  }

  // Add memo reaction
  let success = github.add_reaction(
    repo_parts[0],
    repo_parts[1], 
    current_thread_id,
    ReactionType::Memo
  ).await?;

  if success {
    bentley::success(&format!("Added {} reaction to thread", ReactionType::Memo.emoji()));
    
    // Add comment if provided
    if let Some(comment_text) = comment {
      let comment_with_link = format!("📝: {} - {}", 
        session.merge_request.url, 
        comment_text
      );
      
      // TODO: Implement comment creation
      bentley::info(&format!("Would add comment: {}", comment_with_link));
    }
  } else {
    bentley::warn("Failed to add reaction");
  }

  Ok(())
} 