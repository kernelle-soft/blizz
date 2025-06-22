use crate::display;
use crate::session::SessionManager;
use anyhow::{anyhow, Result};

pub async fn handle() -> Result<()> {
  let session_manager = SessionManager::new()?;

  let session = session_manager
    .load_session()?
    .ok_or_else(|| anyhow!("No active review session. Use 'jerrod start' to begin."))?;

  if let Some(thread) = session.peek_next_thread() {
    // Lean output - just show the thread
    display::display_discussion_thread(thread);
  } else {
    bentley::info("No threads remaining in queue!");
  }

  Ok(())
}
