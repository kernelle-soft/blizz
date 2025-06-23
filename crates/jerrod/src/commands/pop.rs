use crate::session::{get_session_manager, load_current_session};
use anyhow::Result;

pub async fn handle(unresolved: bool) -> Result<()> {
  let session_manager = get_session_manager()?;
  let mut session = load_current_session()?;

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
