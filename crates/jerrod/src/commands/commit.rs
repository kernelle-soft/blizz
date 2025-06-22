use anyhow::{anyhow, Result};
use std::process::Command;
use crate::session::SessionManager;
use crate::display;

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

  // Create commit quietly
  let commit_status = Command::new("git")
    .args(["commit", "--quiet", "-m", &commit_msg])
    .status()?;

  if commit_status.success() {
    // Get commit stats
    let stats_output = Command::new("git")
      .args(["show", "--stat", "--format=", "HEAD"])
      .output()?;
    
    if stats_output.status.success() {
      let stats_text = String::from_utf8_lossy(&stats_output.stdout);
      display_commit_banner(&commit_msg, &stats_text);
    } else {
      // Fallback to simple display
      println!("---");
      println!("{}", commit_msg);
      println!("---");
    }
  } else {
    return Err(anyhow!("Failed to create commit"));
  }

  Ok(())
}

/// Display commit information in banner format
fn display_commit_banner(commit_msg: &str, stats_text: &str) {
  let width = 80;
  let line = display::banner_line(width, '=');
  
  println!("{}", line);
  
  // Parse stats for summary line
  let stats_summary = parse_git_stats(stats_text);
  if !stats_summary.is_empty() {
    println!("📝 {}", stats_summary);
  }
  
  println!("{}", line);
  println!("{}", commit_msg);
  println!("{}", line);
}

/// Parse git stats output into a human-readable summary
fn parse_git_stats(stats_text: &str) -> String {
  let lines: Vec<&str> = stats_text.trim().lines().collect();
  
  // Look for the summary line (usually the last non-empty line)
  for line in lines.iter().rev() {
    let trimmed = line.trim();
    if trimmed.is_empty() {
      continue;
    }
    
    // Check if this looks like a stats summary line
    if trimmed.contains("file") && (trimmed.contains("insertion") || trimmed.contains("deletion")) {
      return trimmed.to_string();
    }
  }
  
  // Fallback: try to count files from the output
  let file_count = lines.iter()
    .filter(|line| line.contains(" | "))
    .count();
  
  if file_count > 0 {
    return format!("{} file{} changed", file_count, if file_count == 1 { "" } else { "s" });
  }
  
  String::new()
} 