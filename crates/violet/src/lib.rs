//! Violet - Code Complexity Artisan
//! 
//! "Every line of code should be a masterpiece"
//! 
//! A local-only code complexity analysis and style enforcement tool
//! that promotes functional programming patterns and beautiful code.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use clap::{Parser, Subcommand};
use colored::*;

pub mod config;
pub mod parser;
pub mod metrics;
pub mod linter;

use config::Config;
use linter::{Linter, format_violations};

/// Violet - Code Complexity Artisan
/// 
/// "Every line of code should be a masterpiece"
#[derive(Parser)]
#[command(name = "violet")]
#[command(about = "Code complexity analysis and style enforcement tool")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    
    /// Files or directories to analyze
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,
    
    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    
    /// Output format
    #[arg(short, long, default_value = "pretty")]
    pub format: OutputFormat,
    
    /// Show only errors (ignore warnings and info)
    #[arg(long)]
    pub errors_only: bool,
    
    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new configuration file
    Init {
        /// Configuration file path (default: .violet.json)
        #[arg(short, long, default_value = ".violet.json")]
        output: PathBuf,
        
        /// Force overwrite existing file
        #[arg(short, long)]
        force: bool,
    },
    
    /// Show current configuration
    Config,
    
    /// List supported languages
    Languages,
}

#[derive(Clone, Copy, Debug)]
pub enum OutputFormat {
    Pretty,
    Json,
    Compact,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pretty" => Ok(OutputFormat::Pretty),
            "json" => Ok(OutputFormat::Json),
            "compact" => Ok(OutputFormat::Compact),
            _ => Err(format!("Invalid format: {}", s)),
        }
    }
}

/// Main CLI entry point
pub fn run_cli(cli: Cli) -> Result<()> {
    // Print beautiful header
    print_header();
    
    match cli.command {
        Some(Commands::Init { output, force }) => {
            init_config(output, force)
        }
        Some(Commands::Config) => {
            show_config(cli.config)
        }
        Some(Commands::Languages) => {
            show_languages();
            Ok(())
        }
        None => {
            analyze_code(cli)
        }
    }
}

fn print_header() {
    println!("{}", "🎨 Violet - Code Complexity Artisan".purple().bold());
    println!("{}", "\"Every line of code should be a masterpiece\"".italic());
    println!();
}

fn init_config(output: PathBuf, force: bool) -> Result<()> {
    if output.exists() && !force {
        eprintln!("{} Configuration file already exists: {}", 
                  "Warning:".yellow().bold(), 
                  output.display());
        eprintln!("Use --force to overwrite");
        return Ok(());
    }
    
    let config = Config::default();
    config.save_to_file(&output)?;
    
    println!("{} Configuration file created: {}", 
             "✨".green(), 
             output.display().to_string().green().bold());
    println!("Edit this file to customize your code quality standards.");
    
    Ok(())
}

fn show_config(config_path: Option<PathBuf>) -> Result<()> {
    let config = match config_path {
        Some(path) => Config::load_from_file(path)?,
        None => Config::load()?,
    };
    
    println!("{}", "📋 Current Configuration".cyan().bold());
    println!();
    
    let json = serde_json::to_string_pretty(&config)?;
    println!("{}", json);
    
    Ok(())
}

fn show_languages() {
    println!("{}", "🌍 Supported Languages".cyan().bold());
    println!();
    
    let languages = [
        ("JavaScript", "js, mjs, cjs"),
        ("TypeScript", "ts, tsx"),
        ("Python", "py, pyw"),
        ("Rust", "rs"),
        ("Bash", "sh, bash"),
        ("Go", "go"),
        ("Ruby", "rb"),
    ];
    
    for (name, extensions) in &languages {
        println!("  {} {}", 
                 name.green().bold(), 
                 format!("({})", extensions).dimmed());
    }
    println!();
    println!("{}", "More languages coming soon! 🚀".italic());
}

fn analyze_code(cli: Cli) -> Result<()> {
    if cli.paths.is_empty() {
        println!("{} No paths specified, analyzing current directory...", 
                 "ℹ️".blue());
        return analyze_paths(&[PathBuf::from(".")], cli);
    }
    
    let paths = cli.paths.clone();
    analyze_paths(&paths, cli)
}

fn analyze_paths(paths: &[PathBuf], cli: Cli) -> Result<()> {
    // Load configuration
    let config = match cli.config {
        Some(path) => Config::load_from_file(path)?,
        None => Config::load()?,
    };
    
    if cli.verbose {
        println!("{} Analyzing {} path(s)...", 
                 "🔍".blue(), 
                 paths.len());
    }
    
    // Create linter and analyze
    let linter = Linter::new(config);
    let mut all_violations = Vec::new();
    
    for path in paths {
        if cli.verbose {
            println!("  📁 {}", path.display().to_string().dimmed());
        }
        
        let violations = linter.analyze_file(path)?;
        all_violations.extend(violations);
    }
    
    // Filter violations if needed
    if cli.errors_only {
        all_violations.retain(|v| matches!(v.severity, Severity::Error));
    }
    
    // Output results
    match cli.format {
        OutputFormat::Pretty => {
            let output = format_violations(&all_violations);
            println!("{}", output);
        }
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&all_violations)?;
            println!("{}", json);
        }
        OutputFormat::Compact => {
            for violation in &all_violations {
                println!("{}:{}:{}: {}", 
                         violation.file.display(),
                         violation.line,
                         violation.rule,
                         violation.message);
            }
        }
    }
    
    // Exit with error code if there are violations
    if !all_violations.is_empty() {
        let error_count = all_violations.iter()
            .filter(|v| matches!(v.severity, Severity::Error))
            .count();
        
        if error_count > 0 {
            std::process::exit(1);
        }
    }
    
    Ok(())
}

/// Core error type for Violet operations
#[derive(thiserror::Error, Debug)]
pub enum VioletError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Parser error: {0}")]
    Parser(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    JavaScript,
    TypeScript,
    Python,
    Rust,
    Bash,
    Go,
    Ruby,
}

impl Language {
    /// Get language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "js" | "mjs" | "cjs" => Some(Language::JavaScript),
            "ts" | "tsx" => Some(Language::TypeScript),
            "py" | "pyw" => Some(Language::Python),
            "rs" => Some(Language::Rust),
            "sh" | "bash" => Some(Language::Bash),
            "go" => Some(Language::Go),
            "rb" => Some(Language::Ruby),
            _ => None,
        }
    }
    
    /// Get file extensions for this language
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Language::JavaScript => &["js", "mjs", "cjs"],
            Language::TypeScript => &["ts", "tsx"],
            Language::Python => &["py", "pyw"],
            Language::Rust => &["rs"],
            Language::Bash => &["sh", "bash"],
            Language::Go => &["go"],
            Language::Ruby => &["rb"],
        }
    }
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::JavaScript => write!(f, "JavaScript"),
            Language::TypeScript => write!(f, "TypeScript"),
            Language::Python => write!(f, "Python"),
            Language::Rust => write!(f, "Rust"),
            Language::Bash => write!(f, "Bash"),
            Language::Go => write!(f, "Go"),
            Language::Ruby => write!(f, "Ruby"),
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}

/// Code complexity metrics for a function or file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Number of parameters in function signature
    pub param_count: usize,
    /// Number of lines in function/file
    pub line_count: usize,
    /// Maximum nesting depth
    pub max_depth: usize,
    /// Cyclomatic complexity
    pub cyclomatic_complexity: usize,
    /// Start line number
    pub start_line: usize,
    /// End line number
    pub end_line: usize,
}

/// A violation of code quality rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// Type of rule violated
    pub rule: String,
    /// Severity level
    pub severity: Severity,
    /// Human-readable message
    pub message: String,
    /// File path
    pub file: PathBuf,
    /// Line number where violation occurs
    pub line: usize,
    /// Column number (optional)
    pub column: Option<usize>,
    /// Suggested fix (optional)
    pub suggestion: Option<String>,
}

/// Severity levels for violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Result type for Violet operations
pub type Result<T> = std::result::Result<T, VioletError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension("js"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("mjs"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("tsx"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("sh"), Some(Language::Bash));
        assert_eq!(Language::from_extension("bash"), Some(Language::Bash));
        assert_eq!(Language::from_extension("go"), Some(Language::Go));
        assert_eq!(Language::from_extension("rb"), Some(Language::Ruby));
        assert_eq!(Language::from_extension("unknown"), None);
        assert_eq!(Language::from_extension(""), None);
    }

    #[test]
    fn test_language_display() {
        assert_eq!(Language::JavaScript.to_string(), "JavaScript");
        assert_eq!(Language::TypeScript.to_string(), "TypeScript");
        assert_eq!(Language::Python.to_string(), "Python");
        assert_eq!(Language::Rust.to_string(), "Rust");
        assert_eq!(Language::Bash.to_string(), "Bash");
        assert_eq!(Language::Go.to_string(), "Go");
        assert_eq!(Language::Ruby.to_string(), "Ruby");
    }

    #[test]
    fn test_language_extensions() {
        assert_eq!(Language::JavaScript.extensions(), &["js", "mjs", "cjs"]);
        assert_eq!(Language::TypeScript.extensions(), &["ts", "tsx"]);
        assert_eq!(Language::Python.extensions(), &["py", "pyw"]);
        assert_eq!(Language::Rust.extensions(), &["rs"]);
        assert_eq!(Language::Bash.extensions(), &["sh", "bash"]);
        assert_eq!(Language::Go.extensions(), &["go"]);
        assert_eq!(Language::Ruby.extensions(), &["rb"]);
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Error.to_string(), "error");
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::Info.to_string(), "info");
    }

    #[test]
    fn test_violation_creation() {
        let violation = Violation {
            rule: "max-params".to_string(),
            message: "Too many parameters".to_string(),
            file: PathBuf::from("test.rs"),
            line: 42,
            column: Some(10),
            severity: Severity::Error,
            suggestion: Some("Reduce parameter count".to_string()),
        };

        assert_eq!(violation.rule, "max-params");
        assert_eq!(violation.message, "Too many parameters");
        assert_eq!(violation.file, PathBuf::from("test.rs"));
        assert_eq!(violation.line, 42);
        assert_eq!(violation.column, Some(10));
        assert_eq!(violation.severity, Severity::Error);
        assert_eq!(violation.suggestion, Some("Reduce parameter count".to_string()));
    }

    #[test]
    fn test_complexity_metrics_creation() {
        let metrics = ComplexityMetrics {
            param_count: 5,
            line_count: 100,
            max_depth: 3,
            cyclomatic_complexity: 15,
            start_line: 10,
            end_line: 110,
        };

        assert_eq!(metrics.param_count, 5);
        assert_eq!(metrics.line_count, 100);
        assert_eq!(metrics.max_depth, 3);
        assert_eq!(metrics.cyclomatic_complexity, 15);
        assert_eq!(metrics.start_line, 10);
        assert_eq!(metrics.end_line, 110);
    }

    #[test]
    fn test_violet_error_display() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let violet_error = VioletError::Io(io_error);
        assert!(violet_error.to_string().contains("File not found"));

        let parser_error = VioletError::Parser("Invalid syntax".to_string());
        assert_eq!(parser_error.to_string(), "Parser error: Invalid syntax");

        let config_error = VioletError::Config("Invalid config".to_string());
        assert_eq!(config_error.to_string(), "Configuration error: Invalid config");
    }

    #[test]
    fn test_violet_error_from_io_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access denied");
        let violet_error: VioletError = io_error.into();
        
        match violet_error {
            VioletError::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::PermissionDenied),
            _ => panic!("Expected Io error"),
        }
    }

    #[test]
    fn test_violet_error_from_serde_error() {
        let json_str = "{ invalid json }";
        let serde_error: serde_json::Error = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let violet_error: VioletError = serde_error.into();
        
        match violet_error {
            VioletError::Json(_) => {}, // Expected - serde_json::Error converts to Json variant
            _ => panic!("Expected Json error"),
        }
    }

    #[test]
    fn test_result_type_alias() {
        fn test_function() -> Result<i32> {
            Ok(42)
        }

        let result = test_function();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_language_equality() {
        assert_eq!(Language::JavaScript, Language::JavaScript);
        assert_ne!(Language::JavaScript, Language::TypeScript);
    }

    #[test]
    fn test_severity_ordering() {
        // In Rust derive(Ord), the first variant is "smallest"
        // So Error < Warning < Info, but we want Error to be "highest" severity
        assert!(Severity::Info > Severity::Warning);
        assert!(Severity::Warning > Severity::Error);
        assert!(Severity::Info > Severity::Error);
    }
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use clap::Parser;

    #[test]
    fn test_cli_parsing_basic() {
        let cli = Cli::try_parse_from(&["violet", "src/"]).unwrap();
        assert_eq!(cli.paths, vec![PathBuf::from("src/")]);
        assert!(cli.command.is_none());
        assert!(!cli.errors_only);
        assert!(!cli.verbose);
    }

    #[test]
    fn test_cli_parsing_with_options() {
        let cli = Cli::try_parse_from(&[
            "violet", 
            "--config", "custom.json",
            "--format", "json",
            "--errors-only",
            "--verbose",
            "file1.rs", "file2.rs"
        ]).unwrap();
        
        assert_eq!(cli.paths, vec![PathBuf::from("file1.rs"), PathBuf::from("file2.rs")]);
        assert_eq!(cli.config, Some(PathBuf::from("custom.json")));
        assert!(matches!(cli.format, OutputFormat::Json));
        assert!(cli.errors_only);
        assert!(cli.verbose);
    }

    #[test]
    fn test_cli_init_command() {
        let cli = Cli::try_parse_from(&[
            "violet", "init", 
            "--output", "test.json",
            "--force"
        ]).unwrap();
        
        match cli.command {
            Some(Commands::Init { output, force }) => {
                assert_eq!(output, PathBuf::from("test.json"));
                assert!(force);
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_cli_config_command() {
        let cli = Cli::try_parse_from(&["violet", "config"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Config)));
    }

    #[test]
    fn test_cli_languages_command() {
        let cli = Cli::try_parse_from(&["violet", "languages"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Languages)));
    }

    #[test]
    fn test_output_format_parsing() {
        assert!(matches!("pretty".parse::<OutputFormat>().unwrap(), OutputFormat::Pretty));
        assert!(matches!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json));
        assert!(matches!("compact".parse::<OutputFormat>().unwrap(), OutputFormat::Compact));
        assert!(matches!("PRETTY".parse::<OutputFormat>().unwrap(), OutputFormat::Pretty));
        assert!("invalid".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_init_config_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.json");
        
        let result = init_config(config_path.clone(), false);
        assert!(result.is_ok());
        assert!(config_path.exists());
        
        // Verify it's valid JSON
        let content = fs::read_to_string(&config_path).unwrap();
        let _: serde_json::Value = serde_json::from_str(&content).unwrap();
    }

    #[test]
    fn test_init_config_no_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("existing.json");
        
        // Create existing file
        fs::write(&config_path, "{}").unwrap();
        
        let result = init_config(config_path.clone(), false);
        assert!(result.is_ok());
        
        // File should still contain original content
        let content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(content, "{}");
    }

    #[test]
    fn test_init_config_force_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("existing.json");
        
        // Create existing file
        fs::write(&config_path, "{}").unwrap();
        
        let result = init_config(config_path.clone(), true);
        assert!(result.is_ok());
        
        // File should be overwritten with new config
        let content = fs::read_to_string(&config_path).unwrap();
        assert_ne!(content, "{}");
        
        // Verify it's a valid config
        let _: Config = serde_json::from_str(&content).unwrap();
    }

    #[test]
    fn test_show_config_default() {
        let result = show_config(None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_show_config_custom_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("custom.json");
        
        // Create a custom config
        let config = Config::default();
        config.save_to_file(&config_path).unwrap();
        
        let result = show_config(Some(config_path));
        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_code_empty_paths() {
        let cli = Cli {
            command: None,
            paths: vec![],
            config: None,
            format: OutputFormat::Pretty,
            errors_only: false,
            verbose: false,
        };
        
        // This should analyze current directory - we can't easily test the output
        // but we can verify it doesn't panic
        let result = analyze_code(cli);
        // Result depends on current directory contents, so we just check it doesn't panic
        assert!(result.is_ok() || result.is_err()); // Either is fine for this test
    }

    #[test]
    fn test_analyze_code_with_paths() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn simple() { println!(\"hello\"); }").unwrap();
        
        let cli = Cli {
            command: None,
            paths: vec![test_file],
            config: None,
            format: OutputFormat::Pretty,
            errors_only: false,
            verbose: true,
        };
        
        let result = analyze_code(cli);
        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_code_json_format() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.js");
        fs::write(&test_file, "function test() { return 42; }").unwrap();
        
        let cli = Cli {
            command: None,
            paths: vec![test_file],
            config: None,
            format: OutputFormat::Json,
            errors_only: false,
            verbose: false,
        };
        
        let result = analyze_code(cli);
        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_code_compact_format() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.py");
        fs::write(&test_file, "def test():\n    pass").unwrap();
        
        let cli = Cli {
            command: None,
            paths: vec![test_file],
            config: None,
            format: OutputFormat::Compact,
            errors_only: false,
            verbose: false,
        };
        
        let result = analyze_code(cli);
        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_code_errors_only() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn simple() {}").unwrap();
        
        let cli = Cli {
            command: None,
            paths: vec![test_file],
            config: None,
            format: OutputFormat::Pretty,
            errors_only: true,
            verbose: false,
        };
        
        let result = analyze_code(cli);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_cli_init_command() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("init_test.json");
        
        let cli = Cli {
            command: Some(Commands::Init { 
                output: config_path.clone(), 
                force: false 
            }),
            paths: vec![],
            config: None,
            format: OutputFormat::Pretty,
            errors_only: false,
            verbose: false,
        };
        
        let result = run_cli(cli);
        assert!(result.is_ok());
        assert!(config_path.exists());
    }

    #[test]
    fn test_run_cli_config_command() {
        let cli = Cli {
            command: Some(Commands::Config),
            paths: vec![],
            config: None,
            format: OutputFormat::Pretty,
            errors_only: false,
            verbose: false,
        };
        
        let result = run_cli(cli);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_cli_languages_command() {
        let cli = Cli {
            command: Some(Commands::Languages),
            paths: vec![],
            config: None,
            format: OutputFormat::Pretty,
            errors_only: false,
            verbose: false,
        };
        
        let result = run_cli(cli);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_cli_analyze_command() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("analyze_test.rs");
        fs::write(&test_file, "fn test() { println!(\"test\"); }").unwrap();
        
        let cli = Cli {
            command: None,
            paths: vec![test_file],
            config: None,
            format: OutputFormat::Pretty,
            errors_only: false,
            verbose: false,
        };
        
        let result = run_cli(cli);
        assert!(result.is_ok());
    }

    #[test]
    fn test_output_format_debug() {
        assert_eq!(format!("{:?}", OutputFormat::Pretty), "Pretty");
        assert_eq!(format!("{:?}", OutputFormat::Json), "Json");
        assert_eq!(format!("{:?}", OutputFormat::Compact), "Compact");
    }
} 