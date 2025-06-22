use anyhow::{anyhow, Result};
use crate::session::SessionManager;
use crate::platform::{ReactionType, GitPlatform, github::GitHubPlatform};

pub async fn handle(
  text: String,
  new: bool,
  complete: bool,
  question: bool,
  defer: bool,
) -> Result<()> {
  // Validate that only one reaction flag is set
  let reaction_flags = [complete, question, defer];
  if reaction_flags.iter().filter(|&&flag| flag).count() > 1 {
    return Err(anyhow!("Cannot specify multiple reaction flags (--complete, --question, --defer)"));
  }

  let session_manager = SessionManager::new()?;
  let session = session_manager.load_session()?
    .ok_or_else(|| anyhow!("No active review session found"))?;

  if session.platform != "github" {
    return Err(anyhow!("Comment system currently only supported for GitHub"));
  }

  // Get the current thread (if not creating a new comment)
  let current_thread_id = if new {
    None
  } else {
    session.thread_queue.front().cloned()
  };

  if !new && current_thread_id.is_none() {
    return Err(anyhow!("No threads in queue"));
  }

  // Create GitHub client
  let github = GitHubPlatform::new().await?;

  // Parse repository
  let repo_parts: Vec<&str> = session.repository.full_name.split('/').collect();
  if repo_parts.len() != 2 {
    return Err(anyhow!("Invalid repository format"));
  }

  // TODO: Implement actual comment creation
  bentley::info("Comment creation not yet implemented");

  // Add reaction if specified
  if let Some(thread_id) = &current_thread_id {
    let reaction = if complete {
      Some(ReactionType::CheckMark)
    } else if question {
      Some(ReactionType::Question)
    } else if defer {
      Some(ReactionType::Memo)
    } else {
      None
    };

    if let Some(reaction) = reaction {
      let success = github.add_reaction(
        repo_parts[0],
        repo_parts[1],
        thread_id,
        reaction.clone()
      ).await?;

      if success {
        bentley::success(&format!("Added {} reaction to thread", reaction.emoji()));
      } else {
        bentley::warn("Failed to add reaction");
      }
    }
  }

  bentley::info(&format!("Comment: {}", text));
  Ok(())
}
