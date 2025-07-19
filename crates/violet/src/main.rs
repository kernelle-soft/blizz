use clap::Parser;
use colored::*;
use std::path::PathBuf;
use std::process;
use violet::config::{get_threshold_for_file, load_config, should_ignore_file, VioletConfig};
use violet::simplicity::{analyze_file, ComplexityRegion, ComplexityBreakdown, FileAnalysis};

const TOTAL_WIDTH: usize = 80;
const PADDING: usize = 2;

/// Violet - Simple Code Complexity Analysis
#[derive(Parser)]
#[command(name = "violet")]
#[command(about = "Violet - A Versatile, Intuitive, and Open Legibility Evaluation Tool")]
#[command(version)]
struct Cli {
  /// Files or directories to analyze
  #[arg(value_name = "PATH")]
  paths: Vec<PathBuf>,

  /// Only show files that exceed the threshold
  #[arg(short, long)]
  quiet: bool,
}

/// Load configuration and exit on error
fn load_config_or_exit() -> VioletConfig {
  match load_config() {
    Ok(config) => config,
    Err(e) => {
      eprintln!("Error loading configuration: {e}");

      // Print the full error chain for more detailed diagnostics
      let mut source = e.source();
      while let Some(err) = source {
        eprintln!("  Caused by: {err}");
        source = err.source();
      }

      process::exit(1);
    }
  }
}

/// Process a single file and return chunk violations count and output
fn process_single_file(
  path: &PathBuf,
  config: &VioletConfig,
  cli: &Cli,
  total_files: &mut i32,
  violation_output: &mut Vec<String>,
) -> usize {
  if should_ignore_file(config, path) {
    return 0;
  }

  match analyze_file(path, config) {
    Ok(analysis) => {
      *total_files += 1;
      let threshold = get_threshold_for_file(config, path);
      if let Some(output) = process_file_analysis(&analysis, config, cli, threshold) {
        let chunk_violations =
          analysis.complexity_regions.iter().filter(|region| region.score > threshold).count();
        violation_output.push(output);
        chunk_violations
      } else {
        0
      }
    }
    Err(e) => {
      eprintln!("Error analyzing {}: {}", path.display(), e);
      0
    }
  }
}

/// Process a directory recursively and return total chunk violations
fn process_directory(
  path: &PathBuf,
  config: &VioletConfig,
  cli: &Cli,
  total_files: &mut i32,
  violation_output: &mut Vec<String>,
) -> usize {
  let files = collect_files_recursively(path, config);
  let mut violations = 0;

  for file_path in files {
    violations += process_single_file(&file_path, config, cli, total_files, violation_output);
  }

  violations
}

/// Print output header and violations
fn print_results(violation_output: Vec<String>) {
  if !violation_output.is_empty() {
    println!(
      "{}",
      "🎨 Violet - A Versatile, Intuitive, and Open Legibility Evaluation Tool".purple().bold()
    );
    println!();

    // Print table header for chunk violations
    let score_width = "SCORE".len();
    let chunk_width = TOTAL_WIDTH - score_width - PADDING;

    println!("{:<width$} SCORE", "CHUNKS", width = chunk_width);
    println!("{}", "=".repeat(TOTAL_WIDTH));

    for output in violation_output {
      print!("{output}");
    }
  } else {
    // All files are clean - print success message
    println!("{} No issues found. What beautiful code you have!", "✅".green());
  }
}

fn main() {
  let cli = Cli::parse();

  if cli.paths.is_empty() {
    eprintln!("Error: No paths specified");
    process::exit(1);
  }

  let config = load_config_or_exit();
  let mut _total_files = 0;
  let mut violating_chunks = 0;
  let mut violation_output = Vec::new();

  for path in &cli.paths {
    if path.is_file() {
      violating_chunks +=
        process_single_file(path, &config, &cli, &mut _total_files, &mut violation_output);
    } else if path.is_dir() {
      violating_chunks +=
        process_directory(path, &config, &cli, &mut _total_files, &mut violation_output);
    } else {
      eprintln!("Warning: {} is not a file or directory", path.display());
    }
  }

  print_results(violation_output);

  if violating_chunks > 0 {
    process::exit(1);
  }
}

/// Recursively collect all files in a directory that should be analyzed
fn collect_files_recursively(dir: &PathBuf, config: &VioletConfig) -> Vec<PathBuf> {
  let mut files = Vec::new();

  if let Ok(entries) = std::fs::read_dir(dir) {
    for entry in entries.flatten() {
      let path = entry.path();

      // Skip if the path should be ignored
      if should_ignore_file(config, &path) {
        continue;
      }

      if path.is_file() {
        files.push(path);
      } else if path.is_dir() {
        // Recursively collect files from subdirectory
        files.extend(collect_files_recursively(&path, config));
      }
    }
  }

  files
}

/// Format preview lines with truncation
fn format_chunk_preview(chunk: &ComplexityRegion) -> String {
  let mut output = String::new();
  let preview_lines: Vec<&str> = chunk.preview.lines().collect(); // Show all lines, not just 5

  for line in preview_lines.iter() {
    if line.len() > 70 {
      let truncated = format!("{}...", &line[..67]);
      output.push_str(&format!("    {}\n", truncated.dimmed()));
    } else {
      output.push_str(&format!("    {line}\n"));
    }
  }

  // No more "..." truncation indicator since we show the full window
  output
}

/// Apply logarithmic scaling to a component score
fn scale_component_score(score: f64) -> f64 {
  (1.0_f64 + score).ln()
}

/// Format a single subscore component (depth, verbosity, or syntactic)
fn report_subscore(name: &str, scaled_score: f64, percent: f64) -> String {
  format!("    {name}: {scaled_score:.1} ({percent:.0}%)\n")
}

/// Format complexity breakdown with percentage scaling
fn format_complexity_breakdown(breakdown: &ComplexityBreakdown) -> String {
  let mut output = String::new();

  let depth_scaled = scale_component_score(breakdown.depth_score);
  let verbosity_scaled = scale_component_score(breakdown.verbosity_score);
  let syntactic_scaled = scale_component_score(breakdown.syntactic_score);

  output.push_str(&report_subscore("depth", depth_scaled, breakdown.depth_percent));
  output.push_str(&report_subscore("verbosity", verbosity_scaled, breakdown.verbosity_percent));
  output.push_str(&report_subscore("syntactics", syntactic_scaled, breakdown.syntactic_percent));

  output
}

/// Format a single violating chunk
fn format_violating_chunk(chunk: &ComplexityRegion) -> String {
  let mut output = String::new();

  let chunk_display = format!("- lines {}-{}", chunk.start_line, chunk.end_line);
  let score_str = format!("{:.1}", chunk.score);
  output.push_str(&format_aligned_row(&chunk_display, &score_str, true, false));

  output.push_str(&format_chunk_preview(chunk));
  output.push_str(&format_complexity_breakdown(&chunk.breakdown));

  output
}

/// Handle ignored file formatting
fn handle_ignored_file(analysis: &FileAnalysis, cli: &Cli) -> Option<String> {
  if !cli.quiet {
    let mut output = String::new();
    output.push_str(&format_aligned_row(
      &analysis.file_path.display().to_string(),
      "(ignored)",
      false,
      true,
    ));
    Some(output)
  } else {
    None
  }
}

fn process_file_analysis(
  analysis: &FileAnalysis,
  _config: &VioletConfig,
  cli: &Cli,
  threshold: f64,
) -> Option<String> {
  if analysis.ignored {
    return handle_ignored_file(analysis, cli);
  }

  // Check if file has any chunks exceeding threshold
  let violating_chunks: Vec<&ComplexityRegion> =
    analysis.complexity_regions.iter().filter(|region| region.score > threshold).collect();

  // Only show files that have violating chunks
  if violating_chunks.is_empty() {
    return None;
  }

  let mut output = String::new();

  // Show file name without score (since we only care about chunks)
  output.push_str(&format_file_header(&analysis.file_path.display().to_string()));

  // Show violating chunks as nested entries
  for chunk in violating_chunks {
    output.push_str(&format_violating_chunk(chunk));
  }

  Some(output)
}

fn format_file_header(file_path: &str) -> String {
  // Format file name without score, using available width
  let formatted_file = format_file_path(file_path, TOTAL_WIDTH - 2);
  format!("{}\n", formatted_file.bold())
}

fn format_aligned_row(
  file_or_chunk: &str,
  score_text: &str,
  is_error: bool,
  is_file: bool,
) -> String {
  // Calculate available width for the file/chunk column
  let avg_column_width = score_text.len();
  let file_column_width = TOTAL_WIDTH - avg_column_width - PADDING;

  // Format the file/chunk text to fit exactly in the available space
  let formatted_file = format_file_path(file_or_chunk, file_column_width);

  // Apply color to score if needed
  let colored_score = if is_error {
    score_text.red().to_string()
  } else if score_text == "(ignored)" {
    score_text.dimmed().to_string()
  } else {
    score_text.green().to_string()
  };

  // Format with exact calculated widths using appropriate padding
  if is_file {
    // For files, pad with dashes
    let padding_needed = file_column_width - formatted_file.len();
    let dashes = "-".repeat(padding_needed);
    format!("{formatted_file}{dashes} {colored_score}\n")
  } else {
    // For chunks, pad with dots
    let padding_needed = file_column_width - formatted_file.len();
    let dots = ".".repeat(padding_needed);
    format!("{formatted_file}{dots} {colored_score}\n")
  }
}

fn format_file_path(path: &str, max_width: usize) -> String {
  if path.len() <= max_width {
    path.to_string()
  } else {
    let truncated_len = max_width - 3; // Reserve 3 chars for "..."
    format!("...{}", &path[path.len() - truncated_len..])
  }
}

// violet ignore chunk
#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::HashMap;
  use std::fs;
  use tempfile::TempDir;
  use violet::simplicity::{ComplexityBreakdown, ComplexityRegion, RegionType};

  #[test]
  fn test_format_file_path_no_truncation() {
    let path = "src/main.rs";
    let result = format_file_path(path, 20);
    assert_eq!(result, "src/main.rs");
  }

  #[test]
  fn test_format_file_path_with_truncation() {
    let path = "very/long/path/to/some/file.rs";
    let result = format_file_path(path, 15);
    assert_eq!(result, "...some/file.rs"); // Corrected expectation
    assert_eq!(result.len(), 15); // Should be exactly max_width
  }

  #[test]
  fn test_format_file_path_exact_length() {
    let path = "exact_length";
    let result = format_file_path(path, 12);
    assert_eq!(result, "exact_length");
  }

  #[test]
  fn test_format_file_header() {
    let result = format_file_header("src/test.rs");
    assert!(result.contains("src/test.rs"));
    assert!(result.ends_with('\n'));
  }

  #[test]
  fn test_format_aligned_row_chunk() {
    let result = format_aligned_row("- lines 10-20", "7.5", true, false);
    assert!(result.contains("- lines 10-20"));
    assert!(result.contains("7.5"));
    assert!(result.contains('.'));
    assert!(result.ends_with('\n'));
  }

  #[test]
  fn test_format_aligned_row_file() {
    let result = format_aligned_row("src/main.rs", "6.2", false, true);
    assert!(result.contains("src/main.rs"));
    assert!(result.contains("6.2"));
    assert!(result.contains('-'));
    assert!(result.ends_with('\n'));
  }

  #[test]
  fn test_format_aligned_row_ignored() {
    let result = format_aligned_row("src/ignored.rs", "(ignored)", false, true);
    assert!(result.contains("src/ignored.rs"));
    assert!(result.contains("(ignored)"));
  }

  #[test]
  fn test_collect_files_recursively_empty_config() {
    let temp_dir = TempDir::new().unwrap();
    let config =
      VioletConfig { thresholds: HashMap::new(), ignore_patterns: vec![], default_threshold: 6.0 };

    // Create test files
    let file1_path = temp_dir.path().join("test1.rs");
    fs::write(&file1_path, "fn main() {}").unwrap();

    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();
    let file2_path = subdir.join("test2.rs");
    fs::write(&file2_path, "fn test() {}").unwrap();

    let files = collect_files_recursively(&temp_dir.path().to_path_buf(), &config);

    assert_eq!(files.len(), 2);
    assert!(files.iter().any(|f| f.file_name().unwrap() == "test1.rs"));
    assert!(files.iter().any(|f| f.file_name().unwrap() == "test2.rs"));
  }

  #[test]
  fn test_collect_files_recursively_with_ignore_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let config = VioletConfig {
      thresholds: HashMap::new(),
      ignore_patterns: vec!["*.ignored".to_string(), "temp*".to_string()],
      default_threshold: 6.0,
    };

    // Create test files
    let included_file = temp_dir.path().join("included.rs");
    fs::write(&included_file, "fn main() {}").unwrap();

    let ignored_file1 = temp_dir.path().join("test.ignored");
    fs::write(&ignored_file1, "should be ignored").unwrap();

    let ignored_file2 = temp_dir.path().join("temp_file.rs");
    fs::write(&ignored_file2, "should be ignored").unwrap();

    let files = collect_files_recursively(&temp_dir.path().to_path_buf(), &config);

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_name().unwrap(), "included.rs");
  }

  #[test]
  fn test_format_chunk_preview_simple() {
    let chunk_score = ComplexityRegion {
      score: 5.0,
      start_line: 1,
      end_line: 3,
      preview: "fn simple() {\n    return 42;\n}".to_string(),
      breakdown: ComplexityBreakdown {
        depth_score: 2.0,
        depth_percent: 40.0,
        verbosity_score: 2.0,
        verbosity_percent: 40.0,
        syntactic_score: 1.0,
        syntactic_percent: 20.0,
      },
      region_type: RegionType::NaturalChunk,
    };

    let preview = format_chunk_preview(&chunk_score);
    
    assert!(preview.contains("fn simple() {"));
    assert!(preview.contains("return 42;"));
    assert!(preview.contains("}"));
    // Should have 4 spaces of indentation for each line
    assert!(preview.contains("    fn simple() {"));
    assert!(preview.contains("        return 42;"));
    assert!(preview.contains("    }"));
  }

  #[test]
  fn test_format_chunk_preview_long_lines() {
    let long_line = "a".repeat(100); // 100 characters
    let chunk_score = ComplexityRegion {
      score: 5.0,
      start_line: 1,
      end_line: 1,
      preview: long_line,
      breakdown: ComplexityBreakdown {
        depth_score: 1.0,
        depth_percent: 100.0,
        verbosity_score: 0.0,
        verbosity_percent: 0.0,
        syntactic_score: 0.0,
        syntactic_percent: 0.0,
      },
      region_type: RegionType::NaturalChunk,
    };

    let preview = format_chunk_preview(&chunk_score);
    
    // Should truncate long lines and add "..."
    assert!(preview.contains("..."));
    assert!(preview.len() < 100); // Should be shorter than original
  }

  #[test]
  fn test_format_chunk_preview_many_lines() {
    let many_lines = (1..10).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
    let chunk_score = ComplexityRegion {
      score: 5.0,
      start_line: 1,
      end_line: 5,
      preview: many_lines,
      breakdown: ComplexityBreakdown {
        depth_score: 2.0,
        depth_percent: 50.0,
        verbosity_score: 2.0,
        verbosity_percent: 50.0,
        syntactic_score: 0.0,
        syntactic_percent: 0.0,
      },
      region_type: RegionType::NaturalChunk,
    };

    let preview = format_chunk_preview(&chunk_score);
    
    // Should now show all lines, not just 5
    assert!(preview.contains("line 1"));
    assert!(preview.contains("line 5"));
    assert!(preview.contains("line 9")); // Should now show beyond 5 lines
    assert!(!preview.contains("...")); // Should not indicate truncation
  }

  #[test]
  fn test_scale_component_score() {
    // Test that scaling is logarithmic
    assert_eq!(scale_component_score(0.0), (1.0_f64).ln());
    assert_eq!(scale_component_score(1.0), (2.0_f64).ln());
    assert_eq!(scale_component_score(10.0), (11.0_f64).ln());
    
    // Test that larger inputs give larger outputs
    let small = scale_component_score(1.0);
    let medium = scale_component_score(10.0);
    let large = scale_component_score(100.0);
    
    assert!(small < medium);
    assert!(medium < large);
    assert!(large.is_finite());
  }

  #[test]
  fn test_report_subscore() {
    let result = report_subscore("Depth", 5.5, 33.3);
    
    assert!(result.contains("Depth"));
    assert!(result.contains("5.5"));
    assert!(result.contains("33%")); // Should round to nearest percent
    assert!(result.contains("(")); // Should have parentheses around percentage
    assert!(result.contains(")")); 
  }

  #[test]
  fn test_format_file_header_line_ending() {
    let file_path = "src/main.rs";
    let header = format_file_header(file_path);
    
    assert!(header.contains("src/main.rs"));
    assert!(header.ends_with('\n')); // Should end with newline
  }

  #[test]
  fn test_format_file_header_long_path() {
    let long_path = "very/long/path/to/some/deeply/nested/file/that/might/exceed/normal/width.rs";
    let header = format_file_header(long_path);
    
    assert!(header.contains("file"));
    assert!(header.contains(".rs"));
    assert!(header.ends_with('\n'));
  }

  #[test]
  fn test_format_violating_chunk() {
    let chunk_score = ComplexityRegion {
      score: 8.5,
      start_line: 10,
      end_line: 15,
      preview: "fn complex() {\n    if deeply {\n        nested();\n    }\n}".to_string(),
      breakdown: ComplexityBreakdown {
        depth_score: 4.0,
        depth_percent: 50.0,
        verbosity_score: 2.0,
        verbosity_percent: 25.0,
        syntactic_score: 2.0,
        syntactic_percent: 25.0,
      },
      region_type: RegionType::DetectedHotspot,
    };

    let formatted = format_violating_chunk(&chunk_score);
    
    // Should contain score
    assert!(formatted.contains("8.5"));
    
    // Should contain line numbers
    assert!(formatted.contains("10") || formatted.contains("15"));
    
    // Should contain preview
    assert!(formatted.contains("fn complex()"));
    
    // Should contain breakdown information
    assert!(formatted.contains("Depth") || formatted.contains("depth"));
  }

  #[test]
  fn test_format_file_path_truncation() {
    // Test normal length path
    let normal_path = "src/main.rs";
    let formatted_normal = format_file_path(normal_path, 50);
    assert_eq!(formatted_normal, normal_path);
    
    // Test path that needs truncation
    let long_path = "very/long/path/to/some/deeply/nested/file.rs";
    let formatted_long = format_file_path(long_path, 20);
    
    // Should be truncated to fit width
    assert!(formatted_long.len() <= 20);
    // Should still show important parts
    assert!(formatted_long.contains("...") || formatted_long.contains("file.rs"));
  }

  #[test]
  fn test_collect_files_recursively_depth() {
    let temp_dir = TempDir::new().unwrap();
    let config = VioletConfig {
      thresholds: HashMap::new(),
      ignore_patterns: vec![],
      default_threshold: 6.0,
    };

    // Create nested directory structure
    let level1 = temp_dir.path().join("level1");
    fs::create_dir(&level1).unwrap();
    let level2 = level1.join("level2");
    fs::create_dir(&level2).unwrap();
    let level3 = level2.join("level3");
    fs::create_dir(&level3).unwrap();

    // Create files at different levels
    fs::write(temp_dir.path().join("root.rs"), "root file").unwrap();
    fs::write(level1.join("level1.rs"), "level1 file").unwrap();
    fs::write(level2.join("level2.rs"), "level2 file").unwrap();
    fs::write(level3.join("level3.rs"), "level3 file").unwrap();

    let files = collect_files_recursively(&temp_dir.path().to_path_buf(), &config);

    assert_eq!(files.len(), 4);
    let file_names: Vec<_> = files.iter().map(|f| f.file_name().unwrap().to_str().unwrap()).collect();
    assert!(file_names.contains(&"root.rs"));
    assert!(file_names.contains(&"level1.rs"));
    assert!(file_names.contains(&"level2.rs"));
    assert!(file_names.contains(&"level3.rs"));
  }
}
