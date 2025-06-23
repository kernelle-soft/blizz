use crate::session::SessionManager;
use crate::commands::{finish, start};
use anyhow::{anyhow, Result};

pub async fn handle() -> Result<()> {
  bentley::info("Refreshing review session...");
  
  let session_manager = SessionManager::new()?;
  
  
  let session = session_manager
    .load_session()?
    .ok_or_else(|| anyhow!("No active review session found. Use 'jerrod start' to begin a new session."))?;
  
  
  let repository = session.repository.full_name.clone();
  let mr_number = session.merge_request.number;
  let platform = session.platform.clone();
  
  bentley::info(&format!("Refreshing session for {} MR #{}", repository, mr_number));
  
  
  bentley::info("Cleaning up existing session...");
  finish::handle().await?;
  
  
  bentley::info("Re-downloading fresh session data...");
  start::handle(repository, mr_number, Some(platform), None, None).await?;
  
  bentley::success("Session refreshed successfully!");
  
  Ok(())
}
