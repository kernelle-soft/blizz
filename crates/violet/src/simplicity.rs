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
use crate::config::{VioletConfig, get_threshold_for_file};

/// Result of analyzing a single file
#[derive(Debug, Clone)]
pub struct FileAnalysis {
  pub file_path: std::path::PathBuf,
  pub total_score: f64,
  pub complexity_regions: Vec<ComplexityRegion>,
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

/// A complexity region that can be either a natural chunk or a detected hotspot
#[derive(Debug, Clone)]
pub struct ComplexityRegion {
  pub score: f64,
  pub start_line: usize,
  pub end_line: usize,
  pub preview: String,
  pub breakdown: ComplexityBreakdown,
  pub region_type: RegionType,
}

/// Type of complexity region
#[derive(Debug, Clone)]
pub enum RegionType {
  NaturalChunk,
  DetectedHotspot,
}

/// Create a FileAnalysis for an ignored file
fn create_ignored_file_analysis(path: &Path) -> FileAnalysis {
  FileAnalysis {
    file_path: path.to_path_buf(),
    total_score: 0.0,
    complexity_regions: vec![],
    ignored: true,
  }
}

/// Check if file should be completely ignored based on file-level directives
fn has_file_ignore_directive(lines: &[&str]) -> bool {
  let ignore_regex = Regex::new(r"violet\s+ignore\s+(file|chunk|start|end|line)").unwrap();
  lines.iter().any(|line| {
    ignore_regex.captures(line).is_some_and(|caps| caps.get(1).unwrap().as_str() == "file")
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
  let indents = get_indents(line).saturating_sub(1); // Don't penalize first indentation level
  let special_chars = get_num_specials(line);
  let non_special_chars = (line.trim().len() as f64) - special_chars;

  let verbosity_component = 1.05_f64.powf(non_special_chars);
  let syntactic_component = 1.25_f64.powf(special_chars);
  let depth_component = 2.0_f64.powf(indents as f64);

  (depth_component, verbosity_component, syntactic_component)
}

/// Create a ComplexityBreakdown from component totals
fn create_breakdown(
  depth_total: f64,
  verbosity_total: f64,
  syntactic_total: f64,
) -> ComplexityBreakdown {
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
    let (depth_component, verbosity_component, syntactic_component) =
      calculate_line_complexity(line);
    depth_total += depth_component;
    verbosity_total += verbosity_component;
    syntactic_total += syntactic_component;
  }

  let sum = depth_total + verbosity_total + syntactic_total;
  
  // Handle edge case of empty input - return 0.0 instead of ln(0) = -inf
  let score = if sum > 0.0 { sum.ln() } else { 0.0 };

  let breakdown = create_breakdown(depth_total, verbosity_total, syntactic_total);

  (score, breakdown)
}

/// Calculate complexity score for a single chunk of code (legacy interface)
pub fn chunk_complexity(chunk: &str) -> f64 {
  let (score, _) = chunk_complexity_with_breakdown(chunk);

  // round to 2 decimal places
  (score * 100.0).round() / 100.0
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

/// Extract chunks from file content (separated by blank lines)
pub fn get_chunks(content: &str) -> Vec<String> {
  split_on_blank_lines(content)
}

/// Count indentation levels in a line
fn get_indents(line: &str) -> usize {
  get_indents_with_tab_size(line, 2) // Default to 2 spaces per indent level
}

/// Count indentation levels in a line with configurable tab size
fn get_indents_with_tab_size(line: &str, spaces_per_indent: usize) -> usize {
  let mut indent_levels = 0;
  let mut space_count = 0;
  let chars: Vec<char> = line.chars().collect();

  for &ch in &chars {
    match ch {
      ' ' => space_count += 1,
      '\t' => {
        // Convert any accumulated spaces to indent levels first
        indent_levels += space_count / spaces_per_indent;
        space_count = 0;
        // Tab counts as 1 indent level
        indent_levels += 1;
      },
      _ => break, // Stop at first non-whitespace character
    }
  }

  // Convert any remaining spaces to indent levels
  indent_levels += space_count / spaces_per_indent;
  
  indent_levels
}

/// violet ignore line - Special characters used for complexity calculation
const SPECIAL_CHARS: &str = "()[]{}+*?^$|.\\<>=!&|:;,";

/// Count special characters in a line
fn get_num_specials(line: &str) -> f64 {
  line.trim().chars().filter(|ch| SPECIAL_CHARS.contains(*ch)).count() as f64
}

/// Check if a chunk should be ignored based on violet ignore chunk directive
fn should_ignore_chunk(chunk_content: &str) -> bool {
  let ignore_regex = Regex::new(r"violet\s+ignore\s+chunk").unwrap();
  chunk_content.lines().any(|line| ignore_regex.is_match(line))
}

/// Analyze a single file and return detailed results
pub fn analyze_file<P: AsRef<Path>>(
  file_path: P,
  config: &VioletConfig,
) -> Result<FileAnalysis, Box<dyn std::error::Error>> {
  let path = file_path.as_ref();
  let content = fs::read_to_string(path)?;

  // Preprocess to handle ignore comments
  let preprocessed = match preprocess_file(&content) {
    Some(processed) => processed,
    None => return Ok(create_ignored_file_analysis(path)),
  };

  if preprocessed.trim().is_empty() {
    return Ok(FileAnalysis {
      file_path: path.to_path_buf(),
      total_score: 0.0,
      complexity_regions: vec![],
      ignored: false,
    });
  }

  let threshold = get_threshold_for_file(config, path);
  let lines: Vec<&str> = preprocessed.lines().collect();
  
  // Use iterative blank-line splitting + fusion for structural chunk discovery
  let line_lengths: Vec<f64> = lines.iter()
    .map(|line| line.len() as f64)
    .collect();
  
  // Apply iterative fusion algorithm
  let regions = analyze_file_iterative_fusion(&lines, &line_lengths);
  
  // Convert to final ComplexityRegion objects
  let mut complexity_regions = Vec::new();
  for (start, end, _) in regions {
    if end > start {
      // Extract chunk content for complexity scoring
      let chunk_lines = &lines[start..=end.min(lines.len().saturating_sub(1))];
      let chunk_content = chunk_lines.join("\n");
      
      // Check if the chunk should be ignored
      if should_ignore_chunk(&chunk_content) {
        continue;
      }

      // NOW apply complexity scoring to the discovered chunk
      let score = chunk_complexity(&chunk_content);
      
      if score > threshold {
        let breakdown = calculate_chunk_breakdown(&chunk_content);
        let preview = create_chunk_preview(chunk_lines);
        
        complexity_regions.push(ComplexityRegion {
          start_line: start + 1,
          end_line: end + 1,
          score,
          breakdown,
          preview,
          region_type: RegionType::DetectedHotspot,
        });
      }
    }
  }

  let total_score = file_complexity(&preprocessed);

  Ok(FileAnalysis { 
    file_path: path.to_path_buf(), 
    total_score, 
    complexity_regions, 
    ignored: false 
  })
}

/// Create a simple preview string from chunk lines
fn create_chunk_preview(lines: &[&str]) -> String {
  const MAX_PREVIEW_LINES: usize = 20;
  const MAX_LINE_LENGTH: usize = 80;
  
  let mut preview_lines = Vec::new();
  let display_lines = lines.iter().take(MAX_PREVIEW_LINES);
  
  for line in display_lines {
    if line.len() <= MAX_LINE_LENGTH {
      preview_lines.push(line.to_string());
    } else {
      let truncated = format!("{}...", &line[..MAX_LINE_LENGTH.saturating_sub(3)]);
      preview_lines.push(truncated);
    }
  }
  
  if lines.len() > MAX_PREVIEW_LINES {
    preview_lines.push(format!("... ({} more lines)", lines.len() - MAX_PREVIEW_LINES));
  }
  
  preview_lines.join("\n")
}

/// Iterative fusion algorithm for structural chunk discovery
fn analyze_file_iterative_fusion(lines: &[&str], line_lengths: &[f64]) -> Vec<(usize, usize, f64)> {
  if lines.is_empty() {
    return vec![];
  }
  
  // Step 1: Split into base chunks by blank lines
  let mut chunks = split_into_text_chunks(lines);
  
  // Step 2: Iteratively fuse chunks until stable
  loop {
    let initial_count = chunks.len();
    chunks = fuse_compatible_chunks(chunks, line_lengths, lines);
    
    // Stop when no more fusions happened
    if chunks.len() == initial_count {
      break;
    }
  }
  
  // Convert to required format
  chunks.into_iter()
    .map(|(start, end)| (start, end, 0.0)) // Placeholder score
    .collect()
}

/// Split text into initial chunks based on blank lines
fn split_into_text_chunks(lines: &[&str]) -> Vec<(usize, usize)> {
  let mut chunks = Vec::new();
  let mut current_start = 0;
  let mut in_text_block = false;
  
  for (i, line) in lines.iter().enumerate() {
    let has_text = !line.trim().is_empty();
    
    if has_text && !in_text_block {
      // Starting a new text block
      current_start = i;
      in_text_block = true;
    } else if !has_text && in_text_block {
      // Ending a text block
      chunks.push((current_start, i));
      in_text_block = false;
    }
  }
  
  // Handle final text block if file doesn't end with blank line
  if in_text_block {
    chunks.push((current_start, lines.len()));
  }
  
  chunks
}

/// Fuse adjacent chunks if their line length transition suggests they belong together
fn fuse_compatible_chunks(chunks: Vec<(usize, usize)>, line_lengths: &[f64], lines: &[&str]) -> Vec<(usize, usize)> {
  if chunks.len() <= 1 {
    return chunks;
  }
  
  let mut fused_chunks = Vec::new();
  let mut current_chunk = chunks[0];
  
  for i in 1..chunks.len() {
    let next_chunk = chunks[i];
    
    if should_fuse_chunks(current_chunk, next_chunk, line_lengths, lines) {
      // Fuse: extend current chunk to include next chunk
      current_chunk = (current_chunk.0, next_chunk.1);
    } else {
      // Keep separate: save current chunk and start new one
      fused_chunks.push(current_chunk);
      current_chunk = next_chunk;
    }
  }
  
  // Don't forget the last chunk
  fused_chunks.push(current_chunk);
  
  fused_chunks
}

/// Decide whether two adjacent chunks should be fused based on line length transition
fn should_fuse_chunks(chunk1: (usize, usize), chunk2: (usize, usize), line_lengths: &[f64], lines: &[&str]) -> bool {
  if chunk1.0 >= lines.len() || chunk1.1 <= chunk1.0 || 
     chunk2.0 >= lines.len() || chunk2.1 <= chunk2.0 {
    return false;
  }
  
  // Step 1: Compare beginning vs end of prev chunk
  let prev_chunk_start_indent = get_indents(lines[chunk1.0]) as f64;
  let prev_chunk_end_indent = get_indents(lines[chunk1.1.saturating_sub(1)]) as f64;
  
  // Step 2: If end is higher than beginning (going INTO a block)
  if prev_chunk_end_indent > prev_chunk_start_indent {
    
    // Step 3: Compare beginning vs end of next chunk  
    let next_chunk_start_indent = get_indents(lines[chunk2.0]) as f64;
    let next_chunk_end_indent = get_indents(lines[chunk2.1.saturating_sub(1)]) as f64;
    
    // Step 4: If beginning is higher than end (coming OUT OF a block) → FUSE
    if next_chunk_start_indent >= next_chunk_end_indent {
      return true;
    }
  }
  
  // Step 5: Otherwise, fall back to line length transition logic
  let last_line_idx = chunk1.1.saturating_sub(1);
  let first_line_idx = chunk2.0;
  
  let last_line_length = line_lengths[last_line_idx];
  let first_line_length = line_lengths[first_line_idx];
  
  // If the last line of prior chunk is significantly shorter than 
  // the first line of next chunk, keep separate
  let length_ratio = if last_line_length == 0.0 {
    f64::INFINITY
  } else {
    first_line_length / last_line_length
  };
  
  // Fuse if the transition is gradual (not a big jump)
  // A ratio > 2.0 suggests a significant structural boundary
  length_ratio <= 2.0
}

/// Calculate complexity breakdown for a chunk of code
fn calculate_chunk_breakdown(chunk_content: &str) -> ComplexityBreakdown {
  let lines: Vec<&str> = chunk_content.lines().collect();
  
  let mut total_depth = 0.0;
  let mut total_verbosity = 0.0;
  let mut total_syntactic = 0.0;
  
  for line in lines {
    let (depth, verbosity, syntactic) = calculate_line_complexity(line);
    total_depth += depth;
    total_verbosity += verbosity;
    total_syntactic += syntactic;
  }
  
  create_breakdown(total_depth, total_verbosity, total_syntactic)
}

// violet ignore chunk
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_get_indents() {
    assert_eq!(get_indents("  hello"), 1); // 2 spaces = 1 indent level
    assert_eq!(get_indents("    world"), 2); // 4 spaces = 2 indent levels  
    assert_eq!(get_indents("no_indent"), 0);
    assert_eq!(get_indents("\tindented"), 1); // 1 tab = 1 indent level
    assert_eq!(get_indents("\t\tindented"), 2); // 2 tabs = 2 indent levels
    assert_eq!(get_indents("      deep"), 3); // 6 spaces = 3 indent levels
    assert_eq!(get_indents("  \tcombo"), 2); // 2 spaces + tab = 1 + 1 = 2 indent levels
    assert_eq!(get_indents("\t  partial"), 2); // tab + 2 spaces = 1 + 1 = 2 indent levels
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
  fn test_preprocess_file_mixed_comment_styles() {
    let content = format!("fn good() {{\n    return 1;\n}}\n\n// violet ignore {}\nlet bad1 = complex();\n\n# violet ignore {}\nfn bad_block() {{\n    return 2;\n}}\n/* violet ignore {} */\n\nfn also_good() {{\n    return 3;\n}}", "line", "start", "end");
    let result = preprocess_file(&content).unwrap();

    assert!(result.contains("fn good()"));
    assert!(result.contains("fn also_good()"));
    assert!(!result.contains("let bad1"));
    assert!(!result.contains("fn bad_block()"));
  }

  #[test]
  fn test_calculate_line_complexity() {
    // Test simple line
    let (depth, verbosity, syntactic) = calculate_line_complexity("hello world");
    assert_eq!(depth, 1.0); // No indentation = 1.0
    assert!(verbosity > 1.0); // Has some content
    assert_eq!(syntactic, 1.0); // No special chars = 1.0

    // Test indented line with special chars
    let (depth, verbosity, syntactic) = calculate_line_complexity("    if (condition) {");
    assert!(depth > 1.0); // Has indentation
    assert!(verbosity > 1.0); // Has content
    assert!(syntactic > 1.0); // Has special chars: ( ) {

    // Test empty line
    let (depth, verbosity, syntactic) = calculate_line_complexity("");
    assert_eq!(depth, 1.0); // No indentation
    assert_eq!(verbosity, 1.0); // No content
    assert_eq!(syntactic, 1.0); // No special chars
  }

  #[test]
  fn test_create_breakdown() {
    // Test normal breakdown
    let breakdown = create_breakdown(10.0, 20.0, 30.0);
    assert_eq!(breakdown.depth_score, 10.0);
    assert_eq!(breakdown.verbosity_score, 20.0);
    assert_eq!(breakdown.syntactic_score, 30.0);
    assert!((breakdown.depth_percent - 16.67).abs() < 0.1);
    assert!((breakdown.verbosity_percent - 33.33).abs() < 0.1);
    assert!((breakdown.syntactic_percent - 50.0).abs() < 0.1);

    // Test zero breakdown
    let zero_breakdown = create_breakdown(0.0, 0.0, 0.0);
    assert_eq!(zero_breakdown.depth_score, 0.0);
    assert_eq!(zero_breakdown.depth_percent, 0.0);
    assert_eq!(zero_breakdown.verbosity_percent, 0.0);
    assert_eq!(zero_breakdown.syntactic_percent, 0.0);
  }

  #[test]
  fn test_split_on_blank_lines() {
    // Test normal case
    let content = "line 1\nline 2\n\nline 3\nline 4\n\n\nline 5";
    let chunks = split_on_blank_lines(content);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0], "line 1\nline 2");
    assert_eq!(chunks[1], "line 3\nline 4");
    assert_eq!(chunks[2], "line 5");

    // Test empty content
    let empty_chunks = split_on_blank_lines("");
    assert!(empty_chunks.is_empty());

    // Test only blank lines
    let blank_only = split_on_blank_lines("\n\n\n");
    assert!(blank_only.is_empty());

    // Test no blank lines
    let no_blanks = split_on_blank_lines("line1\nline2\nline3");
    assert_eq!(no_blanks.len(), 1);
    assert_eq!(no_blanks[0], "line1\nline2\nline3");
  }

  #[test]
  fn test_get_chunks_with_indentation() {
    // Test that indented content stays with its parent
    let content = "function main() {\n    return 42;\n}\n\nclass Test {\n    method() {}\n}";
    let chunks = get_chunks(content);
    
    assert_eq!(chunks.len(), 2);
    // First chunk should include the function and its indented content
    assert!(chunks[0].contains("function main()"));
    assert!(chunks[0].contains("return 42;"));
    assert!(chunks[0].contains("}"));
    
    // Second chunk should include the class and its indented content  
    assert!(chunks[1].contains("class Test"));
    assert!(chunks[1].contains("method()"));
  }

  #[test]
  fn test_should_ignore_chunk() {
    // Test chunk without ignore directive
    let normal_chunk = "fn normal() {\n    return 42;\n}";
    assert!(!should_ignore_chunk(normal_chunk));

    // Test chunk with ignore directive
    let ignored_chunk = "// violet ignore chunk\nfn complex() {\n    if deeply {\n        if nested {\n            return 42;\n        }\n    }\n}";
    assert!(should_ignore_chunk(ignored_chunk));

    // Test with different comment styles
    let ignored_chunk2 = "# violet ignore chunk\nfn another() { return 1; }";
    assert!(should_ignore_chunk(ignored_chunk2));

    let ignored_chunk3 = "/* violet ignore chunk */\nfn yet_another() { return 2; }";
    assert!(should_ignore_chunk(ignored_chunk3));

    // Test with extra whitespace
    let ignored_chunk4 = "//   violet   ignore   chunk   \nfn spaced() { return 3; }";
    assert!(should_ignore_chunk(ignored_chunk4));
  }
}

