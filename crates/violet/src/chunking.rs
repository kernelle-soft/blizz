//! Content chunking and chunk fusion logic
//! 
//! Handles splitting content into logical chunks separated by blank lines.

use crate::scoring;

/// Iterative fusion algorithm to discover natural code boundaries
pub fn find_chunks(content: &str) -> Vec<(usize, usize)> {
  if content.is_empty() {
    return vec![];
  }

  let lines: Vec<&str> = content.lines().collect();
  let line_lengths: Vec<f64> = lines.iter().map(|line| line.len() as f64).collect();

  // Start with blank-line boundaries
  let mut ranges = split_on_blanks(&lines);
  
  // Iteratively merge compatible chunks until stable
  loop {
    let initial_count = ranges.len();
    ranges = merge_chunks(ranges, &line_lengths, &lines);
    
    if ranges.len() == initial_count {
      break;
    }
  }
  
  ranges
}

/// Initial chunking based on blank lines
pub fn split_on_blanks(lines: &[&str]) -> Vec<(usize, usize)> {
  let mut chunks = Vec::new();
  let mut current_start = 0;
  let mut in_text_block = false;
  
  for (i, line) in lines.iter().enumerate() {
    let has_text = !line.trim().is_empty();
    
    if has_text && !in_text_block {
      current_start = i;
      in_text_block = true;
    } else if !has_text && in_text_block {
      chunks.push((current_start, i));
      in_text_block = false;
    }
  }
  
  if in_text_block {
    chunks.push((current_start, lines.len()));
  }
  
  chunks
}

/// Merge adjacent chunks that likely belong together
pub fn merge_chunks(chunks: Vec<(usize, usize)>, line_lengths: &[f64], lines: &[&str]) -> Vec<(usize, usize)> {
  if chunks.len() <= 1 {
    return chunks;
  }
  
  let mut fused_chunks = Vec::new();
  let mut current_chunk = chunks[0];
  
  for i in 1..chunks.len() {
    let next_chunk = chunks[i];
    
    if should_merge(current_chunk, next_chunk, line_lengths, lines) {
      current_chunk = (current_chunk.0, next_chunk.1);
    } else {
      fused_chunks.push(current_chunk);
      current_chunk = next_chunk;
    }
  }
  
  fused_chunks.push(current_chunk);
  
  fused_chunks
}

/// Decide whether to fuse chunks based on indentation patterns and line length transitions
pub fn should_merge(chunk1: (usize, usize), chunk2: (usize, usize), line_lengths: &[f64], lines: &[&str]) -> bool {
  if !are_valid(chunk1, chunk2, lines) {
    return false;
  }
  
  if has_block_entry_exit(chunk1, chunk2, lines) {
    return true;
  }
  
  has_gradual_length_transition(chunk1, chunk2, line_lengths)
}

fn are_valid(chunk1: (usize, usize), chunk2: (usize, usize), lines: &[&str]) -> bool {
  chunk1.0 < lines.len() && chunk1.1 > chunk1.0 && 
  chunk2.0 < lines.len() && chunk2.1 > chunk2.0
}

fn has_block_entry_exit(chunk1: (usize, usize), chunk2: (usize, usize), lines: &[&str]) -> bool {
  let prev_start_indent = scoring::get_indents(lines[chunk1.0]) as f64;
  let prev_end_indent = scoring::get_indents(lines[chunk1.1.saturating_sub(1)]) as f64;
  
  if prev_end_indent <= prev_start_indent {
    return false;
  }
  
  let next_start_indent = scoring::get_indents(lines[chunk2.0]) as f64;
  let next_end_indent = scoring::get_indents(lines[chunk2.1.saturating_sub(1)]) as f64;
  
  next_start_indent >= next_end_indent
}

fn has_gradual_length_transition(chunk1: (usize, usize), chunk2: (usize, usize), line_lengths: &[f64]) -> bool {
  let last_line_idx = chunk1.1.saturating_sub(1);
  let first_line_idx = chunk2.0;
  
  let last_line_length = line_lengths[last_line_idx];
  let first_line_length = line_lengths[first_line_idx];
  
  let length_ratio = calculate_length_ratio(last_line_length, first_line_length);
  
  length_ratio <= 2.0
}

fn calculate_length_ratio(last_length: f64, first_length: f64) -> f64 {
  if last_length == 0.0 {
    f64::INFINITY
  } else {
    first_length / last_length
  }
}

/// Split content into chunks separated by blank lines
fn split_on_blank_lines(content: &str) -> Vec<String> {
  let mut temp_chunks = Vec::new();
  let mut current_chunk = Vec::new();

  for line in content.lines() {
    if line.trim().is_empty() {
      if !current_chunk.is_empty() {
        temp_chunks.push(current_chunk.join("\n"));
        current_chunk.clear();
      }
    } else {
      current_chunk.push(line);
    }
  }

  if !current_chunk.is_empty() {
    temp_chunks.push(current_chunk.join("\n"));
  }

  temp_chunks
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_get_chunks() {
    let content = "function one() {\n  return 1;\n}\n\nfunction two() {\n  return 2;\n}";
    let chunks = find_chunks(content);

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0], (0, 2));
    assert_eq!(chunks[1], (2, 4));
  }

  #[test]
  fn test_get_chunks_no_trailing_newline() {
    let content = "fn main() {\n    println!(\"hello\");\n}";
    let chunks = find_chunks(content);

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], (0, 2));
  }

  #[test]
  fn test_get_chunks_with_indentation() {
    let content = "function main() {\n    return 42;\n}\n\nclass Test {\n    method() {}\n}";
    let chunks = find_chunks(content);
    
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0], (0, 2));
    assert_eq!(chunks[1], (2, 4));
  }
} 