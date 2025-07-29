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

fn calculate_chunk_scores(
  file_content: &str,
  chunks: &[(usize, usize)],
  penalties: &config::PenaltyConfig,
) -> Vec<f64> {
  let lines: Vec<&str> = file_content.lines().collect();
  chunks
    .iter()
    .map(|chunk| {
      let chunk_content = lines[chunk.0..chunk.1].join("\n");
      scoring::complexity(
        &chunk_content,
        penalties.depth,
        penalties.verbosity,
        penalties.syntactics,
      )
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

  let raw_score = scoring::complexity(
    &chunk_content,
    context.penalties.depth,
    context.penalties.verbosity,
    context.penalties.syntactics,
  );

  // Round to 2 decimal places before threshold comparison to match display precision
  let score = (raw_score * 100.0).round() / 100.0;

  if score > context.threshold {
    Some(create_complexity_region(
      start,
      end,
      score,
      &chunk_content,
      &context.lines[start..end],
      context.penalties,
    ))
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

fn calculate_chunk_breakdown(
  chunk_content: &str,
  penalties: &config::PenaltyConfig,
) -> scoring::ComplexityBreakdown {
  scoring::chunk_breakdown(
    chunk_content,
    penalties.depth,
    penalties.verbosity,
    penalties.syntactics,
  )
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
      // Use character-based slicing instead of byte-based to handle Unicode safely
      let truncated = format!("{}...", line.chars().take(MAX_LINE_LENGTH.saturating_sub(3)).collect::<String>());
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
  use std::collections::HashMap;

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
    let simple_score = scoring::complexity(
      simple_content,
      penalties.depth,
      penalties.verbosity,
      penalties.syntactics,
    );
    let complex_score = scoring::complexity(
      complex_content,
      penalties.depth,
      penalties.verbosity,
      penalties.syntactics,
    );

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
    let score =
      scoring::complexity(chunk, penalties.depth, penalties.verbosity, penalties.syntactics);

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

  #[test]
  fn test_penalties_affect_depth_scoring() {
    let nested_code = "fn nested() {\n    if a {\n        if b {\n            if c {\n                return 42;\n            }\n        }\n    }\n}";

    let low_depth_penalty = config::PenaltyConfig { depth: 1.5, verbosity: 1.05, syntactics: 1.15 };

    let high_depth_penalty =
      config::PenaltyConfig { depth: 3.0, verbosity: 1.05, syntactics: 1.15 };

    let low_score = scoring::complexity(
      nested_code,
      low_depth_penalty.depth,
      low_depth_penalty.verbosity,
      low_depth_penalty.syntactics,
    );
    let high_score = scoring::complexity(
      nested_code,
      high_depth_penalty.depth,
      high_depth_penalty.verbosity,
      high_depth_penalty.syntactics,
    );

    assert!(
      high_score > low_score,
      "Higher depth penalty should result in higher complexity score"
    );
  }

  #[test]
  fn test_penalties_affect_verbosity_scoring() {
    let verbose_code = "fn verbose_function_with_very_long_name_and_parameters() {\n    let very_long_variable_name_that_describes_something = 42;\n    println!(\"This is a very long string that adds to verbosity\");\n}";

    let low_verbosity_penalty =
      config::PenaltyConfig { depth: 2.0, verbosity: 1.01, syntactics: 1.15 };

    let high_verbosity_penalty =
      config::PenaltyConfig { depth: 2.0, verbosity: 1.20, syntactics: 1.15 };

    let low_score = scoring::complexity(
      verbose_code,
      low_verbosity_penalty.depth,
      low_verbosity_penalty.verbosity,
      low_verbosity_penalty.syntactics,
    );
    let high_score = scoring::complexity(
      verbose_code,
      high_verbosity_penalty.depth,
      high_verbosity_penalty.verbosity,
      high_verbosity_penalty.syntactics,
    );

    assert!(
      high_score > low_score,
      "Higher verbosity penalty should result in higher complexity score"
    );
  }

  #[test]
  fn test_penalties_affect_syntactics_scoring() {
    let syntactic_code = "fn syntactic() {\n    let result = match value {\n        Some(x) => x.map(|y| y + 1).unwrap_or(0),\n        None => default_value.clone().unwrap(),\n    };\n}";

    let low_syntactics_penalty =
      config::PenaltyConfig { depth: 2.0, verbosity: 1.05, syntactics: 1.05 };

    let high_syntactics_penalty =
      config::PenaltyConfig { depth: 2.0, verbosity: 1.05, syntactics: 1.30 };

    let low_score = scoring::complexity(
      syntactic_code,
      low_syntactics_penalty.depth,
      low_syntactics_penalty.verbosity,
      low_syntactics_penalty.syntactics,
    );
    let high_score = scoring::complexity(
      syntactic_code,
      high_syntactics_penalty.depth,
      high_syntactics_penalty.verbosity,
      high_syntactics_penalty.syntactics,
    );

    assert!(
      high_score > low_score,
      "Higher syntactics penalty should result in higher complexity score"
    );
  }

  #[test]
  fn test_average_chunk_complexity_with_different_penalties() {
    let content = "fn one() {\n    if condition {\n        return complex_operation();\n    }\n}\n\nfn two() {\n    match value {\n        Some(x) => process(x),\n        None => default(),\n    }\n}";

    let default_penalties = get_default_penalties();
    let higher_penalties = config::PenaltyConfig { depth: 3.0, verbosity: 1.10, syntactics: 1.25 };

    let default_score = average_chunk_complexity(content, &default_penalties);
    let higher_score = average_chunk_complexity(content, &higher_penalties);

    assert!(
      higher_score > default_score,
      "Higher penalties should result in higher average complexity"
    );
    assert!(default_score > 0.0);
    assert!(higher_score > 0.0);
  }

  #[test]
  fn test_analyze_file_uses_config_penalties() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let content = "fn test() {\n    if deeply {\n        if nested {\n            if very {\n                return complex();\n            }\n        }\n    }\n}";

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(content.as_bytes()).unwrap();

    let default_config = config::VioletConfig {
      complexity: config::ComplexityConfig {
        thresholds: config::ThresholdConfig { default: 5.0, extensions: HashMap::new() },
        penalties: get_default_penalties(),
      },
      ..Default::default()
    };

    let high_penalty_config = config::VioletConfig {
      complexity: config::ComplexityConfig {
        thresholds: config::ThresholdConfig { default: 5.0, extensions: HashMap::new() },
        penalties: config::PenaltyConfig { depth: 3.0, verbosity: 1.10, syntactics: 1.25 },
      },
      ..Default::default()
    };

    let default_analysis = analyze_file(temp_file.path(), &default_config).unwrap();
    let high_penalty_analysis = analyze_file(temp_file.path(), &high_penalty_config).unwrap();

    // Both should find issues since threshold is low, but high penalty should have higher scores
    assert!(!default_analysis.issues.is_empty());
    assert!(!high_penalty_analysis.issues.is_empty());
    assert!(high_penalty_analysis.average_score > default_analysis.average_score);

    // The complexity region scores should also be higher with higher penalties
    if let (Some(default_issue), Some(high_penalty_issue)) =
      (default_analysis.issues.first(), high_penalty_analysis.issues.first())
    {
      assert!(high_penalty_issue.score > default_issue.score);
    }
  }
}
