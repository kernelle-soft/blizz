use crate::session::SessionManager;
use anyhow::Result;

pub async fn handle() -> Result<()> {
  let mut session_manager = SessionManager::new()?;

  if let Some(session) = session_manager.load_session()? {
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
