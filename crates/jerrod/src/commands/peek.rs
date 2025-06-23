use crate::display;
use crate::session::load_current_session;
use crate::platform::create_platform;
use anyhow::Result;

pub async fn handle() -> Result<()> {
  let session = load_current_session()?;

  let Some(thread) = session.peek_next_thread() else {
    bentley::info("No threads remaining in queue!");
    return Ok(());
  };

  display::display_discussion_thread(thread);
  
  let (Some(file_path), Some(_line_number)) = (&thread.file_path, thread.line_number) else {
    return Ok(());
  };

  bentley::info(&format!("Fetching diff context for {}", file_path));
  
  if let Err(e) = show_file_diff(&session, file_path).await {
    bentley::warn(&format!("Could not fetch diff context: {}", e));
  }

  Ok(())
}

async fn show_file_diff(session: &crate::session::ReviewSession, file_path: &str) -> Result<()> {
  let platform = create_platform(&session.platform).await?;
  
  let repo_parts: Vec<&str> = session.repository.full_name.split('/').collect();
  let [owner, repo] = repo_parts.as_slice() else {
    anyhow::bail!("Invalid repository format: {}", session.repository.full_name);
  };

  let diffs = platform.get_diffs(owner, repo, session.merge_request.number).await?;
  
  let Some(diff) = diffs.iter().find(|d| d.new_path == file_path) else {
    anyhow::bail!("No diff found for file: {}", file_path);
  };

  display::display_file_diff(diff);
  Ok(())
}
