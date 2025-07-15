use clap::Parser;
use colored::*;
use std::path::PathBuf;
use std::process;
use violet::simplicity::{analyze_file, FileAnalysis};

const TOTAL_WIDTH: usize = 80;
const PADDING: usize = 2;

/// Violet - Simple Code Complexity Analysis
#[derive(Parser)]
#[command(name = "violet")]
#[command(about = "Language-agnostic code complexity analysis using information theory")]
#[command(version)]
struct Cli {
  /// Files or directories to analyze
  #[arg(value_name = "PATH")]
  paths: Vec<PathBuf>,

  /// Complexity threshold for warnings (default: 10.0)
  #[arg(short, long, default_value = "10.0")]
  threshold: f64,

  /// Show detailed breakdown for each chunk
  #[arg(short, long)]
  verbose: bool,

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

  println!("{}", "🎨 Violet - Simple Code Complexity Analysis".purple().bold());
  println!("{}", "Information-theoretic complexity scoring".italic());
  println!();

  // Print table header with proper alignment
  let avg_width = "AVG".len();
  let file_width = TOTAL_WIDTH - avg_width - PADDING;
  
  println!("{:<width$} {}", "FILE", "AVG", width = file_width);
  println!("{}", "─".repeat(TOTAL_WIDTH));

  let mut total_files = 0;
  let mut violations = 0;

  for path in &cli.paths {
    if path.is_file() {
      match analyze_file(path) {
        Ok(analysis) => {
          total_files += 1;
          if process_file_analysis(&analysis, &cli) {
            violations += 1;
          }
        }
        Err(e) => {
          eprintln!("Error analyzing {}: {}", path.display(), e);
        }
      }
    } else if path.is_dir() {
      // Simple directory traversal for common source files
      if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
          let file_path = entry.path();
          if is_source_file(&file_path) {
            match analyze_file(&file_path) {
              Ok(analysis) => {
                total_files += 1;
                if process_file_analysis(&analysis, &cli) {
                  violations += 1;
                }
              }
              Err(e) => {
                eprintln!("Error analyzing {}: {}", file_path.display(), e);
              }
            }
          }
        }
      }
    } else {
      eprintln!("Warning: {} is not a file or directory", path.display());
    }
  }

  println!();
  println!("📊 Summary: {} files analyzed", total_files);
  if violations > 0 {
    println!("⚠️  {} files exceed complexity threshold of {}", violations, cli.threshold);
    process::exit(1);
  } else {
    println!("✅ All files within complexity threshold");
  }
}

fn process_file_analysis(analysis: &FileAnalysis, cli: &Cli) -> bool {
  if analysis.ignored {
    if !cli.quiet {
      print_aligned_row(&analysis.file_path.display().to_string(), "(ignored)", false, true);
    }
    return false;
  }

  // Check if file has any high-complexity chunks
  let high_complexity_chunks: Vec<_> = analysis.chunk_scores.iter()
    .filter(|chunk| chunk.score > 15.0)
    .collect();

  // Only show files that have high-complexity chunks
  if high_complexity_chunks.is_empty() {
    return false;
  }

  let exceeds_threshold = analysis.total_score > cli.threshold;

  // Print file row with color-coded score
  let score_str = format!("{:.1}", analysis.total_score);
  print_aligned_row(&analysis.file_path.display().to_string(), &score_str, exceeds_threshold, true);

  // Always show high-complexity chunks (> 15.0) as nested entries
  for chunk in high_complexity_chunks {
    let chunk_display = format!("- lines {}-{}", chunk.start_line, chunk.end_line);
    let score_str = format!("{:.1}", chunk.score);
    print_aligned_row(&chunk_display, &score_str, true, false); // chunks are always red since > 15.0
    
    // Show complexity breakdown - each component on its own line
    let b = &chunk.breakdown;
    let cognitive_load_factor = 2.0;
    
    // Apply the same logarithmic scaling to components as used in final score
    let depth_scaled = (1.0 + b.depth_score).ln() * cognitive_load_factor;
    let verbosity_scaled = (1.0 + b.verbosity_score).ln() * cognitive_load_factor;
    let syntactic_scaled = (1.0 + b.syntactic_score).ln() * cognitive_load_factor;
    
    println!("      depth: {:.1} ({:.0}%)", depth_scaled, b.depth_percent);
    println!("      verbosity: {:.1} ({:.0}%)", verbosity_scaled, b.verbosity_percent);
    println!("      syntactics: {:.1} ({:.0}%)", syntactic_scaled, b.syntactic_percent);
  }

  exceeds_threshold
}

fn print_aligned_row(file_or_chunk: &str, score_text: &str, is_error: bool, is_file: bool) {
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
  
  // Print with exact calculated widths using appropriate padding
  if is_file {
    // For files, pad with periods
    let padding_needed = file_column_width - formatted_file.len();
    let dots = ".".repeat(padding_needed);
    println!("{}{} {}", formatted_file, dots, colored_score);
  } else {
    // For chunks, use normal space padding
    println!("{:<width$} {}", formatted_file, colored_score, width = file_column_width);
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

fn is_source_file(path: &std::path::Path) -> bool {
  if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
    matches!(
      ext,
      "rs"
        | "js"
        | "ts"
        | "py"
        | "go"
        | "java"
        | "cpp"
        | "c"
        | "h"
        | "hpp"
        | "php"
        | "rb"
        | "sh"
        | "bash"
    )
  } else {
    false
  }
}
