use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;
use std::process;

use violet::{
    config::Config,
    linter::{Linter, format_violations},
    Result,
};

/// Violet - Code Complexity Artisan
/// 
/// "Every line of code should be a masterpiece"
#[derive(Parser)]
#[command(name = "violet")]
#[command(about = "Code complexity analysis and style enforcement tool")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Files or directories to analyze
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,
    
    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,
    
    /// Output format
    #[arg(short, long, default_value = "pretty")]
    format: OutputFormat,
    
    /// Show only errors (ignore warnings and info)
    #[arg(long)]
    errors_only: bool,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
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
enum OutputFormat {
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

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    let cli = Cli::parse();
    
    if let Err(e) = run(cli) {
        eprintln!("{} {}", "Error:".red().bold(), e);
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
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
        all_violations.retain(|v| matches!(v.severity, violet::Severity::Error));
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
            .filter(|v| matches!(v.severity, violet::Severity::Error))
            .count();
        
        if error_count > 0 {
            process::exit(1);
        }
    }
    
    Ok(())
}
