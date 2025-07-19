//! Scoring and complexity analysis
//!
//! Provides functions for calculating complexity scores and analyzing code chunks.

use regex::Regex;

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
}

pub fn get_indents(line: &str) -> usize {
  get_indents_with_tab_size(line, 2) // 2 spaces = 1 indent level
}

pub fn get_indents_with_tab_size(line: &str, tab_size: usize) -> usize {
  let mut indent_levels = 0;
  let mut space_count = 0;
  let chars: Vec<char> = line.chars().collect();

  for &ch in &chars {
    match ch {
      ' ' => space_count += 1,
      '\t' => {
        indent_levels += space_count / tab_size;
        space_count = 0;
        indent_levels += 1;
      }
      _ => break,
    }
  }

  indent_levels += space_count / tab_size;

  indent_levels
}

pub fn verbosity(line: &str) -> f64 {
  let special_chars = syntactics(line);
  let non_special_chars = (line.trim().len() as f64) - special_chars;
  non_special_chars
}

pub fn syntactics(line: &str) -> f64 {
  let special_regex = Regex::new(r"[^\w\s]").unwrap();
  special_regex.find_iter(line.trim()).count() as f64
}

pub fn depth(line: &str) -> f64 {
  get_indents(line) as f64
}

pub fn punish(score: f64, penalty: f64) -> f64 {
  penalty.powf(score)
}

/// Calculate complexity with component breakdown
pub fn complexity(
  chunk: &str,
  depth_penalty: f64,
  verbosity_penalty: f64,
  syntactic_penalty: f64,
) -> f64 {
  let lines: Vec<&str> = chunk.lines().collect();
  let mut depth_total = 0.0;
  let mut verbosity_total = 0.0;
  let mut syntactic_total = 0.0;

  for line in lines {
    depth_total += punish(depth(line), depth_penalty);
    verbosity_total += punish(verbosity(line), verbosity_penalty);
    syntactic_total += punish(syntactics(line), syntactic_penalty);
  }

  let sum = depth_total + verbosity_total + syntactic_total;

  // Natural log for information-theoretic scaling
  if sum > 0.0 {
    sum.ln()
  } else {
    0.0
  }
}

pub fn chunk_breakdown(
  chunk: &str,
  _depth_penalty: f64,
  _verbosity_penalty: f64,
  _syntactic_penalty: f64,
) -> ComplexityBreakdown {
  let lines: Vec<&str> = chunk.lines().collect();

  let mut total_depth = 0.0;
  let mut total_verbosity = 0.0;
  let mut total_syntactic = 0.0;

  for line in lines {
    total_depth += depth(line);
    total_verbosity += verbosity(line);
    total_syntactic += syntactics(line);
  }

  breakdown(total_depth, total_verbosity, total_syntactic)
}

pub fn breakdown(
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
  fn test_create_breakdown() {
    let bd = breakdown(10.0, 20.0, 30.0);
    assert_eq!(bd.depth_score, 10.0);
    assert_eq!(bd.verbosity_score, 20.0);
    assert_eq!(bd.syntactic_score, 30.0);
    assert!((bd.depth_percent - 16.67).abs() < 0.1);
    assert!((bd.verbosity_percent - 33.33).abs() < 0.1);
    assert!((bd.syntactic_percent - 50.0).abs() < 0.1);

    let zero_breakdown = breakdown(0.0, 0.0, 0.0);
    assert_eq!(zero_breakdown.depth_score, 0.0);
    assert_eq!(zero_breakdown.depth_percent, 0.0);
    assert_eq!(zero_breakdown.verbosity_percent, 0.0);
    assert_eq!(zero_breakdown.syntactic_percent, 0.0);
  }
}
