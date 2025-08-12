use anyhow::Result;
use clap::Parser;
use secrets::cli::{handle_command, Cli};

#[tokio::main]
async fn main() -> Result<()> {
  let cli = Cli::parse();
  handle_command(cli.command).await
}
