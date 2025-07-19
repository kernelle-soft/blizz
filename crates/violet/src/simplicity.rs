//! Information-theoretic complexity scoring based on indentation, syntax, and verbosity

use regex::Regex;
use std::fs;
use std::path::Path;
use crate::config::{VioletConfig, get_threshold_for_file};
use crate::directives;



#[derive(Debug, Clone)]
pub struct FileAnalysis {
  pub file_path: std::path::PathBuf,
  pub total_score: f64,
  pub complexity_regions: Vec<ComplexityRegion>,
  pub ignored: bool,
}

/// Breakdown showing which factors contribute to complexity
#[derive(Debug, Clone)]
pub struct ComplexityBreakdown {
  pub depth_score: f64,
  pub depth_percent: f64,
  pub verbosity_score: f64,
  pub verbosity_percent: f64,
  pub syntactic_score: f64,
  pub syntactic_percent: f64,
}

/// A region of code that exceeds complexity thresholds
#[derive(Debug, Clone)]
pub struct ComplexityRegion {
  pub score: f64,
  pub start_line: usize,
  pub end_line: usize,
  pub preview: String,
  pub breakdown: ComplexityBreakdown,
  pub region_type: RegionType,
}

#[derive(Debug, Clone)]
pub enum RegionType {
  NaturalChunk,
  DetectedHotspot,
}

fn create_ignored_file_analysis(path: &Path) -> FileAnalysis {
  FileAnalysis {
    file_path: path.to_path_buf(),
    total_score: 0.0,
    complexity_regions: vec![],
    ignored: true,
  }
}



/// Strip out violet ignore directives, returning None if entire file should be ignored
pub fn preprocess_file(content: &str) -> Option<String> {
  let lines: Vec<&str> = content.lines().collect();

  if directives::is_ignored_file(&lines) {
    return None;
  }

  let mut result_lines = Vec::new();
  let mut ignore_depth = 0;
  let mut skip_next_line = false;

  for line in lines.iter() {
    if directives::process_line(line, &mut ignore_depth, &mut skip_next_line, &mut result_lines) {
      continue;
    }

    if ignore_depth == 0 {
      result_lines.push(*line);
    }
  }

  Some(result_lines.join("\n"))
}

/// Calculate complexity components: depth (indentation), verbosity (length), syntax (special chars)
fn calculate_line_complexity(line: &str) -> (f64, f64, f64) {
  let indents = get_indents(line).saturating_sub(1); // First level is free
  let special_chars = get_num_specials(line);
  let non_special_chars = (line.trim().len() as f64) - special_chars;

  let verbosity_component = 1.05_f64.powf(non_special_chars);
  let syntactic_component = 1.25_f64.powf(special_chars);
  let depth_component = 2.0_f64.powf(indents as f64);

  (depth_component, verbosity_component, syntactic_component)
}

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

/// Calculate complexity with component breakdown
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
  
  // Natural log for information-theoretic scaling
  let score = if sum > 0.0 { sum.ln() } else { 0.0 };

  let breakdown = create_breakdown(depth_total, verbosity_total, syntactic_total);

  (score, breakdown)
}

/// Simple complexity score (legacy interface)
pub fn chunk_complexity(chunk: &str) -> f64 {
  let (score, _) = chunk_complexity_with_breakdown(chunk);

  (score * 100.0).round() / 100.0
}

/// Average complexity across all chunks in file
pub fn file_complexity(file_content: &str) -> f64 {
  let chunks = get_chunks(file_content);
  if chunks.is_empty() {
    return 0.0;
  }

  let chunk_scores: Vec<f64> = chunks.iter().map(|chunk| chunk_complexity(chunk)).collect();
  let average_complexity = chunk_scores.iter().sum::<f64>() / chunks.len() as f64;

  average_complexity
}

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

fn get_indents(line: &str) -> usize {
  get_indents_with_tab_size(line, 2) // 2 spaces = 1 indent level
}

fn get_indents_with_tab_size(line: &str, spaces_per_indent: usize) -> usize {
  let mut indent_levels = 0;
  let mut space_count = 0;
  let chars: Vec<char> = line.chars().collect();

  for &ch in &chars {
    match ch {
      ' ' => space_count += 1,
      '\t' => {
        // Convert accumulated spaces to indents, then add tab
        indent_levels += space_count / spaces_per_indent;
        space_count = 0;
        indent_levels += 1;
      },
      _ => break, // Stop at first non-whitespace
    }
  }

  indent_levels += space_count / spaces_per_indent;
  
  indent_levels
}

/// Count special characters using regex (non-word, non-whitespace chars)
fn get_num_specials(line: &str) -> f64 {
  let special_regex = Regex::new(r"[^\w\s]").unwrap();
  special_regex.find_iter(line.trim()).count() as f64
}

/// Analyze file and identify complexity hotspots
pub fn analyze_file<P: AsRef<Path>>(
  file_path: P,
  config: &VioletConfig,
) -> Result<FileAnalysis, Box<dyn std::error::Error>> {
  let path = file_path.as_ref();
  let content = fs::read_to_string(path)?;
  
  let preprocessed = match preprocess_file(&content) {
    Some(processed) => processed,
    None => return Ok(create_ignored_file_analysis(path)),
  };

  if preprocessed.trim().is_empty() {
    return Ok(create_empty_file_analysis(path));
  }

  let complexity_regions = find_complexity_violations(&preprocessed, config, path);
  let total_score = file_complexity(&preprocessed);

  Ok(FileAnalysis { 
    file_path: path.to_path_buf(), 
    total_score, 
    complexity_regions, 
    ignored: false 
  })
}

fn create_empty_file_analysis(path: &Path) -> FileAnalysis {
  FileAnalysis {
    file_path: path.to_path_buf(),
    total_score: 0.0,
    complexity_regions: vec![],
    ignored: false,
  }
}

fn find_complexity_violations(content: &str, config: &VioletConfig, path: &Path) -> Vec<ComplexityRegion> {
  let threshold = get_threshold_for_file(config, path);
  let lines: Vec<&str> = content.lines().collect();
  let line_lengths: Vec<f64> = lines.iter().map(|line| line.len() as f64).collect();
  let regions = analyze_file_iterative_fusion(&lines, &line_lengths);
  
  process_regions_for_violations(regions, &lines, threshold, config.get_ignore_patterns())
}

fn process_regions_for_violations(regions: Vec<(usize, usize, f64)>, lines: &[&str], threshold: f64, ignore_patterns: &[String]) -> Vec<ComplexityRegion> {
  let mut complexity_regions = Vec::new();
  
  for (start, end, _) in regions {
    if let Some(region) = analyze_region_if_complex(start, end, lines, threshold, ignore_patterns) {
      complexity_regions.push(region);
    }
  }
  
  complexity_regions
}

fn analyze_region_if_complex(start: usize, end: usize, lines: &[&str], threshold: f64, ignore_patterns: &[String]) -> Option<ComplexityRegion> {
  if end <= start {
    return None;
  }
  
  let chunk_lines = &lines[start..=end.min(lines.len().saturating_sub(1))];
  let chunk_content = chunk_lines.join("\n");
  
  if directives::has_ignored_patterns(&chunk_content, ignore_patterns) {
    return None;
  }

  let score = chunk_complexity(&chunk_content);
  
  if score > threshold {
    let breakdown = calculate_chunk_breakdown(&chunk_content);
    let preview = create_chunk_preview(chunk_lines);
    
    Some(ComplexityRegion {
      start_line: start + 1,
      end_line: end + 1,
      score,
      breakdown,
      preview,
      region_type: RegionType::DetectedHotspot,
    })
  } else {
    None
  }
}

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

/// Iterative fusion algorithm to discover natural code boundaries
fn analyze_file_iterative_fusion(lines: &[&str], line_lengths: &[f64]) -> Vec<(usize, usize, f64)> {
  if lines.is_empty() {
    return vec![];
  }
  
  // Start with blank-line boundaries
  let mut chunks = split_into_text_chunks(lines);
  
  // Iteratively merge compatible chunks until stable
  loop {
    let initial_count = chunks.len();
    chunks = fuse_compatible_chunks(chunks, line_lengths, lines);
    
    if chunks.len() == initial_count {
      break;
    }
  }
  
  chunks.into_iter()
    .map(|(start, end)| (start, end, 0.0))
    .collect()
}

/// Initial chunking based on blank lines
fn split_into_text_chunks(lines: &[&str]) -> Vec<(usize, usize)> {
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
fn fuse_compatible_chunks(chunks: Vec<(usize, usize)>, line_lengths: &[f64], lines: &[&str]) -> Vec<(usize, usize)> {
  if chunks.len() <= 1 {
    return chunks;
  }
  
  let mut fused_chunks = Vec::new();
  let mut current_chunk = chunks[0];
  
  for i in 1..chunks.len() {
    let next_chunk = chunks[i];
    
    if should_fuse_chunks(current_chunk, next_chunk, line_lengths, lines) {
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
fn should_fuse_chunks(chunk1: (usize, usize), chunk2: (usize, usize), line_lengths: &[f64], lines: &[&str]) -> bool {
  if !are_chunks_valid(chunk1, chunk2, lines) {
    return false;
  }
  
  if has_block_entry_exit_pattern(chunk1, chunk2, lines) {
    return true;
  }
  
  has_gradual_length_transition(chunk1, chunk2, line_lengths)
}

fn are_chunks_valid(chunk1: (usize, usize), chunk2: (usize, usize), lines: &[&str]) -> bool {
  chunk1.0 < lines.len() && chunk1.1 > chunk1.0 && 
  chunk2.0 < lines.len() && chunk2.1 > chunk2.0
}

fn has_block_entry_exit_pattern(chunk1: (usize, usize), chunk2: (usize, usize), lines: &[&str]) -> bool {
  let prev_start_indent = get_indents(lines[chunk1.0]) as f64;
  let prev_end_indent = get_indents(lines[chunk1.1.saturating_sub(1)]) as f64;
  
  if prev_end_indent <= prev_start_indent {
    return false;
  }
  
  let next_start_indent = get_indents(lines[chunk2.0]) as f64;
  let next_end_indent = get_indents(lines[chunk2.1.saturating_sub(1)]) as f64;
  
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
    assert_eq!(get_indents("  hello"), 1);
    assert_eq!(get_indents("    world"), 2);
    assert_eq!(get_indents("no_indent"), 0);
    assert_eq!(get_indents("\tindented"), 1);
    assert_eq!(get_indents("\t\tindented"), 2);
    assert_eq!(get_indents("      deep"), 3);
    assert_eq!(get_indents("  \tcombo"), 2);
    assert_eq!(get_indents("\t  partial"), 2);
  }

  #[test]
  fn test_get_num_specials() {
    assert_eq!(get_num_specials("hello world"), 0.0);
    assert_eq!(get_num_specials("if (condition) { }"), 4.0);
    assert_eq!(get_num_specials("array[index] = value;"), 4.0);
    assert_eq!(get_num_specials("  special: &str  "), 2.0);
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
    assert!(score < 10000.0);
  }

  #[test]
  fn test_chunk_complexity_nested() {
    let simple_chunk = "fn simple() {\n    return 42;\n}";
    let nested_chunk = "fn nested() {\n    if condition {\n        if nested {\n            return 42;\n        }\n    }\n}";

    let simple_score = chunk_complexity(simple_chunk);
    let nested_score = chunk_complexity(nested_chunk);

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

    let preprocessed = preprocess_file(&content).unwrap();

    assert!(preprocessed.contains("fn simple()"));
    assert!(preprocessed.contains("fn another_simple()"));
    assert!(!preprocessed.contains("fn complex()"));

    let chunks = get_chunks(&preprocessed);
    assert_eq!(chunks.len(), 2);

    let total_score = file_complexity(&preprocessed);
    assert!(total_score > 0.0);
    assert!(total_score < 1000.0);
  }

  #[test]
  fn test_complete_pipeline_file_ignore() {
    let content = format!("# violet ignore {}\nfn extremely_complex() {{\n    if deeply {{\n        if nested {{\n            if very {{\n                if much {{\n                    return 42;\n                }}\n            }}\n        }}\n    }}\n}}", "file");

    let preprocessed = preprocess_file(&content);
    assert_eq!(preprocessed, None);
  }

  #[test]
  fn test_complexity_comparison() {
    let simple_content = "fn simple() {\n    return 42;\n}";
    let complex_content = "fn complex() {\n    if condition1 {\n        if condition2 {\n            if condition3 {\n                return nested_result();\n            }\n        }\n    }\n}";

    let simple_score = chunk_complexity(simple_content);
    let complex_score = chunk_complexity(complex_content);

    assert!(complex_score > simple_score * 1.5);
  }

  #[test]
  fn test_information_theoretic_scaling() {
    let minimal = "x";
    let short = "fn f() { return 1; }";
    let medium = "fn medium() {\n    if condition {\n        return process(value);\n    }\n    return default;\n}";

    let minimal_score = chunk_complexity(minimal);
    let short_score = chunk_complexity(short);
    let medium_score = chunk_complexity(medium);

    assert!(minimal_score < short_score);
    assert!(short_score < medium_score);
    assert!(medium_score < 100.0);
    assert!(minimal_score > 0.0);
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
    let (depth, verbosity, syntactic) = calculate_line_complexity("hello world");
    assert_eq!(depth, 1.0);
    assert!(verbosity > 1.0);
    assert_eq!(syntactic, 1.0);

    let (depth, verbosity, syntactic) = calculate_line_complexity("    if (condition) {");
    assert!(depth > 1.0);
    assert!(verbosity > 1.0);
    assert!(syntactic > 1.0);

    let (depth, verbosity, syntactic) = calculate_line_complexity("");
    assert_eq!(depth, 1.0);
    assert_eq!(verbosity, 1.0);
    assert_eq!(syntactic, 1.0);
  }

  #[test]
  fn test_create_breakdown() {
    let breakdown = create_breakdown(10.0, 20.0, 30.0);
    assert_eq!(breakdown.depth_score, 10.0);
    assert_eq!(breakdown.verbosity_score, 20.0);
    assert_eq!(breakdown.syntactic_score, 30.0);
    assert!((breakdown.depth_percent - 16.67).abs() < 0.1);
    assert!((breakdown.verbosity_percent - 33.33).abs() < 0.1);
    assert!((breakdown.syntactic_percent - 50.0).abs() < 0.1);

    let zero_breakdown = create_breakdown(0.0, 0.0, 0.0);
    assert_eq!(zero_breakdown.depth_score, 0.0);
    assert_eq!(zero_breakdown.depth_percent, 0.0);
    assert_eq!(zero_breakdown.verbosity_percent, 0.0);
    assert_eq!(zero_breakdown.syntactic_percent, 0.0);
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
