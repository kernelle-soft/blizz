use crate::display;
use crate::session::SessionManager;
use crate::platform::{GitPlatform, github::GitHubPlatform};
use anyhow::{anyhow, Result};

pub async fn handle() -> Result<()> {
  let mut session_manager = SessionManager::new()?;

  let session = session_manager
    .load_session()?
    .ok_or_else(|| anyhow!("No active review session. Use 'jerrod start' to begin."))?;

  if let Some(thread) = session.peek_next_thread() {

    display::display_discussion_thread(thread);
    

    if let (Some(file_path), Some(_line_number)) = (&thread.file_path, thread.line_number) {
      bentley::info(&format!("Fetching diff context for {}", file_path));
      
      
      if session.platform == "github" {
        if let Ok(platform) = GitHubPlatform::new().await {
          let repo_parts: Vec<&str> = session.repository.name.split('/').collect();
          if repo_parts.len() == 2 {
            if let Ok(diffs) = platform.get_diffs(repo_parts[0], repo_parts[1], session.merge_request.number).await {
      
              if let Some(diff) = diffs.iter().find(|d| d.new_path == *file_path) {
                display::display_file_diff(diff);
              }
            }
          }
        }
      }
    }
  } else {
    bentley::info("No threads remaining in queue!");
  }

  Ok(())
}
