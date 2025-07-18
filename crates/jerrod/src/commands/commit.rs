use crate::platform::create_platform_with_host;
use crate::session::load_current_session;
use anyhow::{anyhow, Result};
use std::process::Command;

pub async fn handle(
  message: String,
  details: Option<String>,
  thread_id: Option<String>,
) -> Result<()> {
  let session = load_current_session().ok();

  let mut commit_msg = message;

  if let Some(details) = details {
    commit_msg.push_str("\n\n");
    commit_msg.push_str(&details);
  }

  if let Some(ref session) = session {
    let current_thread_id =
      thread_id.or_else(|| session.peek_next_thread().map(|thread| thread.id.clone()));
    let mr_url = &session.merge_request.url;
    commit_msg.push_str(&format!("\n\nMerge Request: {mr_url}"));

    if let Some(thread_id) = current_thread_id {
      // Use strategy pattern for platform-specific URL formatting
      if let Ok(platform) =
        create_platform_with_host(&session.platform, session.host.as_deref()).await
      {
        let comment_url = platform.format_comment_url(mr_url, &thread_id);
        commit_msg.push_str(&format!("\nAddressing Thread:\n{comment_url}"));
      } else {
        // Fallback to generic thread reference if platform creation fails
        commit_msg.push_str(&format!("\nAddressing Thread: {mr_url} (thread {thread_id})"));
      }
    }
  }

  let add_status = Command::new("git").args(["add", "."]).status()?;

  if !add_status.success() {
    return Err(anyhow!("Failed to stage changes"));
  }

  let commit_status =
    Command::new("git").args(["commit", "--quiet", "-m", &commit_msg]).status()?;

  if commit_status.success() {
    let stats_output =
      Command::new("git").args(["show", "--stat", "--format=", "HEAD"]).output()?;

    if stats_output.status.success() {
      let stats_text = String::from_utf8_lossy(&stats_output.stdout);
      display_commit_banner(&commit_msg, &stats_text);
    } else {
      println!("---");
      println!("{commit_msg}");
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
  let line = bentley::banner_line(width, '-');

  println!("{line}");

  // Parse stats for summary line
  let stats_summary = parse_git_stats(stats_text);
  if !stats_summary.is_empty() {
    println!("📝 {stats_summary}");
  }

  println!("{line}");

  // Display commit message with word wrapping
  let mut current_line = String::new();

  for word in commit_msg.split_whitespace() {
    if current_line.len() + word.len() < width {
      if !current_line.is_empty() {
        current_line.push(' ');
      }
      current_line.push_str(word);
    } else if !current_line.is_empty() {
      println!("{current_line}");
      current_line = word.to_string();
    } else {
      println!("{word}");
    }
  }

  if !current_line.is_empty() {
    println!("{current_line}");
  }

  println!("{line}");
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
  let file_count = lines.iter().filter(|line| line.contains(" | ")).count();

  if file_count > 0 {
    return format!("{} file{} changed", file_count, if file_count == 1 { "" } else { "s" });
  }

  String::new()
}
