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
  let reaction_count = reaction_flags.iter().filter(|&&flag| flag).count();
  
  if reaction_count > 1 {
    return Err(anyhow!("Only one reaction flag can be used at a time"));
  }

  let session_manager = SessionManager::new()?;
  let session = session_manager.load_session()?
    .ok_or_else(|| anyhow!("No active review session found"))?;

  // Determine reaction type if any
  let reaction_type = if complete {
    Some(ReactionType::CheckMark)
  } else if question {
    Some(ReactionType::Question)
  } else if defer {
    Some(ReactionType::Memo)
  } else {
    None
  };

  // For GitHub, handle reactions
  if session.platform == "github" && reaction_type.is_some() {
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

    // Add reaction
    if let Some(ref reaction) = reaction_type {
      let success = github.add_reaction(
        repo_parts[0],
        repo_parts[1], 
        current_thread_id,
        reaction.clone()
      ).await?;

      if success {
        bentley::success(&format!("Added {} reaction to thread", reaction.emoji()));
      } else {
        bentley::warn("Failed to add reaction");
      }
    }
  }

  // Format comment with linkback if needed
  let final_comment = if reaction_type.is_some() {
    format!("{}: {} - {}", 
      reaction_type.as_ref().unwrap().emoji(),
      session.merge_request.url, 
      text
    )
  } else {
    text
  };

  // TODO: Implement actual comment creation
  if new {
    bentley::info(&format!("Would add MR-level comment: {}", final_comment));
  } else {
    bentley::info(&format!("Would add thread reply: {}", final_comment));
  }

  Ok(())
}
