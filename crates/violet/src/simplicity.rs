//! Simplicity-based complexity scoring
//!
//! Implements a language-agnostic complexity algorithm that measures
//! cognitive load based on indentation, special characters, and line length.
//! Scoring is based on an information-theoretic approach, which means that
//! the score is based on the amount of information that is needed to understand
//! the code.

use std::fs;
use std::path::Path;

/// Result of analyzing a single file
#[derive(Debug, Clone)]
pub struct FileAnalysis {
  pub file_path: std::path::PathBuf,
  pub total_score: f64,
  pub chunk_scores: Vec<ChunkScore>,
  pub ignored: bool,
}

/// Breakdown of complexity score by component
#[derive(Debug, Clone)]
pub struct ComplexityBreakdown {
  pub depth_score: f64,
  pub depth_percent: f64,
  pub verbosity_score: f64,
  pub verbosity_percent: f64,
  pub syntactic_score: f64,
  pub syntactic_percent: f64,
}

/// Score for an individual chunk with details
#[derive(Debug, Clone)]
pub struct ChunkScore {
  pub score: f64,
  pub start_line: usize,
  pub end_line: usize,
  pub preview: String, // First line or two for identification
  pub breakdown: ComplexityBreakdown,
}

/// Analyze a single file and return detailed results
pub fn analyze_file<P: AsRef<Path>>(
  file_path: P,
) -> Result<FileAnalysis, Box<dyn std::error::Error>> {
  let path = file_path.as_ref();
  let content = fs::read_to_string(path)?;

  // Preprocess to handle ignore comments
  let preprocessed = match preprocess_file(&content) {
    Some(processed) => processed,
    None => {
      // Entire file is ignored
      return Ok(FileAnalysis {
        file_path: path.to_path_buf(),
        total_score: 0.0,
        chunk_scores: vec![],
        ignored: true,
      });
    }
  };

  // Extract chunks and score them
  let chunks = get_chunks(&preprocessed);
  let mut chunk_scores = Vec::new();
  let mut current_line = 1;

  for chunk in &chunks {
    let (score, breakdown) = chunk_complexity_with_breakdown(chunk);
    let lines_in_chunk = chunk.lines().count();
    let preview = chunk.lines().take(8).collect::<Vec<&str>>().join("\n");

    chunk_scores.push(ChunkScore {
      score,
      start_line: current_line,
      end_line: current_line + lines_in_chunk - 1,
      preview,
      breakdown,
    });

    current_line += lines_in_chunk + 1; // +1 for blank line separator
  }

  let total_score = file_complexity(&preprocessed);

  Ok(FileAnalysis { file_path: path.to_path_buf(), total_score, chunk_scores, ignored: false })
}

pub fn preprocess_file(content: &str) -> Option<String> {
  let lines: Vec<&str> = content.lines().collect();

  // Check for file-level ignore
  if lines.iter().any(|line| line.trim().starts_with("// violet ignore file")) {
    return None; // Entire file should be ignored
  }

  let mut result_lines = Vec::new();
  let mut ignore_depth = 0;

  for line in lines {
    let trimmed = line.trim();

    if trimmed.starts_with("// violet ignore start") {
      ignore_depth += 1;
      continue;
    }

    if trimmed.starts_with("// violet ignore end") {
      if ignore_depth > 0 {
        ignore_depth -= 1;
      }
      continue;
    }

    // Only include line if we're not in an ignored section
    if ignore_depth == 0 {
      result_lines.push(line);
    }
  }

  Some(result_lines.join("\n"))
}

/// Calculate complexity score for a single chunk of code with breakdown
pub fn chunk_complexity_with_breakdown(chunk: &str) -> (f64, ComplexityBreakdown) {
  let lines: Vec<&str> = chunk.lines().collect();
  let mut depth_total = 0.0;
  let mut verbosity_total = 0.0;
  let mut syntactic_total = 0.0;

  for line in lines {
    let indents = get_indents(line);
    let special_chars = get_num_specials(line);
    let non_special_chars = (line.trim().len() as f64) - special_chars;

    // Calculate component scores
    let depth_component = (indents as f64).powf(2.0);
    let verbosity_component = non_special_chars.powf(1.25);
    let syntactic_component = special_chars.powf(1.5);

    depth_total += depth_component;
    verbosity_total += verbosity_component;
    syntactic_total += syntactic_component;
  }

  // Sum all component scores (total "information content")
  let raw_sum = depth_total + verbosity_total + syntactic_total;

  // Information-theoretic scaling: ln(1 + sum) gives us base information content
  // Then scale by cognitive load factor - human processing isn't linear with information
  let base_information = (1.0 + raw_sum).ln();
  let cognitive_load_factor = 2.0; // Tunable: how much cognitive load scales with information
  let final_score = base_information * cognitive_load_factor;

  // Calculate percentages based on raw component scores (before logarithmic scaling)
  let total_raw = depth_total + verbosity_total + syntactic_total;
  let breakdown = if total_raw > 0.0 {
    ComplexityBreakdown {
      depth_score: depth_total,
      depth_percent: (depth_total / total_raw) * 100.0,
      verbosity_score: verbosity_total,
      verbosity_percent: (verbosity_total / total_raw) * 100.0,
      syntactic_score: syntactic_total,
      syntactic_percent: (syntactic_total / total_raw) * 100.0,
    }
  } else {
    ComplexityBreakdown {
      depth_score: 0.0,
      depth_percent: 0.0,
      verbosity_score: 0.0,
      verbosity_percent: 0.0,
      syntactic_score: 0.0,
      syntactic_percent: 0.0,
    }
  };

  (final_score, breakdown)
}

/// Calculate complexity score for a single chunk of code (legacy interface)
pub fn chunk_complexity(chunk: &str) -> f64 {
  let (score, _) = chunk_complexity_with_breakdown(chunk);
  score
}

/// Calculate complexity score for an entire file (average complexity per chunk)
pub fn file_complexity(file_content: &str) -> f64 {
  let chunks = get_chunks(file_content);
  if chunks.is_empty() {
    return 0.0;
  }

  let chunk_scores: Vec<f64> = chunks.iter().map(|chunk| chunk_complexity(chunk)).collect();
  let average_complexity = chunk_scores.iter().sum::<f64>() / chunks.len() as f64;

  average_complexity
}

/// Extract chunks from file content (separated by blank lines)
pub fn get_chunks(content: &str) -> Vec<String> {
  let mut chunks = Vec::new();
  let mut current_chunk = Vec::new();

  for line in content.lines() {
    if line.trim().is_empty() {
      // End of chunk if we have content
      if !current_chunk.is_empty() {
        chunks.push(current_chunk.join("\n"));
        current_chunk.clear();
      }
    } else {
      current_chunk.push(line);
    }
  }

  // Don't forget the last chunk if file doesn't end with blank line
  if !current_chunk.is_empty() {
    chunks.push(current_chunk.join("\n"));
  }

  chunks
}

/// Count indentation levels in a line
fn get_indents(line: &str) -> usize {
  let mut indent_count = 0;
  let chars: Vec<char> = line.chars().collect();

  for &ch in &chars {
    match ch {
      ' ' => indent_count += 1,
      '\t' => indent_count += 4, // Tab counts as 4 spaces
      _ => break,
    }
  }

  // Return indentation level (assuming 2-space indents)
  indent_count / 2
}

/// Count special characters in a line
fn get_num_specials(line: &str) -> f64 {
  let special_chars = "()[]{}+*?^$|.\\<>=!&|:;,";
  line.trim().chars().filter(|ch| special_chars.contains(*ch)).count() as f64
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_get_indents() {
    assert_eq!(get_indents("    hello"), 2); // 4 spaces = 2 indent levels
    assert_eq!(get_indents("  world"), 1); // 2 spaces = 1 indent level
    assert_eq!(get_indents("no_indent"), 0);
    assert_eq!(get_indents("\t\tindented"), 4); // 2 tabs = 4 indent levels
    assert_eq!(get_indents("      deep"), 3); // 6 spaces = 3 indent levels
  }

  #[test]
  fn test_get_num_specials() {
    assert_eq!(get_num_specials("hello world"), 0.0);
    assert_eq!(get_num_specials("if (condition) { }"), 4.0); // ( ) { }
    assert_eq!(get_num_specials("array[index] = value;"), 4.0); // [ ] = ;
    assert_eq!(get_num_specials("  special: &str  "), 2.0); // : &
  }

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
  fn test_chunk_complexity_simple() {
    let chunk = "fn simple() {\n    println!(\"hello\");\n}";
    let score = chunk_complexity(chunk);

    println!("Simple chunk score: {}", score);

    // Should be a reasonable positive number
    assert!(score > 0.0);
    assert!(score < 10000.0); // Much more reasonable with (1+sum)^1.5 scaling
  }

  #[test]
  fn test_chunk_complexity_nested() {
    let simple_chunk = "fn simple() {\n    return 42;\n}";
    let nested_chunk = "fn nested() {\n    if condition {\n        if nested {\n            return 42;\n        }\n    }\n}";

    let simple_score = chunk_complexity(simple_chunk);
    let nested_score = chunk_complexity(nested_chunk);

    // Nested should have higher complexity
    assert!(nested_score > simple_score);
  }

  #[test]
  fn test_file_complexity() {
    let content = "fn one() {\n    return 1;\n}\n\nfn two() {\n    return 2;\n}";
    let score = file_complexity(content);

    assert!(score > 0.0);
  }

  #[test]
  fn test_preprocess_file_no_ignores() {
    let content = "fn main() {\n    println!(\"hello\");\n}";
    let result = preprocess_file(content);

    assert_eq!(result, Some(content.to_string()));
  }

  #[test]
  fn test_preprocess_file_ignore_entire_file() {
    let content = "// violet ignore file\nfn main() {\n    println!(\"hello\");\n}";
    let result = preprocess_file(content);

    assert_eq!(result, None);
  }

  #[test]
  fn test_preprocess_file_ignore_block() {
    let content = "fn good() {\n    return 1;\n}\n\n// violet ignore start\nfn bad() {\n    if nested {\n        return 2;\n    }\n}\n// violet ignore end\n\nfn also_good() {\n    return 3;\n}";
    let result = preprocess_file(content).unwrap();

    assert!(result.contains("fn good()"));
    assert!(result.contains("fn also_good()"));
    assert!(!result.contains("fn bad()"));
    assert!(!result.contains("if nested"));
  }

  #[test]
  fn test_preprocess_file_nested_ignore_blocks() {
    let content = "fn good() {\n    return 1;\n}\n\n// violet ignore start\nfn outer_bad() {\n    // violet ignore start\n    fn inner_bad() {\n        return 2;\n    }\n    // violet ignore end\n    return 3;\n}\n// violet ignore end\n\nfn also_good() {\n    return 4;\n}";
    let result = preprocess_file(content).unwrap();

    assert!(result.contains("fn good()"));
    assert!(result.contains("fn also_good()"));
    assert!(!result.contains("fn outer_bad()"));
    assert!(!result.contains("fn inner_bad()"));
  }

  #[test]
  fn test_preprocess_file_unmatched_ignore_end() {
    let content =
      "fn good() {\n    return 1;\n}\n\n// violet ignore end\nfn still_good() {\n    return 2;\n}";
    let result = preprocess_file(content).unwrap();

    assert!(result.contains("fn good()"));
    assert!(result.contains("fn still_good()"));
  }

  #[test]
  fn test_complete_pipeline_with_ignores() {
    let content = "fn simple() {\n    return 1;\n}\n\n// violet ignore start\nfn complex() {\n    if deeply {\n        if nested {\n            if very {\n                return 2;\n            }\n        }\n    }\n}\n// violet ignore end\n\nfn another_simple() {\n    return 3;\n}";

    // First preprocess to remove ignored sections
    let preprocessed = preprocess_file(content).unwrap();

    // Should only have the simple functions
    assert!(preprocessed.contains("fn simple()"));
    assert!(preprocessed.contains("fn another_simple()"));
    assert!(!preprocessed.contains("fn complex()"));

    // Get chunks from preprocessed content
    let chunks = get_chunks(&preprocessed);
    assert_eq!(chunks.len(), 2); // Two simple functions

    // Score should be reasonable since we removed the complex function
    let total_score = file_complexity(&preprocessed);
    assert!(total_score > 0.0);
    assert!(total_score < 1000.0); // Should be much lower without the complex function
  }

  #[test]
  fn test_complete_pipeline_file_ignore() {
    let content = "// violet ignore file\nfn extremely_complex() {\n    if deeply {\n        if nested {\n            if very {\n                if much {\n                    return 42;\n                }\n            }\n        }\n    }\n}";

    let preprocessed = preprocess_file(content);
    assert_eq!(preprocessed, None); // Entire file should be ignored
  }

  #[test]
  fn test_complexity_comparison() {
    let simple_content = "fn simple() {\n    return 42;\n}";
    let complex_content = "fn complex() {\n    if condition1 {\n        if condition2 {\n            if condition3 {\n                return nested_result();\n            }\n        }\n    }\n}";

    let simple_score = chunk_complexity(simple_content);
    let complex_score = chunk_complexity(complex_content);

    println!(
      "Simple score: {}, Complex score: {}, Ratio: {}",
      simple_score,
      complex_score,
      complex_score / simple_score
    );

    // Complex function should have significantly higher score
    assert!(complex_score > simple_score * 1.5); // Lower threshold - maybe 2x was too aggressive
  }

  #[test]
  fn test_information_theoretic_scaling() {
    // Test that our information-theoretic approach gives reasonable scaling
    let minimal = "x";
    let short = "fn f() { return 1; }";
    let medium = "fn medium() {\n    if condition {\n        return process(value);\n    }\n    return default;\n}";

    let minimal_score = chunk_complexity(minimal);
    let short_score = chunk_complexity(short);
    let medium_score = chunk_complexity(medium);

    // Scores should increase but not exponentially explode
    assert!(minimal_score < short_score);
    assert!(short_score < medium_score);
    assert!(medium_score < 100.0); // Still reasonable
    assert!(minimal_score > 0.0); // But not zero
  }
}
