use crate::session::load_current_session;
use crate::platform::{GitPlatform, ReactionType};
use crate::platform::github::GitHubPlatform;
use anyhow::{anyhow, Result};

pub async fn handle() -> Result<()> {
  let session = load_current_session()?;

  // Get the current thread
  let current_thread = session.peek_next_thread()
    .ok_or_else(|| anyhow!("No current thread. Use 'jerrod peek' to see the next thread."))?;

  // Parse repository info from session
  let repo_parts: Vec<&str> = session.repository.full_name.split('/').collect();
  if repo_parts.len() != 2 {
    return Err(anyhow!("Invalid repository format: {}", session.repository.full_name));
  }
  let owner = repo_parts[0];
  let repo = repo_parts[1];

  // Create platform client
  let platform = GitHubPlatform::new().await?;

  // Try to resolve the discussion
  match platform.resolve_discussion_with_pr(owner, repo, session.merge_request.number, &current_thread.id).await {
    Ok(true) => {
      bentley::success(&format!("Resolved thread #{}", current_thread.id));
    },
    Ok(false) => {
      // Native resolution not available, use reaction-based resolution
      bentley::info("Using reaction-based resolution...");
      match platform.add_reaction(owner, repo, &current_thread.id, ReactionType::ThumbsUp).await {
        Ok(true) => {
          bentley::success(&format!("Resolved thread #{} with 👍 reaction", current_thread.id));
        },
        Ok(false) => {
          bentley::warn(&format!("Could not resolve thread #{} - reaction failed", current_thread.id));
        },
        Err(reaction_err) => {
          bentley::error(&format!("Failed to resolve thread #{}: {}", current_thread.id, reaction_err));
          return Err(anyhow!("Resolution failed for thread #{}: {}", current_thread.id, reaction_err));
        }
      }
    },
    Err(e) => {
      bentley::warn(&format!("Native resolution failed for thread #{}: {}", current_thread.id, e));
      
      // Fallback: add a reaction to indicate resolution
      bentley::info("Falling back to reaction-based resolution...");
      match platform.add_reaction(owner, repo, &current_thread.id, ReactionType::ThumbsUp).await {
        Ok(true) => {
          bentley::success(&format!("Resolved thread #{} with 👍 reaction", current_thread.id));
        },
        Ok(false) => {
          bentley::warn(&format!("Could not resolve thread #{} - both native and reaction resolution failed", current_thread.id));
        },
        Err(reaction_err) => {
          bentley::error(&format!("Both resolution methods failed for thread #{}: native={}, reaction={}", current_thread.id, e, reaction_err));
          return Err(anyhow!("Both resolution and reaction failed for thread #{}", current_thread.id));
        }
      }
    }
  }

  Ok(())
} 