use anyhow::{anyhow, Result};
use crate::session::SessionManager;
use crate::platform::{GitPlatform, github::GitHubPlatform};

pub async fn handle(
  text: String,
  new: bool,
) -> Result<()> {

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

  // Create the comment
  if new {
    // Create a new MR-level comment
    let pr_number = session.merge_request.number.to_string();
    let _note = github.add_comment(
      repo_parts[0],
      repo_parts[1],
      &pr_number,
      &text
    ).await?;
    bentley::success("Added new comment to MR");
  } else {
    // For GitHub, we can't reply to specific comments directly
    // Instead, we'll create a new comment with a reference and quote
    if let Some(thread_id) = &current_thread_id {
      // Find the original thread to get context for the quote
      let original_thread = session.discussions.get(thread_id);
      
              let referenced_text = if let Some(thread) = original_thread {
        // Create a nice reference with quote and link
        let empty_string = String::new();
        let original_content = thread.notes.first()
          .map(|note| &note.body)
          .unwrap_or(&empty_string);
        
        // Truncate to 256 characters with ellipsis
        let truncated_quote = if original_content.len() > 256 {
          format!("{}...", &original_content[..253])
        } else {
          original_content.clone()
        };
        
        // Create GitHub comment link
        let comment_url = format!("{}#issuecomment-{}", session.merge_request.url, thread_id);
        
        // Format with proper markdown quote
        format!("Re: [comment]({})\n\n> {}\n\n{}", comment_url, truncated_quote.replace('\n', "\n> "), text)
      } else {
        // Fallback to old format if we can't find the thread
        format!("Re: comment {}\n\n{}", thread_id, text)
      };
      
      let pr_number = session.merge_request.number.to_string();
      let _note = github.add_comment(
        repo_parts[0],
        repo_parts[1],
        &pr_number,
        &referenced_text
      ).await?;
      bentley::success("Added comment with reference to thread");
    }
  }



  bentley::info(&format!("Comment: {}", text));
  Ok(())
}
