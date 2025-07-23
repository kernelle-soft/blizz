//! Information-theoretic complexity scoring based on indentation, syntax, and verbosity

use crate::chunking;
use crate::config;
use crate::directives;
use crate::scoring;
use std::fs;
use std::path::Path;

// Complexity scoring constants moved to config

#[derive(Debug)]
struct ChunkAnalysisContext<'a> {
  lines: &'a [&'a str],
  threshold: f64,
  ignore_patterns: &'a [String],
  penalties: &'a config::PenaltyConfig,
}

#[derive(Debug, Clone)]
pub struct FileAnalysis {
  pub file_path: std::path::PathBuf,
  pub average_score: f64,
  pub issues: Vec<scoring::ComplexityRegion>,
  pub ignored: bool,
}

fn ignored_file_analysis(path: &Path) -> FileAnalysis {
  FileAnalysis { file_path: path.to_path_buf(), average_score: 0.0, issues: vec![], ignored: true }
}

/// Average complexity across all chunks in file
pub fn average_chunk_complexity(file_content: &str, penalties: &config::PenaltyConfig) -> f64 {
  let chunks = chunking::find_chunks(file_content);
  if chunks.is_empty() {
    return 0.0;
  }

  let chunk_scores = calculate_chunk_scores(file_content, &chunks, penalties);
  chunk_scores.iter().sum::<f64>() / chunks.len() as f64
}

fn calculate_chunk_scores(file_content: &str, chunks: &[(usize, usize)], penalties: &config::PenaltyConfig) -> Vec<f64> {
  let lines: Vec<&str> = file_content.lines().collect();
  chunks
    .iter()
    .map(|chunk| {
      let chunk_content = lines[chunk.0..chunk.1].join("\n");
      scoring::complexity(&chunk_content, penalties.depth, penalties.verbosity, penalties.syntactics)
    })
    .collect()
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
  let file_average_score = average_chunk_complexity(&preprocessed, &config.complexity.penalties);

  Ok(FileAnalysis {
    file_path: path.to_path_buf(),
    average_score: file_average_score,
    issues,
    ignored: false,
  })
}

fn empty_file_analysis(path: &Path) -> FileAnalysis {
  FileAnalysis { file_path: path.to_path_buf(), average_score: 0.0, issues: vec![], ignored: false }
}

fn find_issues(
  chunks: Vec<(usize, usize)>,
  lines: &[&str],
  threshold: f64,
  config: &config::VioletConfig,
) -> Vec<scoring::ComplexityRegion> {
  let context = ChunkAnalysisContext { 
    lines, 
    threshold, 
    ignore_patterns: &config.ignore_patterns,
    penalties: &config.complexity.penalties,
  };

  chunks.into_iter().filter_map(|(start, end)| analyze_chunk(start, end, &context)).collect()
}

fn analyze_chunk(
  start: usize,
  end: usize,
  context: &ChunkAnalysisContext,
) -> Option<scoring::ComplexityRegion> {
  if end <= start {
    return None;
  }

  let chunk_content = context.lines[start..end].join("\n");

  if directives::has_ignored_patterns(&chunk_content, context.ignore_patterns) {
    return None;
  }

  let raw_score =
    scoring::complexity(&chunk_content, context.penalties.depth, context.penalties.verbosity, context.penalties.syntactics);

  // Round to 2 decimal places before threshold comparison to match display precision
  let score = (raw_score * 100.0).round() / 100.0;

  if score > context.threshold {
    Some(create_complexity_region(start, end, score, &chunk_content, &context.lines[start..end], context.penalties))
  } else {
    None
  }
}

fn create_complexity_region(
  start: usize,
  end: usize,
  score: f64,
  chunk_content: &str,
  lines: &[&str],
  penalties: &config::PenaltyConfig,
) -> scoring::ComplexityRegion {
  let breakdown = calculate_chunk_breakdown(chunk_content, penalties);
  let preview = create_chunk_preview(lines);

  build_complexity_region(start, end, score, breakdown, preview)
}

fn calculate_chunk_breakdown(chunk_content: &str, penalties: &config::PenaltyConfig) -> scoring::ComplexityBreakdown {
  scoring::chunk_breakdown(chunk_content, penalties.depth, penalties.verbosity, penalties.syntactics)
}

fn build_complexity_region(
  start: usize,
  end: usize,
  score: f64,
  breakdown: scoring::ComplexityBreakdown,
  preview: String,
) -> scoring::ComplexityRegion {
  scoring::ComplexityRegion { start_line: start + 1, end_line: end + 1, score, breakdown, preview }
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

  fn get_default_penalties() -> config::PenaltyConfig {
    config::PenaltyConfig::default()
  }

  #[test]
  fn test_file_complexity() {
    let content = "fn one() {\n    return 1;\n}\n\nfn two() {\n    return 2;\n}";
    let penalties = get_default_penalties();
    let score = average_chunk_complexity(content, &penalties);

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

    let penalties = get_default_penalties();
    let total_score = average_chunk_complexity(&preprocessed, &penalties);
    assert!(total_score > 0.0);
    assert!(total_score < 1000.0);
  }

  #[test]
  fn test_complexity_comparison() {
    let simple_content = "fn simple() {\n    return 42;\n}";
    let complex_content = "fn complex() {\n    if condition1 {\n        if condition2 {\n            if condition3 {\n                return nested_result();\n            }\n        }\n    }\n}";

    let penalties = get_default_penalties();
    let simple_score =
      scoring::complexity(simple_content, penalties.depth, penalties.verbosity, penalties.syntactics);
    let complex_score =
      scoring::complexity(complex_content, penalties.depth, penalties.verbosity, penalties.syntactics);

    assert!(complex_score > simple_score * 1.5);
  }

  #[test]
  fn test_information_theoretic_scaling() {
    let minimal = "x";
    let short = "fn f() { return 1; }";
    let medium = "fn medium() {\n    if condition {\n        return process(value);\n    }\n    return default;\n}";

    let penalties = get_default_penalties();
    let minimal_score =
      scoring::complexity(minimal, penalties.depth, penalties.verbosity, penalties.syntactics);
    let short_score =
      scoring::complexity(short, penalties.depth, penalties.verbosity, penalties.syntactics);
    let medium_score =
      scoring::complexity(medium, penalties.depth, penalties.verbosity, penalties.syntactics);

    assert!(minimal_score < short_score);
    assert!(short_score < medium_score);
    assert!(medium_score < 100.0);
    assert!(minimal_score > 0.0);
  }

  #[test]
  fn test_chunk_complexity_simple() {
    let chunk = "fn simple() {\n    println!(\"hello\");\n}";
    let penalties = get_default_penalties();
    let score = scoring::complexity(chunk, penalties.depth, penalties.verbosity, penalties.syntactics);

    assert!(score > 0.0);
    assert!(score < 10000.0);
  }

  #[test]
  fn test_chunk_complexity_nested() {
    let simple_chunk = "fn simple() {\n    return 42;\n}";
    let nested_chunk = "fn nested() {\n    if condition {\n        if nested {\n            return 42;\n        }\n    }\n}";

    let penalties = get_default_penalties();
    let simple_score =
      scoring::complexity(simple_chunk, penalties.depth, penalties.verbosity, penalties.syntactics);
    let nested_score =
      scoring::complexity(nested_chunk, penalties.depth, penalties.verbosity, penalties.syntactics);

    assert!(nested_score > simple_score);
  }
}
