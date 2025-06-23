use clap::Parser;
use std::process;
use violet::{Cli, run_cli};

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    let cli = Cli::parse();
    
    if let Err(e) = run_cli(cli) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
