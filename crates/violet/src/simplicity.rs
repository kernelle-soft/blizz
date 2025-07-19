//! Information-theoretic complexity scoring based on indentation, syntax, and verbosity

use regex::Regex;
use std::fs;
use std::path::Path;
use crate::chunking;
use crate::config;
use crate::directives;
use crate::scoring;

#[derive(Debug, Clone)]
pub struct FileAnalysis {
  pub file_path: std::path::PathBuf,
  pub average_score: f64,
  pub issues: Vec<scoring::ComplexityRegion>,
  pub ignored: bool,
}

fn ignored_file_analysis(path: &Path) -> FileAnalysis {
  FileAnalysis {
    file_path: path.to_path_buf(),
    average_score: 0.0,
    issues: vec![],
    ignored: true,
  }
}

/// Calculate complexity components: depth (indentation), verbosity (length), syntax (special chars)
pub fn line_complexity(line: &str) -> (f64, f64, f64) {
  let indents = scoring::get_indents(line).saturating_sub(1); // First level is free
  let special_chars = get_num_specials(line);
  let non_special_chars = (line.trim().len() as f64) - special_chars;

  let verbosity_component = 1.05_f64.powf(non_special_chars);
  let syntactic_component = 1.25_f64.powf(special_chars);
  let depth_component = 2.0_f64.powf(indents as f64);

  (depth_component, verbosity_component, syntactic_component)
}


/// Average complexity across all chunks in file
pub fn average_chunk_complexity(file_content: &str) -> f64 {
  let chunks = chunking::find_chunks(file_content);
  if chunks.is_empty() {
    return 0.0;
  }

  let chunk_scores: Vec<f64> = chunks.iter().map(|chunk| scoring::complexity(&file_content[chunk.0..chunk.1], 2.0, 1.05, 1.25)).collect();
  let average_complexity = chunk_scores.iter().sum::<f64>() / chunks.len() as f64;

  average_complexity
}

/// Count special characters using regex (non-word, non-whitespace chars)
fn get_num_specials(line: &str) -> f64 {
  let special_regex = Regex::new(r"[^\w\s]").unwrap();
  special_regex.find_iter(line.trim()).count() as f64
}

/// Analyze file and identify complexity hotspots
pub fn analyze_file<P: AsRef<Path>>(
  file_path: P,
  config: &config::VioletConfig,
) -> Result<FileAnalysis, Box<dyn std::error::Error>> {
  let path = file_path.as_ref();
  let content = fs::read_to_string(path)?;
  
  let preprocessed = match directives::preprocess_file(&content) {
    Some(processed) => processed,
    None => return Ok(ignored_file_analysis(path)),
  };

  if preprocessed.trim().is_empty() {
    return Ok(empty_file_analysis(path));
  }

  let threshold = config::get_threshold(config, path);
  let chunks = chunking::find_chunks(&preprocessed);
  let lines: Vec<&str> = preprocessed.lines().collect();

  let issues = find_issues(chunks, &lines, threshold, config);
  let file_average_score = average_chunk_complexity(&preprocessed);

  Ok(FileAnalysis { 
    file_path: path.to_path_buf(), 
    average_score: file_average_score, 
    issues, 
    ignored: false 
  })
}

fn empty_file_analysis(path: &Path) -> FileAnalysis {
  FileAnalysis {
    file_path: path.to_path_buf(),
    average_score: 0.0,
    issues: vec![],
    ignored: false,
  }
}

fn find_issues(chunks: Vec<(usize, usize)>, lines: &[&str], threshold: f64, config: &config::VioletConfig) -> Vec<scoring::ComplexityRegion> {

  let mut issues = Vec::new();
  
  for (start, end) in chunks {
    if let Some(region) = analyze_chunk(start, end, &lines, threshold, &config.ignore_patterns) {
      issues.push(region);
    }
  }
  
  issues
}

fn analyze_chunk(start: usize, end: usize, lines: &[&str], threshold: f64, ignore_patterns: &[String]) -> Option<scoring::ComplexityRegion> {
  if end <= start {
    return None;
  }
  
  let chunk_content = lines[start..end].join("\n");
  
  if directives::has_ignored_patterns(&chunk_content, ignore_patterns) {
    return None;
  }

  let score = scoring::complexity(&chunk_content, 2.0, 1.05, 1.25);
  
  if score > threshold {
    let breakdown = scoring::chunk_breakdown(&chunk_content, 2.0, 1.05, 1.25);
    let preview = create_chunk_preview(&lines[start..end]);
    
    Some(scoring::ComplexityRegion {
      start_line: start + 1,
      end_line: end + 1,
      score,
      breakdown,
      preview,
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_get_num_specials() {
    assert_eq!(get_num_specials("hello world"), 0.0);
    assert_eq!(get_num_specials("if (condition) { }"), 4.0);
    assert_eq!(get_num_specials("array[index] = value;"), 4.0);
    assert_eq!(get_num_specials("  special: &str  "), 2.0);
  }

  #[test]
  fn test_file_complexity() {
    let content = "fn one() {\n    return 1;\n}\n\nfn two() {\n    return 2;\n}";
    let score = average_chunk_complexity(content);

    assert!(score > 0.0);
  }

  #[test]
  fn test_complete_pipeline_with_ignores() {
    let content = format!("fn simple() {{\n    return 1;\n}}\n\n# violet ignore {}\nfn complex() {{\n    if deeply {{\n        if nested {{\n            if very {{\n                return 2;\n            }}\n        }}\n    }}\n}}\n# violet ignore {}\n\nfn another_simple() {{\n    return 3;\n}}", "start", "end");

    let preprocessed = directives::preprocess_file(&content).unwrap();

    assert!(preprocessed.contains("fn simple()"));
    assert!(preprocessed.contains("fn another_simple()"));
    assert!(!preprocessed.contains("fn complex()"));

    let chunks = chunking::find_chunks(&preprocessed);
    assert_eq!(chunks.len(), 2);

    let total_score = average_chunk_complexity(&preprocessed);
    assert!(total_score > 0.0);
    assert!(total_score < 1000.0);
  }

  #[test]
  fn test_complexity_comparison() {
    let simple_content = "fn simple() {\n    return 42;\n}";
    let complex_content = "fn complex() {\n    if condition1 {\n        if condition2 {\n            if condition3 {\n                return nested_result();\n            }\n        }\n    }\n}";

    let simple_score = scoring::complexity(simple_content, 2.0, 1.05, 1.25);
    let complex_score = scoring::complexity(complex_content, 2.0, 1.05, 1.25);

    assert!(complex_score > simple_score * 1.5);
  }

  #[test]
  fn test_information_theoretic_scaling() {
    let minimal = "x";
    let short = "fn f() { return 1; }";
    let medium = "fn medium() {\n    if condition {\n        return process(value);\n    }\n    return default;\n}";

    let minimal_score = scoring::complexity(minimal, 2.0, 1.05, 1.25);
    let short_score = scoring::complexity(short, 2.0, 1.05, 1.25);
    let medium_score = scoring::complexity(medium, 2.0, 1.05, 1.25);

    assert!(minimal_score < short_score);
    assert!(short_score < medium_score);
    assert!(medium_score < 100.0);
    assert!(minimal_score > 0.0);
  }

  #[test]
  fn test_calculate_line_complexity() {
    let (depth, verbosity, syntactic) = line_complexity("hello world");
    assert_eq!(depth, 1.0);
    assert!(verbosity > 1.0);
    assert_eq!(syntactic, 1.0);

    let (depth, verbosity, syntactic) = line_complexity("    if (condition) {");
    assert!(depth > 1.0);
    assert!(verbosity > 1.0);
    assert!(syntactic > 1.0);

    let (depth, verbosity, syntactic) = line_complexity("");
    assert_eq!(depth, 1.0);
    assert_eq!(verbosity, 1.0);
    assert_eq!(syntactic, 1.0);
  }

  #[test]
  fn test_chunk_complexity_simple() {
    let chunk = "fn simple() {\n    println!(\"hello\");\n}";
    let score = scoring::complexity(chunk, 2.0, 1.05, 1.25);

    assert!(score > 0.0);
    assert!(score < 10000.0);
  }

  #[test]
  fn test_chunk_complexity_nested() {
    let simple_chunk = "fn simple() {\n    return 42;\n}";
    let nested_chunk = "fn nested() {\n    if condition {\n        if nested {\n            return 42;\n        }\n    }\n}";

    let simple_score = scoring::complexity(simple_chunk, 2.0, 1.05, 1.25);
    let nested_score = scoring::complexity(nested_chunk, 2.0, 1.05, 1.25);

    assert!(nested_score > simple_score);
  }
}
