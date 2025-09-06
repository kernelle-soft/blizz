//! Insights REST Server
//!
//! HTTP REST API server for the insights knowledge management system.
//! Provides RESTful endpoints for insight management, search, and administration.

use anyhow::Result;
use clap::Parser;
use std::net::SocketAddr;
use tracing::Level;

use thoughts::server::startup::start_server;

#[derive(Parser)]
#[command(name = "thoughts_server")]
#[command(about = "Thoughts REST API Server")]
#[command(version = concat!(env!("CARGO_PKG_VERSION"), ", courtesy of Blizz and Kernelle Software"))]
struct Args {
  /// Server bind address  
  #[arg(long, default_value = "127.0.0.1:3000")]
  bind: SocketAddr,

  /// Enable verbose logging
  #[arg(short, long)]
  verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
  let args = Args::parse();

  // Initialize logging
  if args.verbose {
    tracing_subscriber::fmt::init();
  } else {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
  }

  bentley::info!(&format!("Starting Insights REST Server v{}", env!("CARGO_PKG_VERSION")));
  bentley::info!(&format!("Binding to address: {}", args.bind));

  // Start the server
  start_server(args.bind).await?;

  Ok(())
}
