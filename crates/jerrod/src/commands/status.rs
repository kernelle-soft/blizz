use crate::session::SessionManager;
use anyhow::{anyhow, Result};

pub async fn handle() -> Result<()> {
  let session_manager = SessionManager::new()?;

  let session = session_manager
    .load_session()?
    .ok_or_else(|| anyhow!("No active review session. Use 'jerrod start' to begin."))?;

  bentley::announce("Review Session Status");
  println!("📋 {}", session.merge_request.title);

  println!("Repository: {}", session.repository.full_name);
  println!("Platform: {}", session.platform);
  println!("MR/PR: #{} - {}", session.merge_request.number, session.merge_request.title);
  println!("State: {:?}", session.merge_request.state);
  println!("Author: {}", session.merge_request.author.display_name);

  if let Some(assignee) = &session.merge_request.assignee {
    println!("Assignee: {}", assignee.display_name);
  }

  println!(
    "Branches: {} → {}",
    session.merge_request.source_branch, session.merge_request.target_branch
  );
  println!("URL: {}", session.merge_request.url);

  bentley::info(&format!("Threads remaining in queue: {}", session.threads_remaining()));

  if session.has_unresolved_threads() {
    bentley::warn(&format!("Unresolved threads: {}", session.unresolved_threads.len()));
  }

  if !session.pipelines.is_empty() {
    bentley::info(&format!("Pipelines: {}", session.pipelines.len()));
    for pipeline in &session.pipelines {
      println!("  - {} ({:?})", pipeline.id, pipeline.status);
    }
  }

  bentley::info(&format!(
    "Session created: {}",
    session.created_at.format("%Y-%m-%d %H:%M:%S UTC")
  ));
  bentley::info(&format!("Last updated: {}", session.updated_at.format("%Y-%m-%d %H:%M:%S UTC")));

  Ok(())
}
