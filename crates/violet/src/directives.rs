//! Violet directive processing
//!
//! Handles parsing and processing of violet directives in source code.

use regex::Regex;

/// Strip out violet directives, returning None if entire file should be ignored
pub fn preprocess_file(content: &str) -> Option<String> {
  let lines: Vec<&str> = content.lines().collect();

  if is_ignored_file(&lines) {
    return None;
  }

  let mut result_lines = Vec::new();
  let mut ignore_depth = 0;
  let mut skip_next_line = false;

  for line in lines.iter() {
    if process_line(line, &mut ignore_depth, &mut skip_next_line, &mut result_lines) {
      continue;
    }

    if ignore_depth == 0 {
      result_lines.push(*line);
    }
  }

  Some(result_lines.join("\n"))
}

const IGNORE_DIRECTIVE_PATTERN: &str = r"violet\signore\s(file|chunk|start|end|line)";
const IGNORE_CHUNK_PATTERN: &str = r"violet\signore\schunk";

/// Check if lines contain a directive to ignore the entire file
pub fn is_ignored_file(lines: &[&str]) -> bool {
  let ignore_regex = Regex::new(IGNORE_DIRECTIVE_PATTERN).unwrap();
  lines.iter().any(|line| {
    ignore_regex.captures(line).is_some_and(|caps| caps.get(1).unwrap().as_str() == "file")
  })
}

/// Check if chunk content contains a directive to ignore the chunk
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

    let directive_start = "violet";
    let directive_ignore = "ignore";
    let directive_chunk = "chunk";
    let full_directive = format!("{directive_start} {directive_ignore} {directive_chunk}");

    let ignored_chunk = format!("// {full_directive}\nfn complex() {{\n    if deeply {{\n        if nested {{\n            return 42;\n        }}\n    }}\n}}");
    assert!(is_ignored_chunk(&ignored_chunk));

    let ignored_chunk2 = format!("# {full_directive}\nfn another() {{ return 1; }}");
    assert!(is_ignored_chunk(&ignored_chunk2));

    let ignored_chunk3 = format!("/* {full_directive} */\nfn yet_another() {{ return 2; }}");
    assert!(is_ignored_chunk(&ignored_chunk3));

    let ignored_chunk4 = format!("// {full_directive} - this is a comment explaining why the chunk is ignored  \nfn spaced() {{ return 3; }}");
    assert!(is_ignored_chunk(&ignored_chunk4));
  }

  #[test]
  fn test_preprocess_file_no_ignores() {
    let content = "fn main() {\n    println!(\"hello\");\n}";
    let result = preprocess_file(content);

    assert_eq!(result, Some(content.to_string()));
  }

  #[test]
  fn test_preprocess_file_ignore_entire_file() {
    let file_directive = "file";
    let content1 =
      format!("# violet ignore {file_directive}\nfn main() {{\n    println!(\"hello\");\n}}");
    let content2 =
      format!("// violet ignore {file_directive}\nfn main() {{\n    println!(\"hello\");\n}}");
    let content3 =
      format!("/* violet ignore {file_directive} */\nfn main() {{\n    println!(\"hello\");\n}}");

    assert_eq!(preprocess_file(&content1), None);
    assert_eq!(preprocess_file(&content2), None);
    assert_eq!(preprocess_file(&content3), None);
  }

  #[test]
  fn test_preprocess_file_ignore_block() {
    let start_directive = "start";
    let end_directive = "end";
    let content = format!("fn good() {{\n    return 1;\n}}\n\n# violet ignore {start_directive}\nfn bad() {{\n    if nested {{\n        return 2;\n    }}\n}}\n# violet ignore {end_directive}\n\nfn also_good() {{\n    return 3;\n}}");
    let result = preprocess_file(&content).unwrap();

    assert!(result.contains("fn good()"));
    assert!(result.contains("fn also_good()"));
    assert!(!result.contains("fn bad()"));
    assert!(!result.contains("if nested"));
  }

  #[test]
  fn test_preprocess_file_nested_ignore_blocks() {
    let start_directive = "start";
    let end_directive = "end";
    let content = format!("fn good() {{\n    return 1;\n}}\n\n/* violet ignore {start_directive} */\nfn outer_bad() {{\n    # violet ignore {start_directive}\n    fn inner_bad() {{\n        return 2;\n    }}\n    # violet ignore {end_directive}\n    return 3;\n}}\n/* violet ignore {end_directive} */\n\nfn also_good() {{\n    return 4;\n}}");
    let result = preprocess_file(&content).unwrap();

    assert!(result.contains("fn good()"));
    assert!(result.contains("fn also_good()"));
    assert!(!result.contains("fn outer_bad()"));
    assert!(!result.contains("fn inner_bad()"));
  }

  #[test]
  fn test_preprocess_file_unmatched_ignore_end() {
    let end_directive = "end";
    let content = format!(
        "fn good() {{\n    return 1;\n}}\n\n# violet ignore {end_directive}\nfn still_good() {{\n    return 2;\n}}"
      );
    let result = preprocess_file(&content).unwrap();

    assert!(result.contains("fn good()"));
    assert!(result.contains("fn still_good()"));
  }

  #[test]
  fn test_preprocess_file_ignore_line() {
    let line_directive = "line";
    let content = format!("fn good() {{\n    return 1;\n}}\n\n// violet ignore {line_directive}\nlet bad_line = very_complex_calculation();\n\nfn also_good() {{\n    return 2;\n}}");
    let result = preprocess_file(&content).unwrap();

    assert!(result.contains("fn good()"));
    assert!(result.contains("fn also_good()"));
    assert!(!result.contains("let bad_line"));
    assert!(!result.contains("very_complex_calculation"));
  }

  #[test]
  fn test_preprocess_file_mixed_comment_styles() {
    let line_directive = "line";
    let start_directive = "start";
    let end_directive = "end";
    let content = format!("fn good() {{\n    return 1;\n}}\n\n// violet ignore {line_directive}\nlet bad1 = complex();\n\n# violet ignore {start_directive}\nfn bad_block() {{\n    return 2;\n}}\n/* violet ignore {end_directive} */\n\nfn also_good() {{\n    return 3;\n}}");
    let result = preprocess_file(&content).unwrap();

    assert!(result.contains("fn good()"));
    assert!(result.contains("fn also_good()"));
    assert!(!result.contains("let bad1"));
    assert!(!result.contains("fn bad_block()"));
  }

  #[test]
  fn test_preprocess_file_ignore_entire_file_directive() {
    let file_directive = "file";
    let content = format!("# violet ignore {file_directive}\nfn extremely_complex() {{\n    if deeply {{\n        if nested {{\n            if very {{\n                if much {{\n                    return 42;\n                }}\n            }}\n        }}\n    }}\n}}");

    let preprocessed = preprocess_file(&content);
    assert_eq!(preprocessed, None);
  }
}
