use crate::session::SessionManager;
use crate::commands::{finish, start};
use anyhow::{anyhow, Result};

pub async fn handle() -> Result<()> {
  bentley::info("Refreshing review session...");
  
  let session_manager = SessionManager::new()?;
  
  // Load existing session to get repository and MR details
  let session = session_manager
    .load_session()?
    .ok_or_else(|| anyhow!("No active review session found. Use 'jerrod start' to begin a new session."))?;
  
  // Extract session details before cleanup
  let repository = session.repository.full_name.clone();
  let mr_number = session.merge_request.number;
  let platform = session.platform.clone();
  
  bentley::info(&format!("Refreshing session for {} MR #{}", repository, mr_number));
  
  // Clean up existing session
  bentley::info("Cleaning up existing session...");
  finish::handle().await?;
  
  // Start fresh session with same parameters
  bentley::info("Re-downloading fresh session data...");
  start::handle(repository, mr_number, Some(platform), None, None).await?;
  
  bentley::success("Session refreshed successfully!");
  
  Ok(())
}
