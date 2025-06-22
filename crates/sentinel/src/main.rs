use anyhow::Result;
use clap::{Parser, Subcommand};
use sentinel::{services, Sentinel};

#[derive(Parser)]
#[command(name = "sentinel")]
#[command(about = "Secure credential storage for Kernelle tools - the watchful guardian of secrets")]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  /// Setup credentials for a service
  Setup {
    /// Service to setup (github, gitlab, jira, notion)
    service: String,
    /// Force reconfiguration of existing credentials
    #[arg(long)]
    force: bool,
  },
  /// List configured services
  List,
  /// Verify credentials for a service
  Verify {
    /// Service to verify (github, gitlab, jira, notion)
    service: String,
  },
  /// Delete credentials for a service
  Delete {
    /// Service to delete credentials for
    service: String,
    /// Specific credential key to delete (optional - deletes all if not specified)
    key: Option<String>,
    /// Skip confirmation prompt
    #[arg(long)]
    force: bool,
  },
  /// Get credentials for a service (for debugging - outputs to stdout)
  Get {
    /// Service to get credentials for
    service: String,
    /// Specific credential key to get
    key: String,
  },
}

#[tokio::main]
async fn main() -> Result<()> {
  bentley::spotlight("Sentinel - The Watchful Guardian of Secrets");

  let cli = Cli::parse();
  let sentinel = Sentinel::new();

  match cli.command {
    Commands::Setup { service, force } => {
      handle_setup(&sentinel, &service, force).await?;
    }
    Commands::List => {
      handle_list(&sentinel).await?;
    }
    Commands::Verify { service } => {
      handle_verify(&sentinel, &service).await?;
    }
    Commands::Delete { service, key, force } => {
      handle_delete(&sentinel, &service, key, force).await?;
    }
    Commands::Get { service, key } => {
      handle_get(&sentinel, &service, &key).await?;
    }
  }

  Ok(())
}

async fn handle_setup(sentinel: &Sentinel, service_name: &str, force: bool) -> Result<()> {
  let service_config = match service_name.to_lowercase().as_str() {
    "github" => services::github(),
    "gitlab" => services::gitlab(),
    "jira" => services::jira(),
    "notion" => services::notion(),
    _ => {
      bentley::error(&format!(
        "Unsupported service: {}. Supported services: github, gitlab, jira, notion",
        service_name
      ));
      return Ok(());
    }
  };

  // Check if credentials already exist
  let missing = sentinel.verify_service_credentials(&service_config)?;

  if missing.is_empty() && !force {
    bentley::success(&format!(
      "All credentials for {} are already configured!",
      service_config.name
    ));
    bentley::info("Use --force to reconfigure existing credentials");
    return Ok(());
  }

  bentley::announce(&format!("Setting up credentials for {}", service_config.name));
  bentley::info(&service_config.description);

  for cred_spec in &service_config.required_credentials {
    if force || missing.contains(&cred_spec.key) {
      bentley::info(&format!("\n📝 Setting up: {}", cred_spec.description));
      
      if let Some(example) = &cred_spec.example {
        bentley::info(&format!("   Example format: {}", example));
      }

      let prompt = format!("Enter {}: ", cred_spec.key);
      let value = rpassword::prompt_password(prompt)?;

      if value.trim().is_empty() {
        bentley::warn(&format!("Skipping empty {} - you can set it up later", cred_spec.key));
        continue;
      }

      sentinel.store_credential(&service_config.name, &cred_spec.key, value.trim())?;
    }
  }

  bentley::flourish(&format!("Credentials setup complete for {}!", service_config.name));
  Ok(())
}

async fn handle_list(sentinel: &Sentinel) -> Result<()> {
  bentley::announce("Configured Services");

  let services = ["github", "gitlab", "jira", "notion"];
  let mut configured_count = 0;

  for service_name in &services {
    let service_config = match *service_name {
      "github" => services::github(),
      "gitlab" => services::gitlab(),
      "jira" => services::jira(),
      "notion" => services::notion(),
      _ => continue,
    };

    let missing = sentinel.verify_service_credentials(&service_config)?;
    let configured = service_config.required_credentials.len() - missing.len();
    let total = service_config.required_credentials.len();

    if configured > 0 {
      bentley::success(&format!(
        "{}: {}/{} credentials configured",
        service_config.name, configured, total
      ));
      configured_count += 1;
    } else {
      bentley::info(&format!("{}: Not configured", service_config.name));
    }
  }

  if configured_count == 0 {
    bentley::info("\nNo services configured yet. Use 'sentinel setup <service>' to get started!");
  }

  Ok(())
}

async fn handle_verify(sentinel: &Sentinel, service_name: &str) -> Result<()> {
  let service_config = match service_name.to_lowercase().as_str() {
    "github" => services::github(),
    "gitlab" => services::gitlab(),
    "jira" => services::jira(),
    "notion" => services::notion(),
    _ => {
      bentley::error(&format!("Unsupported service: {}", service_name));
      return Ok(());
    }
  };

  bentley::info(&format!("Verifying credentials for {}...", service_config.name));

  let missing = sentinel.verify_service_credentials(&service_config)?;

  if missing.is_empty() {
    bentley::success(&format!("✅ All required credentials configured for {}", service_config.name));
  } else {
    bentley::warn(&format!("❌ Missing credentials for {}: {}", service_config.name, missing.join(", ")));
    bentley::info(&format!("Run 'sentinel setup {}' to configure missing credentials", service_name));
  }

  Ok(())
}

async fn handle_delete(
  sentinel: &Sentinel,
  service_name: &str,
  key: Option<String>,
  force: bool,
) -> Result<()> {
  let service_config = match service_name.to_lowercase().as_str() {
    "github" => services::github(),
    "gitlab" => services::gitlab(),
    "jira" => services::jira(),
    "notion" => services::notion(),
    _ => {
      bentley::error(&format!("Unsupported service: {}", service_name));
      return Ok(());
    }
  };

  if let Some(key) = key {
    // Delete specific credential
    if !force {
      bentley::warn(&format!("This will delete the '{}' credential for {}", key, service_name));
      let confirm = rpassword::prompt_password("Type 'yes' to confirm: ")?;
      if confirm.trim().to_lowercase() != "yes" {
        bentley::info("Cancelled");
        return Ok(());
      }
    }

    sentinel.delete_credential(&service_config.name, &key)?;
  } else {
    // Delete all credentials for service
    if !force {
      bentley::warn(&format!("This will delete ALL credentials for {}", service_name));
      let confirm = rpassword::prompt_password("Type 'yes' to confirm: ")?;
      if confirm.trim().to_lowercase() != "yes" {
        bentley::info("Cancelled");
        return Ok(());
      }
    }

    for cred_spec in &service_config.required_credentials {
      if sentinel.get_credential(&service_config.name, &cred_spec.key).is_ok() {
        sentinel.delete_credential(&service_config.name, &cred_spec.key)?;
      }
    }
    bentley::success(&format!("All credentials deleted for {}", service_name));
  }

  Ok(())
}

async fn handle_get(sentinel: &Sentinel, service_name: &str, key: &str) -> Result<()> {
  match sentinel.get_credential(service_name, key) {
    Ok(value) => {
      // For debugging purposes - output to stdout
      println!("{}", value);
    }
    Err(_) => {
      bentley::error(&format!("Credential not found: {}/{}", service_name, key));
      std::process::exit(1);
    }
  }
  Ok(())
} 