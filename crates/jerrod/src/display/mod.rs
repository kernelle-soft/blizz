use crate::platform::{Discussion, Note, FileDiff};
use chrono::{DateTime, Local, Utc};

/// Convert UTC timestamp to human-readable local timezone format
/// Mimics the 2utc4me function from the old shell scripts
pub fn format_timestamp(utc_time: DateTime<Utc>) -> String {
  let local_time: DateTime<Local> = utc_time.into();
  local_time.format("%A, %B %d, %Y at %I:%M %p").to_string()
}

/// Display thread information in tabular format as a banner
/// Mimics the display_thread_info function from peek-thread.sh
pub fn display_thread_header(note: &Note, thread_id: &str) {
  let formatted_time = format_timestamp(note.created_at);
  let header = format!("🧵 {} | {} | ID: {}", note.author.display_name, formatted_time, thread_id);
  
  // Use bentley's banner functionality instead of manual banner creation
  bentley::as_banner(
    |msg| eprintln!("{}", msg),
    &header,
    Some(80),
    Some('-')
  );
  
  // Display content with proper word wrapping
  let width = 80;
  for content_line in note.body.lines() {
    if content_line.len() <= width {
      println!("{}", content_line);
    } else {
      // Word wrap long lines
      let words: Vec<&str> = content_line.split_whitespace().collect();
      let mut current_line = String::new();
      
      for word in words {
        if current_line.len() + word.len() + 1 <= width {
          if !current_line.is_empty() {
            current_line.push(' ');
          }
          current_line.push_str(word);
        } else {
          if !current_line.is_empty() {
            println!("{}", current_line);
            current_line = word.to_string();
          } else {
            println!("{}", word);
          }
        }
      }
      
      if !current_line.is_empty() {
        println!("{}", current_line);
      }
    }
  }
  
  // Close the banner with bentley
  println!("{}", bentley::banner_line(80, '-'));
}

pub fn display_file_context(file_path: &str, line_number: Option<u32>) {
  println!();
  bentley::info(&format!("File: {}", file_path));
  
  if let Some(line) = line_number {
    bentley::info(&format!("Line: {} (new)", line));
  }
}

/// Display discussion replies in formatted style
/// Mimics the display_replies function from peek-thread.sh
pub fn display_replies(discussion: &Discussion) {
  if discussion.notes.len() > 1 {
    println!();
    bentley::info("Replies:");
    println!();
    
    // Display all replies after the first note
    for reply in discussion.notes.iter().skip(1) {
      let formatted_time = format_timestamp(reply.created_at);
      bentley::info(&format!("  {} ({}):", reply.author.display_name, formatted_time));
      
      // Display reply content with indentation
      for line in reply.body.lines() {
        println!("    {}", line);
      }
      println!();
    }
  }
}

pub fn display_discussion_thread(discussion: &Discussion) {
  if let Some(first_note) = discussion.notes.first() {
    // Display the main thread header
    display_thread_header(first_note, &discussion.id);
    
    // Display file context if available
    if let Some(file_path) = &discussion.file_path {
      display_file_context(file_path, discussion.line_number);
    }
    
    // Display any replies
    display_replies(discussion);
  }
}

pub fn display_file_diff(diff: &FileDiff) {
  // Use bentley's banner functionality for diff display
  let header = format!("📄 File: {}", diff.new_path);
  let full_header = if let Some(old_path) = &diff.old_path {
    if old_path != &diff.new_path {
      format!("{}\n   (renamed from {})", header, old_path)
    } else {
      header
    }
  } else {
    header
  };
  
  bentley::as_banner(
    |msg| println!("{}", msg),
    &full_header,
    Some(80),
    Some('═')
  );
  
  // Display diff content with color coding
  for line in diff.diff.lines() {
    if line.starts_with("@@") {
      // Hunk headers
      println!("🔵 {}", line);
    } else if line.starts_with('+') {
      // Added lines
      println!("🟢 {}", line);
    } else if line.starts_with('-') {
      // Removed lines
      println!("🔴 {}", line);
    } else {
      // Context lines
      println!("   {}", line);
    }
  }
  
  println!("{}", bentley::banner_line(80, '═'));
} 