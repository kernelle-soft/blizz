//! Violet ignore directive processing
//! 
//! Handles parsing and processing of "violet ignore" directives in source code.

use regex::Regex;

const IGNORE_DIRECTIVE_PATTERN: &str = r"violet\signore\s(file|chunk|start|end|line)";
const IGNORE_CHUNK_PATTERN: &str = r"violet\signore\schunk";

/// Check if lines contain a "violet ignore file" directive
pub fn is_ignored_file(lines: &[&str]) -> bool {
  let ignore_regex = Regex::new(IGNORE_DIRECTIVE_PATTERN).unwrap();
  lines.iter().any(|line| {
    ignore_regex.captures(line).is_some_and(|caps| caps.get(1).unwrap().as_str() == "file")
  })
}

/// Check if chunk content contains a "violet ignore chunk" directive
pub fn is_ignored_chunk(chunk_content: &str) -> bool {
  let ignore_regex = Regex::new(IGNORE_CHUNK_PATTERN).unwrap();
  chunk_content.lines().any(|line| ignore_regex.is_match(line))
}

/// Check if chunk should be ignored based on directives and regex patterns
pub fn has_ignored_patterns(chunk_content: &str, ignore_patterns: &[String]) -> bool {
  if is_ignored_chunk(chunk_content) {
    return true;
  }
  
  for pattern in ignore_patterns {
    if let Ok(regex) = Regex::new(pattern) {
      if regex.is_match(chunk_content) {
        return true;
      }
    }
  }
  
  false
}

/// Process a specific ignore directive and update state
pub fn process_directive<'a>(
  directive: &str,
  ignore_depth: &mut usize,
  skip_next_line: &mut bool,
  result_lines: &mut Vec<&'a str>,
  line: &'a str,
) {
  match directive {
    "start" => {
      *ignore_depth += 1;
    }
    "end" => {
      if *ignore_depth > 0 {
        *ignore_depth -= 1;
      }
    }
    "line" => {
      *skip_next_line = true;
    }
    "chunk" => {
      // Keep directive lines for chunk removal during analysis
      if *ignore_depth == 0 {
        result_lines.push(line);
      }
    }
    _ => {}
  }
}

/// Process a single line for ignore directives
pub fn process_line<'a>(
  line: &'a str,
  ignore_depth: &mut usize,
  skip_next_line: &mut bool,
  result_lines: &mut Vec<&'a str>,
) -> bool {
  let ignore_regex = Regex::new(IGNORE_DIRECTIVE_PATTERN).unwrap();

  if *skip_next_line {
    *skip_next_line = false;
    return true;
  }

  if let Some(captures) = ignore_regex.captures(line) {
    let directive = captures.get(1).unwrap().as_str();
    process_directive(directive, ignore_depth, skip_next_line, result_lines, line);
    return true;
  }

  false
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_should_ignore_chunk() {
    let normal_chunk = "fn normal() {\n    return 42;\n}";
    assert!(!is_ignored_chunk(normal_chunk));

    let ignored_chunk = "// violet ignore chunk\nfn complex() {\n    if deeply {\n        if nested {\n            return 42;\n        }\n    }\n}";
    assert!(is_ignored_chunk(ignored_chunk));

    let ignored_chunk2 = "# violet ignore chunk\nfn another() { return 1; }";
    assert!(is_ignored_chunk(ignored_chunk2));

    let ignored_chunk3 = "/* violet ignore chunk */\nfn yet_another() { return 2; }";
    assert!(is_ignored_chunk(ignored_chunk3));

    let ignored_chunk4 = "// violet ignore chunk - this is a comment explaining why the chunk is ignored  \nfn spaced() { return 3; }";
    assert!(is_ignored_chunk(ignored_chunk4));
  }
} 