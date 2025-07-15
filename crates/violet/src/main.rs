use clap::Parser;
use colored::*;
use std::path::PathBuf;
use std::process;
use violet::config::VioletConfig;
use violet::simplicity::{analyze_file, FileAnalysis};

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

fn main() {
  let cli = Cli::parse();

  if cli.paths.is_empty() {
    eprintln!("Error: No paths specified");
    process::exit(1);
  }

  // Load configuration
  let config = match VioletConfig::load() {
    Ok(config) => config,
    Err(e) => {
      eprintln!("Error loading configuration: {}", e);
      process::exit(1);
    }
  };

  let mut _total_files = 0;
  let mut violating_chunks = 0;
  let mut violation_output = Vec::new();

  for path in &cli.paths {
    if path.is_file() {
      // Check if file should be ignored
      if config.should_ignore(path) {
        continue;
      }

      match analyze_file(path) {
        Ok(analysis) => {
          _total_files += 1;
          let threshold = config.threshold_for_file(path);
          if let Some(output) = process_file_analysis(&analysis, &config, &cli, threshold) {
            // Count the number of chunks that exceed threshold in this file
            let chunk_violations =
              analysis.chunk_scores.iter().filter(|chunk| chunk.score > threshold).count();
            violating_chunks += chunk_violations;
            violation_output.push(output);
          }
        }
        Err(e) => {
          eprintln!("Error analyzing {}: {}", path.display(), e);
        }
      }
    } else if path.is_dir() {
      // Recursive directory traversal
      let files = collect_files_recursively(path, &config);
      for file_path in files {
        match analyze_file(&file_path) {
          Ok(analysis) => {
            _total_files += 1;
            let threshold = config.threshold_for_file(&file_path);
            if let Some(output) = process_file_analysis(&analysis, &config, &cli, threshold) {
              // Count the number of chunks that exceed threshold in this file
              let chunk_violations =
                analysis.chunk_scores.iter().filter(|chunk| chunk.score > threshold).count();
              violating_chunks += chunk_violations;
              violation_output.push(output);
            }
          }
          Err(e) => {
            eprintln!("Error analyzing {}: {}", file_path.display(), e);
          }
        }
      }
    } else {
      eprintln!("Warning: {} is not a file or directory", path.display());
    }
  }

  // Only print headers and output if there are violations
  if !violation_output.is_empty() {
    println!("{}", "🎨 Violet - Simple Code Complexity Analysis".purple().bold());
    println!("{}", "Information-theoretic complexity scoring".italic());
    println!();

    // Print table header for chunk violations
    let score_width = "SCORE".len();
    let chunk_width = TOTAL_WIDTH - score_width - PADDING;

    println!("{:<width$} {}", "CHUNKS", "SCORE", width = chunk_width);
    println!("{}", "=".repeat(TOTAL_WIDTH));

    // Print all violation output
    for output in violation_output {
      print!("{}", output);
    }
  }

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
      if config.should_ignore(&path) {
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

fn process_file_analysis(
  analysis: &FileAnalysis,
  _config: &VioletConfig,
  cli: &Cli,
  threshold: f64,
) -> Option<String> {
  if analysis.ignored {
    if !cli.quiet {
      let mut output = String::new();
      output.push_str(&format_aligned_row(
        &analysis.file_path.display().to_string(),
        "(ignored)",
        false,
        true,
      ));
      return Some(output);
    }
    return None;
  }

  // Check if file has any chunks exceeding threshold
  let violating_chunks: Vec<_> =
    analysis.chunk_scores.iter().filter(|chunk| chunk.score > threshold).collect();

  // Only show files that have violating chunks
  if violating_chunks.is_empty() {
    return None;
  }

  let mut output = String::new();

  // Show file name without score (since we only care about chunks)
  output.push_str(&format_file_header(&analysis.file_path.display().to_string()));

  // Show violating chunks as nested entries
  for chunk in violating_chunks {
    let chunk_display = format!("- lines {}-{}", chunk.start_line, chunk.end_line);
    let score_str = format!("{:.1}", chunk.score);
    output.push_str(&format_aligned_row(&chunk_display, &score_str, true, false)); // chunks are always red since they exceed threshold

    // Show truncated preview of the chunk (preserve indentation)
    let preview_lines: Vec<&str> = chunk.preview.lines().take(5).collect();
    for line in preview_lines.iter() {
      let truncated =
        if line.len() > 70 { format!("{}...", &line[..67]) } else { line.to_string() };
      output.push_str(&format!("    {}\n", truncated.dimmed()));
    }
    if chunk.preview.lines().count() > 5 {
      output.push_str(&format!("    {}\n", "...".dimmed()));
    }

    // Show complexity breakdown - each component on its own line
    let b = &chunk.breakdown;

    // Apply the same logarithmic scaling to components as used in final score
    let depth_scaled = (1.0_f64 + b.depth_score).ln();
    let verbosity_scaled = (1.0_f64 + b.verbosity_score).ln();
    let syntactic_scaled = (1.0_f64 + b.syntactic_score).ln();

    output.push_str(&format!("    depth: {:.1} ({:.0}%)\n", depth_scaled, b.depth_percent));
    output
      .push_str(&format!("    verbosity: {:.1} ({:.0}%)\n", verbosity_scaled, b.verbosity_percent));
    output.push_str(&format!(
      "    syntactics: {:.1} ({:.0}%)\n",
      syntactic_scaled, b.syntactic_percent
    ));
  }

  Some(output) // Return the collected output
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
    format!("{}{} {}\n", formatted_file, dashes, colored_score)
  } else {
    // For chunks, pad with dots
    let padding_needed = file_column_width - formatted_file.len();
    let dots = ".".repeat(padding_needed);
    format!("{}{} {}\n", formatted_file, dots, colored_score)
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
