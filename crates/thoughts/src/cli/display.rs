//! Display formatting utilities for CLI output

use colored::*;

/// Highlight search terms in text
pub fn highlight_keywords(text: &str, terms: &[String]) -> String {
  let mut result = text.to_string();

  let mut sorted = terms.to_vec();
  sorted.sort_by_key(|b| std::cmp::Reverse(b.len()));

  for term in sorted {
    if term.is_empty() {
      continue;
    }

    let term_lower = term.to_lowercase();
    let mut highlighted = String::new();
    let mut end = 0;

    let result_lower = result.to_lowercase();
    let mut start = 0;

    while let Some(pos) = result_lower[start..].find(&term_lower) {
      let abs_pos = start + pos;

      highlighted.push_str(&result[end..abs_pos]);

      let match_text = &result[abs_pos..abs_pos + term.len()];
      highlighted.push_str(&match_text.yellow().bold().to_string());

      end = abs_pos + term.len();
      start = end;
    }

    highlighted.push_str(&result[end..]);
    result = highlighted;
  }

  result
}

/// Wrap text to fit within a specified width
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
  let mut lines = Vec::new();

  for paragraph in text.split('\n') {
    if paragraph.trim().is_empty() {
      lines.push(String::new());
      continue;
    }

    let words: Vec<&str> = paragraph.split_whitespace().collect();
    let mut current_line = String::new();

    for word in words {
      if current_line.is_empty() {
        current_line = word.to_string();
      } else if current_line.len() + 1 + word.len() <= width {
        current_line.push(' ');
        current_line.push_str(word);
      } else {
        lines.push(current_line);
        current_line = word.to_string();
      }
    }

    if !current_line.is_empty() {
      lines.push(current_line);
    }
  }

  lines
}

/// Display a single search result with keyword highlighting
pub fn display_search_result(
  topic: &str,
  name: &str,
  overview: &str,
  details: &str,
  terms: &[String],
  overview_only: bool,
) {
  let header = format!("=== {}/{} ===", topic.blue().bold(), name.yellow().bold());

  println!("{header}");

  // Wrap and display the content with proper formatting
  let wrap_with = if header.len() < 80 { 80 } else { header.len() };

  let content =
    if overview_only { overview.to_string() } else { format!("{overview}\n\n{details}") };

  let highlighted_content = highlight_keywords(&content, terms);
  let wrapped_lines = wrap_text(&highlighted_content, wrap_with);
  for line in wrapped_lines {
    println!("{line}");
  }
  println!();
}
