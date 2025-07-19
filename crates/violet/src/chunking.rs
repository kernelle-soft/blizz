//! Content chunking and chunk fusion logic
//! 
//! Handles splitting content into logical chunks separated by blank lines.

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

/// Split content into natural chunks separated by blank lines
pub fn get_chunks(content: &str) -> Vec<String> {
  split_on_blank_lines(content)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_get_chunks() {
    let content = "function one() {\n  return 1;\n}\n\nfunction two() {\n  return 2;\n}";
    let chunks = get_chunks(content);

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0], "function one() {\n  return 1;\n}");
    assert_eq!(chunks[1], "function two() {\n  return 2;\n}");
  }

  #[test]
  fn test_get_chunks_no_trailing_newline() {
    let content = "fn main() {\n    println!(\"hello\");\n}";
    let chunks = get_chunks(content);

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], "fn main() {\n    println!(\"hello\");\n}");
  }

  #[test]
  fn test_split_on_blank_lines() {
    let content = "line 1\nline 2\n\nline 3\nline 4\n\n\nline 5";
    let chunks = split_on_blank_lines(content);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0], "line 1\nline 2");
    assert_eq!(chunks[1], "line 3\nline 4");
    assert_eq!(chunks[2], "line 5");

    let empty_chunks = split_on_blank_lines("");
    assert!(empty_chunks.is_empty());

    let blank_only = split_on_blank_lines("\n\n\n");
    assert!(blank_only.is_empty());

    let no_blanks = split_on_blank_lines("line1\nline2\nline3");
    assert_eq!(no_blanks.len(), 1);
    assert_eq!(no_blanks[0], "line1\nline2\nline3");
  }

  #[test]
  fn test_get_chunks_with_indentation() {
    let content = "function main() {\n    return 42;\n}\n\nclass Test {\n    method() {}\n}";
    let chunks = get_chunks(content);
    
    assert_eq!(chunks.len(), 2);
    assert!(chunks[0].contains("function main()"));
    assert!(chunks[0].contains("return 42;"));
    assert!(chunks[0].contains("}"));
    
    assert!(chunks[1].contains("class Test"));
    assert!(chunks[1].contains("method()"));
  }
} 