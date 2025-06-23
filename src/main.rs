use clap::Parser;
use std::process;
use violet::{
    Cli, OutputFormat, 
    collect_files, process_files, format_violations, get_exit_code,
    config::Config
};

fn main() {
    let cli = Cli::parse();
    
    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = if let Some(config_path) = &cli.config {
        Config::load_from_file(std::path::Path::new(config_path))?
    } else {
        Config::load_from_file(std::path::Path::new("violet.json"))?
    };
    
    // Determine output format
    let format = OutputFormat::from(&cli);
    
    // Collect files to analyze
    let files = collect_files(&cli.paths)?;
    
    if files.is_empty() {
        eprintln!("No supported files found to analyze");
        return Ok(());
    }
    
    // Process files and get violations
    let violations = process_files(&files, &config)?;
    
    // Format and print output
    let output = format_violations(&violations, &format);
    print!("{}", output);
    
    // Exit with appropriate code
    process::exit(get_exit_code(&violations));
} 