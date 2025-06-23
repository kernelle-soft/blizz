use crate::session::SessionManager;
use anyhow::{anyhow, Result};

pub async fn handle(unresolved: bool) -> Result<()> {
  let mut session_manager = SessionManager::new()?;

  let mut session = session_manager
    .load_session()?
    .ok_or_else(|| anyhow!("No active review session. Use 'jerrod start' to begin."))?;

  if let Some(thread) = session.pop_thread(unresolved) {
    if unresolved {
      bentley::warn(&format!("Marked thread #{} as unresolved and removed from queue", thread.id));
    } else {
      bentley::success(&format!("Removed thread #{} from queue", thread.id));
    }

    session_manager.save_session(&session)?;

    bentley::info(&format!("Threads remaining: {}", session.threads_remaining()));
  } else {
    bentley::info("No threads in queue to pop!");
  }

  Ok(())
}
