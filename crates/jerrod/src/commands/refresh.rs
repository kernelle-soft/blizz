use crate::commands::{finish, start};
use crate::session::load_current_session;
use anyhow::Result;

pub async fn handle() -> Result<()> {
  bentley::info("Refreshing review session...");
  let session = load_current_session()?;

  let repository = session.repository.full_name.clone();
  let mr_number = session.merge_request.number;
  let platform = session.platform.clone();

  bentley::info(&format!("Refreshing session for {repository} MR #{mr_number}"));

  bentley::info("Cleaning up existing session...");
  finish::handle().await?;

  bentley::info("Re-downloading fresh session data...");
  start::handle(repository, mr_number, Some(platform)).await?;

  bentley::success("Session refreshed successfully!");

  Ok(())
}
