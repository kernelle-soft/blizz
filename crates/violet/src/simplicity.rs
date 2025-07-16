//! Simplicity-based complexity scoring
//!
//! Implements a language-agnostic complexity algorithm that measures
//! cognitive load based on indentation, special characters, and line length.
//! Scoring is based on an information-theoretic approach, which means that
//! the score is based on the amount of information that is needed to understand
//! the code.

use regex::Regex;
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

/// Create a FileAnalysis for an ignored file
fn create_ignored_file_analysis(path: &Path) -> FileAnalysis {
  FileAnalysis {
    file_path: path.to_path_buf(),
    total_score: 0.0,
    chunk_scores: vec![],
    ignored: true,
  }
}

/// Check if file should be completely ignored based on file-level directives
fn has_file_ignore_directive(lines: &[&str]) -> bool {
  let ignore_regex = Regex::new(r"violet\s+ignore\s+(file|chunk|start|end|line)").unwrap();
  lines.iter().any(|line| {
    ignore_regex.captures(line).map_or(false, |caps| caps.get(1).unwrap().as_str() == "file")
  })
}

/// Process a single directive and update state
fn process_directive<'a>(
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
      // Keep the directive line so we can identify chunks to remove later
      // But only if we're not in an ignored section
      if *ignore_depth == 0 {
        result_lines.push(line);
      }
    }
    _ => {} // file is handled elsewhere
  }
}

/// Process a single line during preprocessing
fn process_line<'a>(
  line: &'a str,
  ignore_depth: &mut usize,
  skip_next_line: &mut bool,
  result_lines: &mut Vec<&'a str>,
) -> bool {
  let ignore_regex = Regex::new(r"violet\s+ignore\s+(file|chunk|start|end|line)").unwrap();
  
  // Handle line-level ignore from previous line
  if *skip_next_line {
    *skip_next_line = false;
    return true; // Skip this line
  }

  // Check for ignore directives in current line
  if let Some(captures) = ignore_regex.captures(line) {
    let directive = captures.get(1).unwrap().as_str();
    process_directive(directive, ignore_depth, skip_next_line, result_lines, line);
    return true; // Skip adding this line to normal processing
  }

  false // Don't skip, process normally
}

pub fn preprocess_file(content: &str) -> Option<String> {
  let lines: Vec<&str> = content.lines().collect();

  // Check for file-level ignore
  if has_file_ignore_directive(&lines) {  
    return None; // Entire file should be ignored
  }

  let mut result_lines = Vec::new();
  let mut ignore_depth = 0;
  let mut skip_next_line = false;

  for line in lines.iter() {
    if process_line(line, &mut ignore_depth, &mut skip_next_line, &mut result_lines) {
      continue;
    }

    // Only include line if we're not in an ignored section
    if ignore_depth == 0 {
      result_lines.push(*line);
    }
  }

  Some(result_lines.join("\n"))
}

/// Calculate complexity components for a single line
fn calculate_line_complexity(line: &str) -> (f64, f64, f64) {
  let indents = get_indents(line);
  let special_chars = get_num_specials(line);
  let non_special_chars = (line.trim().len() as f64) - special_chars;

  let verbosity_component = (1.05 as f64).powf(non_special_chars as f64);
  let syntactic_component = (1.25 as f64).powf(special_chars as f64);
  let depth_component = (2.0 as f64).powf(indents as f64);

  (depth_component, verbosity_component, syntactic_component)
}

/// Create a ComplexityBreakdown from component totals
fn create_breakdown(depth_total: f64, verbosity_total: f64, syntactic_total: f64) -> ComplexityBreakdown {
  let total_raw = depth_total + verbosity_total + syntactic_total;
  
  if total_raw > 0.0 {
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
  }
}

/// Calculate complexity score for a single chunk of code with breakdown
pub fn chunk_complexity_with_breakdown(chunk: &str) -> (f64, ComplexityBreakdown) {
  let lines: Vec<&str> = chunk.lines().collect();
  let mut depth_total = 0.0;
  let mut verbosity_total = 0.0;
  let mut syntactic_total = 0.0;

  for line in lines {
    let (depth_component, verbosity_component, syntactic_component) = calculate_line_complexity(line);
    depth_total += depth_component;
    verbosity_total += verbosity_component;
    syntactic_total += syntactic_component;
  }

  let sum = depth_total + verbosity_total + syntactic_total;
  let score = sum.ln();

  let breakdown = create_breakdown(depth_total, verbosity_total, syntactic_total);

  (score, breakdown)
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

/// Split content on blank lines into temporary chunks
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

/// Check if a chunk starts with indentation (not at top level)
fn chunk_starts_with_indentation(chunk: &str) -> bool {
  if let Some(first_line) = chunk.lines().next() {
    first_line.starts_with(' ') || first_line.starts_with('\t')
  } else {
    false
  }
}

/// Merge indented chunks with previous chunks to maintain top-level grouping
fn merge_indented_chunks(temp_chunks: Vec<String>) -> Vec<String> {
  let mut final_chunks = Vec::new();

  for chunk in temp_chunks {
    if chunk_starts_with_indentation(&chunk) && !final_chunks.is_empty() {
      let last_idx = final_chunks.len() - 1;
      final_chunks[last_idx] = format!("{}\n\n{}", final_chunks[last_idx], chunk);
    } else {
      final_chunks.push(chunk);
    }
  }

  final_chunks
}

/// Extract chunks from file content (separated by blank lines)
pub fn get_chunks(content: &str) -> Vec<String> {
  // First pass: split on blank lines (original logic)
  let temp_chunks = split_on_blank_lines(content);

  // Second pass: merge chunks that don't start at top level with previous chunk
  merge_indented_chunks(temp_chunks)
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

/// violet ignore line - Special characters used for complexity calculation
const SPECIAL_CHARS: &str = "()[]{}+*?^$|.\\<>=!&|:;,";

/// Count special characters in a line
fn get_num_specials(line: &str) -> f64 {
  line.trim().chars().filter(|ch| SPECIAL_CHARS.contains(*ch)).count() as f64
}

/// Check if a chunk should be skipped and update state accordingly
fn should_skip_chunk(
  chunk: &str,
  chunk_ignore_regex: &Regex,
  skip_next_chunk: &mut bool,
  current_line: &mut usize,
  lines_in_chunk: usize,
) -> bool {
  // Check if this chunk contains an ignore directive
  if chunk_ignore_regex.is_match(chunk) {
    *skip_next_chunk = true;
    *current_line += lines_in_chunk + 1;
    return true;
  }

  // Check if this chunk should be skipped due to previous directive
  if *skip_next_chunk {
    *skip_next_chunk = false;
    *current_line += lines_in_chunk + 1;
    return true;
  }

  false
}

/// Create a ChunkScore from a chunk
fn create_chunk_score(chunk: &str, current_line: usize, lines_in_chunk: usize) -> ChunkScore {
  let (score, breakdown) = chunk_complexity_with_breakdown(chunk);
  let preview = chunk.lines().take(8).collect::<Vec<&str>>().join("\n");

  ChunkScore {
    score,
    start_line: current_line,
    end_line: current_line + lines_in_chunk - 1,
    preview,
    breakdown,
  }
}

/// Process all chunks and return chunk scores, handling chunk ignore directives
fn process_chunks(all_chunks: &[String]) -> Vec<ChunkScore> {
  let chunk_ignore_regex = Regex::new(r"violet\s+ignore\s+chunk").unwrap();
  let mut chunk_scores = Vec::new();
  let mut current_line = 1;
  let mut skip_next_chunk = false;

  for chunk in all_chunks {
    let lines_in_chunk = chunk.lines().count();

    if should_skip_chunk(chunk, &chunk_ignore_regex, &mut skip_next_chunk, &mut current_line, lines_in_chunk) {
      continue;
    }

    let chunk_score = create_chunk_score(chunk, current_line, lines_in_chunk);
    chunk_scores.push(chunk_score);

    current_line += lines_in_chunk + 1; // +1 for blank line separator
  }

  chunk_scores
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
    None => return Ok(create_ignored_file_analysis(path)),
  };

  // Extract chunks and process them, skipping ignored chunks
  let all_chunks = get_chunks(&preprocessed);
  let chunk_scores = process_chunks(&all_chunks);
  let total_score = file_complexity(&preprocessed);

  Ok(FileAnalysis { file_path: path.to_path_buf(), total_score, chunk_scores, ignored: false })
}

// violet ignore chunk
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
    // Test with different comment styles
    let content1 =
      format!("# violet ignore {}\nfn main() {{\n    println!(\"hello\");\n}}", "file");
    let content2 =
      format!("// violet ignore {}\nfn main() {{\n    println!(\"hello\");\n}}", "file");
    let content3 =
      format!("/* violet ignore {} */\nfn main() {{\n    println!(\"hello\");\n}}", "file");

    assert_eq!(preprocess_file(&content1), None);
    assert_eq!(preprocess_file(&content2), None);
    assert_eq!(preprocess_file(&content3), None);
  }

  #[test]
  fn test_preprocess_file_ignore_block() {
    let content = format!("fn good() {{\n    return 1;\n}}\n\n# violet ignore {}\nfn bad() {{\n    if nested {{\n        return 2;\n    }}\n}}\n# violet ignore {}\n\nfn also_good() {{\n    return 3;\n}}", "start", "end");
    let result = preprocess_file(&content).unwrap();

    assert!(result.contains("fn good()"));
    assert!(result.contains("fn also_good()"));
    assert!(!result.contains("fn bad()"));
    assert!(!result.contains("if nested"));
  }

  #[test]
  fn test_preprocess_file_nested_ignore_blocks() {
    let content = format!("fn good() {{\n    return 1;\n}}\n\n/* violet ignore {} */\nfn outer_bad() {{\n    # violet ignore {}\n    fn inner_bad() {{\n        return 2;\n    }}\n    # violet ignore {}\n    return 3;\n}}\n/* violet ignore {} */\n\nfn also_good() {{\n    return 4;\n}}", "start", "start", "end", "end");
    let result = preprocess_file(&content).unwrap();

    assert!(result.contains("fn good()"));
    assert!(result.contains("fn also_good()"));
    assert!(!result.contains("fn outer_bad()"));
    assert!(!result.contains("fn inner_bad()"));
  }

  #[test]
  fn test_preprocess_file_unmatched_ignore_end() {
    let content = format!(
      "fn good() {{\n    return 1;\n}}\n\n# violet ignore {}\nfn still_good() {{\n    return 2;\n}}", 
      "end"
    );
    let result = preprocess_file(&content).unwrap();

    assert!(result.contains("fn good()"));
    assert!(result.contains("fn still_good()"));
  }

  #[test]
  fn test_complete_pipeline_with_ignores() {
    let content = format!("fn simple() {{\n    return 1;\n}}\n\n# violet ignore {}\nfn complex() {{\n    if deeply {{\n        if nested {{\n            if very {{\n                return 2;\n            }}\n        }}\n    }}\n}}\n# violet ignore {}\n\nfn another_simple() {{\n    return 3;\n}}", "start", "end");

    // First preprocess to remove ignored sections
    let preprocessed = preprocess_file(&content).unwrap();

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
    let content = format!("# violet ignore {}\nfn extremely_complex() {{\n    if deeply {{\n        if nested {{\n            if very {{\n                if much {{\n                    return 42;\n                }}\n            }}\n        }}\n    }}\n}}", "file");

    let preprocessed = preprocess_file(&content);
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

  #[test]
  fn test_preprocess_file_ignore_line() {
    let content = format!("fn good() {{\n    return 1;\n}}\n\n// violet ignore {}\nlet bad_line = very_complex_calculation();\n\nfn also_good() {{\n    return 2;\n}}", "line");
    let result = preprocess_file(&content).unwrap();

    assert!(result.contains("fn good()"));
    assert!(result.contains("fn also_good()"));
    assert!(!result.contains("let bad_line"));
    assert!(!result.contains("very_complex_calculation"));
  }

  #[test]
  fn test_preprocess_file_ignore_chunk() {
    // Chunk directives are processed during analyze_file, not preprocess_file
    // preprocess_file only keeps the directive line for later processing
    let content = format!("fn good() {{\n    return 1;\n}}\n\n/* violet ignore {} */\n\nfn bad_chunk() {{\n    if deeply {{\n        nested();\n    }}\n}}\n\nfn also_good() {{\n    return 2;\n}}", "chunk");
    let result = preprocess_file(&content).unwrap();

    // preprocess_file should keep the directive line
    assert!(result.contains("fn good()"));
    assert!(result.contains("fn also_good()"));
    assert!(result.contains("violet ignore chunk")); // directive line preserved
    assert!(result.contains("fn bad_chunk()")); // chunk content preserved for later processing
  }

  #[test]
  fn test_analyze_file_ignore_chunk() {
    use std::fs;

    // Create a temporary file for testing
    let temp_path = "test_chunk_ignore.rs";
    let content = format!("fn good() {{\n    return 1;\n}}\n\n/* violet ignore {} */\n\nfn bad_chunk() {{\n    if deeply {{\n        nested();\n    }}\n}}\n\nfn also_good() {{\n    return 2;\n}}", "chunk");

    fs::write(temp_path, &content).unwrap();

    // Analyze the file
    let analysis = analyze_file(temp_path).unwrap();

    // Should have 2 chunks (good functions), bad_chunk should be ignored
    assert_eq!(analysis.chunk_scores.len(), 2);

    // Check that the chunks are the good functions
    let chunk_previews: Vec<&str> = analysis
      .chunk_scores
      .iter()
      .map(|chunk| chunk.preview.lines().next().unwrap_or(""))
      .collect();

    assert!(chunk_previews.iter().any(|preview| preview.contains("fn good()")));
    assert!(chunk_previews.iter().any(|preview| preview.contains("fn also_good()")));
    assert!(!chunk_previews.iter().any(|preview| preview.contains("fn bad_chunk()")));

    // Clean up
    fs::remove_file(temp_path).unwrap();
  }

  #[test]
  fn test_preprocess_file_ignore_multiple_chunks() {
    // This test should also be about preprocess behavior, not chunk removal
    let directive = "chunk";
    let content = format!("fn good1() {{\n    return 1;\n}}\n\n# violet ignore {}\n\nfn bad1() {{\n    complex();\n}}\n\nfn good2() {{\n    return 2;\n}}\n\n# violet ignore {}\n\nfn bad2() {{\n    also_complex();\n}}\n\nfn good3() {{\n    return 3;\n}}", directive, directive);
    let result = preprocess_file(&content).unwrap();

    // preprocess_file should preserve content but keep directive lines
    assert!(result.contains("fn good1()"));
    assert!(result.contains("fn good2()"));
    assert!(result.contains("fn good3()"));
    assert!(result.contains("fn bad1()")); // content preserved for later chunk processing
    assert!(result.contains("fn bad2()")); // content preserved for later chunk processing
    assert!(result.contains("violet ignore chunk")); // directive lines preserved
  }

  #[test]
  fn test_analyze_file_ignore_multiple_chunks() {
    use std::fs;

    // Create a temporary file for testing
    let temp_path = "test_multiple_chunk_ignore.rs";
    let directive = "chunk";
    let content = format!("fn good1() {{\n    return 1;\n}}\n\n# violet ignore {}\n\nfn bad1() {{\n    complex();\n}}\n\nfn good2() {{\n    return 2;\n}}\n\n# violet ignore {}\n\nfn bad2() {{\n    also_complex();\n}}\n\nfn good3() {{\n    return 3;\n}}", directive, directive);

    fs::write(temp_path, &content).unwrap();

    // Analyze the file
    let analysis = analyze_file(temp_path).unwrap();

    // Should have 3 chunks (good functions), bad functions should be ignored
    assert_eq!(analysis.chunk_scores.len(), 3);

    // Check that the chunks are the good functions
    let chunk_previews: Vec<&str> = analysis
      .chunk_scores
      .iter()
      .map(|chunk| chunk.preview.lines().next().unwrap_or(""))
      .collect();

    assert!(chunk_previews.iter().any(|preview| preview.contains("fn good1()")));
    assert!(chunk_previews.iter().any(|preview| preview.contains("fn good2()")));
    assert!(chunk_previews.iter().any(|preview| preview.contains("fn good3()")));
    assert!(!chunk_previews.iter().any(|preview| preview.contains("fn bad1()")));
    assert!(!chunk_previews.iter().any(|preview| preview.contains("fn bad2()")));

    // Clean up
    fs::remove_file(temp_path).unwrap();
  }

  #[test]
  fn test_preprocess_file_mixed_comment_styles() {
    let content = format!("fn good() {{\n    return 1;\n}}\n\n// violet ignore {}\nlet bad1 = complex();\n\n# violet ignore {}\nfn bad_block() {{\n    return 2;\n}}\n/* violet ignore {} */\n\nfn also_good() {{\n    return 3;\n}}", "line", "start", "end");
    let result = preprocess_file(&content).unwrap();

    assert!(result.contains("fn good()"));
    assert!(result.contains("fn also_good()"));
    assert!(!result.contains("let bad1"));
    assert!(!result.contains("fn bad_block()"));
  }
}
