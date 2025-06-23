use crate::session::{get_session_manager, load_current_session};
use anyhow::Result;

pub async fn handle() -> Result<()> {
  if let Ok(session) = load_current_session() {
    let session_manager = get_session_manager()?;
    bentley::info(&format!("Finishing review session for MR #{}", session.merge_request.number));

    if session.threads_remaining() > 0 {
      bentley::warn(&format!("{} threads still in queue", session.threads_remaining()));
    }

    if session.has_unresolved_threads() {
      bentley::warn(&format!("{} unresolved threads", session.unresolved_threads.len()));
    }

    session_manager.clear_session()?;
    bentley::success("Review session finished and cleaned up");
  } else {
    bentley::info("No active review session to finish");
  }

  Ok(())
}
