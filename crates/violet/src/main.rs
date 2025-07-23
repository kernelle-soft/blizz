use clap::Parser;
use colored::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process;
use std::sync::OnceLock;
use violet::config;
use violet::scoring;
use violet::simplicity;

const TOTAL_WIDTH: usize = 80;
const PADDING: usize = 2;

#[derive(Parser)]
#[command(name = "violet")]
#[command(about = "Violet - A Versatile, Intuitive, and Open Legibility Evaluation Tool")]
#[command(version)]
struct Cli {
  #[arg(value_name = "PATH")]
  paths: Vec<PathBuf>,

  /// Only show files with violations
  #[arg(short, long)]
  quiet: bool,
}

/// Map file extensions to human-readable language names
fn extension_to_language(ext: &str) -> &str {
  get_language_map().get(ext).unwrap_or(&ext)
}

fn display_threshold_config(config: &config::VioletConfig) {
  let thresholds = &config.complexity.thresholds.extensions;
  if thresholds.is_empty() {
    display_simple_threshold(config.complexity.thresholds.default);
  } else {
    display_threshold_table(config);
  }
  println!();
}

fn display_simple_threshold(threshold: f64) {
  println!("threshold: {threshold:.2}");
}

fn display_threshold_table(config: &config::VioletConfig) {
  print_table_header();
  print_default_threshold(config.complexity.thresholds.default);
  print_language_thresholds(&config.complexity.thresholds.extensions);
}

// violet ignore chunk - The equal sign is a formatting character that creates artificial complexity
fn print_table_header() {
  println!("language                threshold");
  println!("=================================");
}

fn print_default_threshold(threshold: f64) {
  println!("{:<23} {:>6.2}", "default", threshold);
}

fn print_language_thresholds(thresholds: &std::collections::HashMap<String, f64>) {
  let mut sorted_thresholds: Vec<_> = thresholds.iter().collect();
  sorted_thresholds.sort_by_key(|(ext, _)| ext.as_str());

  for (extension, threshold) in sorted_thresholds {
    let language = extension_to_language(extension);
    println!("{language:<23} {threshold:>6.2}");
  }
}

fn load_config_or_exit() -> config::VioletConfig {
  match config::load_config() {
    Ok(config) => config,
    Err(e) => {
      eprintln!("Error loading configuration: {e}");

      let mut source = e.source();
      while let Some(err) = source {
        eprintln!("  Caused by: {err}");
        source = err.source();
      }

      process::exit(1);
    }
  }
}

fn process_single_file(
  path: &PathBuf,
  config: &config::VioletConfig,
  cli: &Cli,
  total_files: &mut i32,
  violation_output: &mut Vec<String>,
) -> usize {
  if config::should_ignore_file(config, path) {
    return 0;
  }

  match simplicity::analyze_file(path, config) {
    Ok(analysis) => {
      *total_files += 1;
      let threshold = config::get_threshold(config, path);
      if let Some(output) = process_file_analysis(&analysis, config, cli, threshold) {
        let chunk_violations =
          analysis.issues.iter().filter(|region| region.score > threshold).count();
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

fn process_directory(
  path: &PathBuf,
  config: &config::VioletConfig,
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

fn print_results(violation_output: Vec<String>, config: &config::VioletConfig) {
  print_tool_announcement();

  if !violation_output.is_empty() {
    display_threshold_config(config);
    print_violations_table(&violation_output);
  } else {
    print_success_message();
  }
}

fn print_tool_announcement() {
  println!(
    "{}",
    "🎨 Violet - A Versatile, Intuitive, and Open Legibility Evaluation Tool".purple().bold()
  );
  println!();
}

fn print_violations_table(violation_output: &[String]) {
  let score_width = "score".len();
  let chunk_width = TOTAL_WIDTH - score_width - PADDING;

  println!("{:<width$} score", "chunk", width = chunk_width);
  println!("{}", "=".repeat(TOTAL_WIDTH));

  for output in violation_output {
    print!("{output}");
  }
}

fn print_success_message() {
  println!("{} No issues found. What beautiful code you have!", "✨".purple());
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

  print_results(violation_output, &config);

  if violating_chunks > 0 {
    process::exit(1);
  }
}

/// Recursively collect files, respecting ignore patterns
fn collect_files_recursively(dir: &PathBuf, config: &config::VioletConfig) -> Vec<PathBuf> {
  let mut files = Vec::new();

  if let Ok(entries) = std::fs::read_dir(dir) {
    for entry in entries.flatten() {
      let path = entry.path();

      if config::should_ignore_file(config, &path) {
        continue;
      }

      if path.is_file() {
        files.push(path);
      } else if path.is_dir() {
        files.extend(collect_files_recursively(&path, config));
      }
    }
  }

  files
}

fn format_chunk_preview(chunk: &scoring::ComplexityRegion) -> String {
  let mut output = String::new();
  let preview_lines: Vec<&str> = chunk.preview.lines().collect();

  for line in preview_lines.iter() {
    if line.len() > 70 {
      let truncated = format!("{}...", &line[..67]);
      output.push_str(&format!("    {}\n", truncated.dimmed()));
    } else {
      output.push_str(&format!("    {line}\n"));
    }
  }

  output
}

/// Logarithmic scaling for component display
fn scale_component_score(score: f64) -> f64 {
  (1.0_f64 + score).ln()
}

fn report_subscore(name: &str, scaled_score: f64, percent: f64) -> String {
  format!("    {name}: {scaled_score:.2} ({percent:.0}%)\n")
}

fn format_complexity_breakdown(breakdown: &scoring::ComplexityBreakdown) -> String {
  let mut output = String::new();

  let depth_scaled = scale_component_score(breakdown.depth_score);
  let verbosity_scaled = scale_component_score(breakdown.verbosity_score);
  let syntactic_scaled = scale_component_score(breakdown.syntactic_score);

  output.push_str(&report_subscore("depth", depth_scaled, breakdown.depth_percent));
  output.push_str(&report_subscore("verbosity", verbosity_scaled, breakdown.verbosity_percent));
  output.push_str(&report_subscore("syntactics", syntactic_scaled, breakdown.syntactic_percent));

  output
}

fn format_violating_chunk(chunk: &scoring::ComplexityRegion) -> String {
  let mut output = String::new();

  let chunk_display = format!("- lines {}-{}", chunk.start_line, chunk.end_line);
  let score_str = format!("{:.2}", chunk.score);
  output.push_str(&format_aligned_row(&chunk_display, &score_str, true, false));

  output.push_str(&format_chunk_preview(chunk));
  output.push_str(&format_complexity_breakdown(&chunk.breakdown));

  output
}

fn handle_ignored_file(analysis: &simplicity::FileAnalysis, cli: &Cli) -> Option<String> {
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
  analysis: &simplicity::FileAnalysis,
  _config: &config::VioletConfig,
  cli: &Cli,
  threshold: f64,
) -> Option<String> {
  if analysis.ignored {
    return handle_ignored_file(analysis, cli);
  }

  let complex_chunks: Vec<&scoring::ComplexityRegion> =
    analysis.issues.iter().filter(|chunk| chunk.score > threshold).collect();

  if complex_chunks.is_empty() {
    return None;
  }

  let mut output = String::new();
  output.push_str(&format_file_header(&analysis.file_path.display().to_string()));

  for chunk in complex_chunks {
    output.push_str(&format_violating_chunk(chunk));
  }

  Some(output)
}

fn format_file_header(file_path: &str) -> String {
  let formatted_file = format_file_path(file_path, TOTAL_WIDTH - 2);
  format!("{}\n", formatted_file.bold())
}

fn format_aligned_row(
  file_or_chunk: &str,
  score_text: &str,
  is_error: bool,
  is_file: bool,
) -> String {
  let avg_column_width = score_text.len();
  let file_column_width = TOTAL_WIDTH - avg_column_width - PADDING;

  let formatted_file = format_file_path(file_or_chunk, file_column_width);

  let colored_score = if is_error {
    score_text.red().to_string()
  } else if score_text == "(ignored)" {
    score_text.dimmed().to_string()
  } else {
    score_text.green().to_string()
  };

  if is_file {
    let padding_needed = file_column_width - formatted_file.len();
    let dashes = "-".repeat(padding_needed);
    format!("{formatted_file}{dashes} {colored_score}\n")
  } else {
    let padding_needed = file_column_width - formatted_file.len();
    let dots = ".".repeat(padding_needed);
    format!("{formatted_file}{dots} {colored_score}\n")
  }
}

fn format_file_path(path: &str, max_width: usize) -> String {
  if path.len() <= max_width {
    path.to_string()
  } else {
    let truncated_len = max_width - 3;
    format!("...{}", &path[path.len() - truncated_len..])
  }
}

// violet ignore chunk
/// Get the static language mapping table
fn get_language_map() -> &'static HashMap<&'static str, &'static str> {
  static LANGUAGE_MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
  LANGUAGE_MAP.get_or_init(|| {
    let mut map = HashMap::new();

    // JavaScript family
    map.insert(".js", "javascript");
    map.insert(".mjs", "modules javascript");
    map.insert(".cjs", "commonjs javascript");
    map.insert(".jsx", "react javascript");
    map.insert(".ts", "typescript");
    map.insert(".tsx", "react typescript");

    // Python family
    map.insert(".py", "python");
    map.insert(".pyw", "windows python");
    map.insert(".pyc", "compiled python");

    // Systems languages
    map.insert(".rs", "rust");
    map.insert(".go", "go");
    map.insert(".c", "C");
    map.insert(".h", "C headers");
    map.insert(".cpp", "C++");
    map.insert(".cc", "C++");
    map.insert(".cxx", "C++");
    map.insert(".c++", "C++");
    map.insert(".hpp", "C++ headers");
    map.insert(".hxx", "C++ headers");

    // JVM languages
    map.insert(".java", "java");
    map.insert(".kt", "kotlin");
    map.insert(".kts", "kotlin script");
    map.insert(".scala", "scala");
    map.insert(".groovy", "groovy");
    map.insert(".gvy", "groovy");
    map.insert(".gy", "groovy");
    map.insert(".gsh", "groovy shell");

    // Other languages
    map.insert(".cs", "C#");
    map.insert(".php", "php");
    map.insert(".rb", "ruby");
    map.insert(".swift", "swift");
    map.insert(".hs", "haskell");
    map.insert(".ex", "elixir");
    map.insert(".exs", "elixir (script)");
    map.insert(".pl", "perl");
    map.insert(".pm", "perl (module)");
    map.insert(".lua", "lua");
    map.insert(".dart", "dart");
    map.insert(".r", "R");
    map.insert(".R", "R (alt)");
    map.insert(".m", "matlab");
    map.insert(".vb", "visual basic");
    map.insert(".gd", "gdscript");
    map.insert(".asm", "assembly");
    map.insert(".s", "assembly");

    // Shell scripts
    map.insert(".sh", "shell scripts");
    map.insert(".bash", "bash");
    map.insert(".zsh", "zsh");
    map.insert(".fish", "fish");
    map.insert(".ps1", "powershell");

    // Web technologies
    map.insert(".html", "html");
    map.insert(".htm", "html (alt)");
    map.insert(".css", "css");
    map.insert(".scss", "sass (scss)");
    map.insert(".sass", "sass");
    map.insert(".less", "less");
    map.insert(".vue", "vue");

    // Data formats
    map.insert(".json", "json");
    map.insert(".xml", "xml");
    map.insert(".yaml", "yaml");
    map.insert(".yml", "yml");
    map.insert(".toml", "toml");
    map.insert(".sql", "sql");
    map.insert(".md", "markdown");

    // Infrastructure
    map.insert(".dockerfile", "dockerfile");
    map.insert(".tf", "terraform");
    map.insert(".hcl", "hcl");

    map
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::HashMap;
  use std::fs;
  use tempfile::TempDir;
  use violet::scoring::{ComplexityBreakdown, ComplexityRegion};

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
    assert_eq!(result, "...some/file.rs");
    assert_eq!(result.len(), 15);
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
    let config = config::VioletConfig {
      complexity: config::ComplexityConfig {
        thresholds: config::ThresholdConfig { default: 6.0, extensions: HashMap::new() },
        penalties: config::PenaltyConfig::default(),
      },
      ..Default::default()
    };

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
    let config = config::VioletConfig {
      complexity: config::ComplexityConfig {
        thresholds: config::ThresholdConfig { default: 6.0, extensions: HashMap::new() },
        penalties: config::PenaltyConfig::default(),
      },
      ignore_files: vec!["*.ignored".to_string(), "temp*".to_string()],
      ..Default::default()
    };

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
    };

    let preview = format_chunk_preview(&chunk_score);

    assert!(preview.contains("fn simple() {"));
    assert!(preview.contains("return 42;"));
    assert!(preview.contains("}"));
    assert!(preview.contains("    fn simple() {"));
    assert!(preview.contains("        return 42;"));
    assert!(preview.contains("    }"));
  }

  #[test]
  fn test_format_chunk_preview_long_lines() {
    let long_line = "a".repeat(100);
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
    };

    let preview = format_chunk_preview(&chunk_score);

    assert!(preview.contains("..."));
    assert!(preview.len() < 100);
  }

  #[test]
  fn test_format_chunk_preview_many_lines() {
    let many_lines = (1..10).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
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
    };

    let preview = format_chunk_preview(&chunk_score);

    assert!(preview.contains("line 1"));
    assert!(preview.contains("line 5"));
    assert!(preview.contains("line 9"));
    assert!(!preview.contains("..."));
  }

  #[test]
  fn test_scale_component_score() {
    assert_eq!(scale_component_score(0.0), (1.0_f64).ln());
    assert_eq!(scale_component_score(1.0), (2.0_f64).ln());
    assert_eq!(scale_component_score(10.0), (11.0_f64).ln());

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
    assert!(result.contains("33%"));
    assert!(result.contains("("));
    assert!(result.contains(")"));
  }

  #[test]
  fn test_format_file_header_line_ending() {
    let file_path = "src/main.rs";
    let header = format_file_header(file_path);

    assert!(header.contains("src/main.rs"));
    assert!(header.ends_with('\n'));
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
    };

    let formatted = format_violating_chunk(&chunk_score);

    assert!(formatted.contains("8.5"));

    assert!(formatted.contains("10") || formatted.contains("15"));

    assert!(formatted.contains("fn complex()"));

    assert!(formatted.contains("Depth") || formatted.contains("depth"));
  }

  #[test]
  fn test_format_file_path_truncation() {
    let normal_path = "src/main.rs";
    let formatted_normal = format_file_path(normal_path, 50);
    assert_eq!(formatted_normal, normal_path);

    let long_path = "very/long/path/to/some/deeply/nested/file.rs";
    let formatted_long = format_file_path(long_path, 20);

    assert!(formatted_long.len() <= 20);
    assert!(formatted_long.contains("...") || formatted_long.contains("file.rs"));
  }

  #[test]
  fn test_collect_files_recursively_depth() {
    let temp_dir = TempDir::new().unwrap();
    let config = config::VioletConfig {
      complexity: config::ComplexityConfig {
        thresholds: config::ThresholdConfig { default: 6.0, extensions: HashMap::new() },
        penalties: config::PenaltyConfig::default(),
      },
      ..Default::default()
    };

    let level1 = temp_dir.path().join("level1");
    fs::create_dir(&level1).unwrap();
    let level2 = level1.join("level2");
    fs::create_dir(&level2).unwrap();
    let level3 = level2.join("level3");
    fs::create_dir(&level3).unwrap();

    fs::write(temp_dir.path().join("root.rs"), "root file").unwrap();
    fs::write(level1.join("level1.rs"), "level1 file").unwrap();
    fs::write(level2.join("level2.rs"), "level2 file").unwrap();
    fs::write(level3.join("level3.rs"), "level3 file").unwrap();

    let files = collect_files_recursively(&temp_dir.path().to_path_buf(), &config);

    assert_eq!(files.len(), 4);
    let file_names: Vec<_> =
      files.iter().map(|f| f.file_name().unwrap().to_str().unwrap()).collect();
    assert!(file_names.contains(&"root.rs"));
    assert!(file_names.contains(&"level1.rs"));
    assert!(file_names.contains(&"level2.rs"));
    assert!(file_names.contains(&"level3.rs"));
  }

  #[test]
  fn test_extension_to_language() {
    assert_eq!(extension_to_language(".rs"), "rust");
    assert_eq!(extension_to_language(".js"), "javascript");
    assert_eq!(extension_to_language(".ts"), "typescript");
    assert_eq!(extension_to_language(".py"), "python");
    assert_eq!(extension_to_language(".go"), "go");
    assert_eq!(extension_to_language(".java"), "java");

    assert_eq!(extension_to_language(".cpp"), "C++");
    assert_eq!(extension_to_language(".cc"), "C++");
    assert_eq!(extension_to_language(".cxx"), "C++");

    assert_eq!(extension_to_language(".sh"), "shell scripts");
    assert_eq!(extension_to_language(".bash"), "bash");
    assert_eq!(extension_to_language(".zsh"), "zsh");

    assert_eq!(extension_to_language(".unknown"), ".unknown");
    assert_eq!(extension_to_language(".xyz"), ".xyz");

    assert_eq!(extension_to_language(".R"), "R (alt)");
    assert_eq!(extension_to_language(".r"), "R");

    assert_eq!(extension_to_language(".js"), "javascript");
    assert_eq!(extension_to_language(".jsx"), "react javascript");
    assert_eq!(extension_to_language(".ts"), "typescript");
    assert_eq!(extension_to_language(".tsx"), "react typescript");

    assert_eq!(extension_to_language(".c"), "C");
    assert_eq!(extension_to_language(".h"), "C headers");
    assert_eq!(extension_to_language(".cpp"), "C++");
    assert_eq!(extension_to_language(".hpp"), "C++ headers");

    assert_eq!(extension_to_language(".mjs"), "modules javascript");
    assert_eq!(extension_to_language(".cjs"), "commonjs javascript");

    assert_eq!(extension_to_language(".py"), "python");
    assert_eq!(extension_to_language(".pyw"), "windows python");
    assert_eq!(extension_to_language(".pyc"), "compiled python");
  }
}
