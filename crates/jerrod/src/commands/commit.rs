use anyhow::{anyhow, Result};
use std::process::Command;
use crate::session::SessionManager;

pub async fn handle(message: String, details: Option<String>, thread_id: Option<String>) -> Result<()> {
  // Get current session info if available
  let session_manager = SessionManager::new()?;
  let session = session_manager.load_session()?;

  // Build commit message
  let mut commit_msg = message;
  
  if let Some(details) = details {
    commit_msg.push_str("\n\n");
    commit_msg.push_str(&details);
  }

  // Add session and thread references if available
  if let Some(session) = session {
    let mr_url = &session.merge_request.url;
    commit_msg.push_str(&format!("\n\nMerge Request: {}", mr_url));
    
    if let Some(thread_id) = thread_id {
      // For GitHub, link to the specific comment
      if session.platform == "github" {
        commit_msg.push_str(&format!("\nAddressing Thread: {}#issuecomment-{}", mr_url, thread_id));
      } else {
        // For GitLab (when implemented)
        commit_msg.push_str(&format!("\nAddressing Thread: {}/diffs#note_{}", mr_url, thread_id));
      }
    }
  }

  // Stage all changes
  let add_status = Command::new("git")
    .args(["add", "."])
    .status()?;

  if !add_status.success() {
    return Err(anyhow!("Failed to stage changes"));
  }

  // Create commit
  let commit_status = Command::new("git")
    .args(["commit", "-m", &commit_msg])
    .status()?;

  if commit_status.success() {
    bentley::success("Commit created successfully!");
    bentley::info("Commit message:");
    println!("---");
    println!("{}", commit_msg);
    println!("---");
  } else {
    return Err(anyhow!("Failed to create commit"));
  }

  Ok(())
} 