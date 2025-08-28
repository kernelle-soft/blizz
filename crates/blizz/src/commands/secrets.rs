use anyhow::Result;
use secrets::cli::{handle_command, Commands};

pub type SecretsCommands = Commands;

pub async fn handle_secrets_command(command: SecretsCommands) -> Result<()> {
  // Set quiet mode when called from blizz
  std::env::set_var("SECRETS_QUIET", "1");

  handle_command(command).await
}
