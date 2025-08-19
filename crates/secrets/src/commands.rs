use anyhow::Result;
use crate::Secrets;
use std::path::PathBuf;

use std::io::Write;
use std::path::Path;
use crate::keeper_client;

pub async fn store(
  _secrets: &Secrets,
  group: &str,
  name: &str,
  value: Option<String>,
  force: bool,
) -> Result<()> {
  let secret_value = if let Some(val) = value {
    val
  } else {
    let prompt = format!("Enter value for {group}/{name}: ");
    crate::encryption::EncryptionManager::prompt_for_password(&prompt)?
  };

  if secret_value.trim().is_empty() {
    bentley::error("Cannot store empty secret value");
    return Ok(());
  }

  // Get master password once
  let master_password = get_master_password(_secrets).await?;

  // Load existing credentials or create new ones
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  let mut credentials_path = base_path;
  credentials_path.push("persistent");
  credentials_path.push("keeper");
  credentials_path.push("credentials.enc");

  // Load existing credentials or start with empty
  let mut all_credentials = if credentials_path.exists() {
    use crate::PasswordBasedCredentialStore;
    if let Some(store) = PasswordBasedCredentialStore::load_from_file(&credentials_path)? {
      match store.decrypt_credentials(&master_password) {
        Ok(creds) => creds,
        Err(_) => {
          bentley::error("invalid master password");
          return Ok(());
        }
      }
    } else {
      std::collections::HashMap::new()
    }
  } else {
    std::collections::HashMap::new()
  };

  // Check if secret already exists (now that we have the credentials loaded)
  if !force {
    if let Some(group_secrets) = all_credentials.get(group) {
      if group_secrets.contains_key(name) {
        bentley::warn(&format!("Secret {group}/{name} already exists"));
        bentley::info("Use --force to overwrite existing secret");
        return Ok(());
      }
    }
  }

  // Add/update the secret
  all_credentials
    .entry(group.to_string())
    .or_default()
    .insert(name.to_string(), secret_value.trim().to_string());

  // Save back to file
  use crate::PasswordBasedCredentialStore;
  let store = PasswordBasedCredentialStore::new(&all_credentials, &master_password)?;
  store.save_to_file(&credentials_path)?;

  bentley::success(&format!("Stored secret: {group}/{name}"));
  Ok(())
}

pub async fn read(secrets: &Secrets, group: &str, name: &str) -> Result<()> {
  // Get the credentials file path
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  let mut credentials_path = base_path.clone();
  credentials_path.push("persistent");
  credentials_path.push("keeper");
  credentials_path.push("credentials.enc");

  // Check if credentials file exists
  if !credentials_path.exists() {
    bentley::error(&format!("Secret not found: {group}/{name}"));
    std::process::exit(1);
  }

  // Load the encrypted store from file
  use crate::PasswordBasedCredentialStore;
  let store = match PasswordBasedCredentialStore::load_from_file(&credentials_path)? {
    Some(store) => store,
    None => {
      bentley::warn(&format!("secret not found: {group}/{name}"));
      std::process::exit(1);
    }
  };

  // Get master password using daemon integration
  let master_password = get_master_password(secrets).await?;

  // Decrypt all credentials
  let all_credentials = match store.decrypt_credentials(&master_password) {
    Ok(creds) => creds,
    Err(_) => {
      bentley::error("invalid master password or corrupted data");
      std::process::exit(1);
    }
  };

  // Look for the specific secret
  match all_credentials.get(group).and_then(|group_secrets| group_secrets.get(name)) {
    Some(value) => {
      println!("{value}");
    }
    None => {
      bentley::warn(&format!("secret not found: {group}/{name}"));
      std::process::exit(1);
    }
  }

  Ok(())
}

pub async fn delete(
  secrets: &Secrets,
  group: &str,
  name: Option<String>,
  force: bool,
) -> Result<()> {
  // Get the credentials file path
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  let mut credentials_path = base_path.clone();
  credentials_path.push("persistent");
  credentials_path.push("keeper");
  credentials_path.push("credentials.enc");

  // Check if credentials file exists
  if !credentials_path.exists() {
    bentley::error("No secrets stored yet");
    return Ok(());
  }

  // Get master password using daemon integration
  let master_password = get_master_password(secrets).await?;

  // Load the encrypted store from file
  use crate::PasswordBasedCredentialStore;
  let store = match PasswordBasedCredentialStore::load_from_file(&credentials_path)? {
    Some(store) => store,
    None => {
      bentley::error("No secrets found");
      return Ok(());
    }
  };

  // Decrypt all credentials
  let mut all_credentials = match store.decrypt_credentials(&master_password) {
    Ok(creds) => creds,
    Err(_) => {
      bentley::error("Invalid master password or corrupted data");
      return Ok(());
    }
  };

  if let Some(name) = name {
    // Delete specific secret
    let secret_exists =
      all_credentials.get(group).is_some_and(|group_secrets| group_secrets.contains_key(&name));

    if !secret_exists {
      bentley::error(&format!("Secret not found: {group}/{name}"));
      return Ok(());
    }

    if !force {
      bentley::warn(&format!("This will delete the secret: {group}/{name}"));
      let confirm = crate::encryption::EncryptionManager::prompt_confirmation("Type 'yes' to confirm: ")?;
      if confirm.trim().to_lowercase() != "yes" {
        bentley::info("Cancelled");
        return Ok(());
      }
    }

    // Remove the secret
    if let Some(group_secrets) = all_credentials.get_mut(group) {
      group_secrets.remove(&name);

      // Remove the group entirely if no credentials left
      if group_secrets.is_empty() {
        all_credentials.remove(group);
      }
    }

    // Save updated credentials back to file
    let updated_store = PasswordBasedCredentialStore::new(&all_credentials, &master_password)?;
    updated_store.save_to_file(&credentials_path)?;

    bentley::success(&format!("Deleted secret: {group}/{name}"));
  } else {
    // Delete all secrets for group
    if !all_credentials.contains_key(group) {
      bentley::info(&format!("No secrets found for group: {group}"));
      return Ok(());
    }

    if !force {
      bentley::warn(&format!("This will delete ALL secrets for group: {group}"));
      let confirm = crate::encryption::EncryptionManager::prompt_confirmation("Type 'yes' to confirm: ")?;
      if confirm.trim().to_lowercase() != "yes" {
        bentley::info("Cancelled");
        return Ok(());
      }
    }

    // Count secrets before deletion
    let secret_count = all_credentials.get(group).map_or(0, |secrets| secrets.len());

    // Remove the entire group
    all_credentials.remove(group);

    // Save updated credentials back to file
    let updated_store = PasswordBasedCredentialStore::new(&all_credentials, &master_password)?;
    updated_store.save_to_file(&credentials_path)?;

    bentley::success(&format!("Deleted {secret_count} secrets for group: {group}"));
  }

  Ok(())
}

pub async fn list(
  secrets: &Secrets,
  group_filter: Option<String>,
  show_keys: bool,
  quiet: bool,
) -> Result<()> {
  // Get the credentials file path (same logic as PasswordBasedCryptoManager::new)
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  let mut credentials_path = base_path.clone();
  credentials_path.push("persistent");
  credentials_path.push("keeper");
  credentials_path.push("credentials.enc");

  // Check if credentials file exists
  if !credentials_path.exists() {
    bentley::info("no secrets stored yet");
    return Ok(());
  }

  // Load the encrypted store from file
  use crate::PasswordBasedCredentialStore;
  let store = match PasswordBasedCredentialStore::load_from_file(&credentials_path)? {
    Some(store) => store,
    None => {
      bentley::info("no secrets found");
      return Ok(());
    }
  };

  // Get master password using daemon integration
  let master_password = get_master_password(secrets).await?;

  // Decrypt all credentials
  let all_credentials = match store.decrypt_credentials(&master_password) {
    Ok(creds) => creds,
    Err(_) => {
      bentley::error("invalid master password or corrupted data");
      return Ok(());
    }
  };

  // Display the contents
  if all_credentials.is_empty() {
    bentley::info("vault is empty");
    return Ok(());
  }

  // Filter by group if specified
  let filter_group = group_filter.clone();
  let credentials_to_show = if let Some(filter) = group_filter {
    all_credentials.into_iter().filter(|(group, _)| group == &filter).collect()
  } else {
    all_credentials
  };

  if credentials_to_show.is_empty() {
    if let Some(filter) = filter_group {
      bentley::info(&format!("no secrets found for group: {filter}"));
    } else {
      bentley::info("no secrets found");
    }
    return Ok(());
  }

  // Display format depends on show_keys flag
  if show_keys {
    // Show detailed view with group/key pairs
    for (group, secrets_map) in credentials_to_show {
      bentley::info(&format!("\n{group}/"));
      for key in secrets_map.keys() {
        bentley::info(&format!("   {group}/{key}"));
      }
    }
  } else {
    // Show summary view with just groups and counts
    for (group, secrets_map) in credentials_to_show {
      let count = secrets_map.len();
      let plural = if count == 1 { "secret" } else { "secrets" };
      bentley::info(&format!("{group}: {count} {plural}"));
    }

    if !quiet {
      bentley::info("\nuse --keys to see individual secret names");
    }
  }

  Ok(())
}

pub async fn clear(secrets: &Secrets, force: bool, quiet: bool) -> Result<()> {
  bentley::warn("this will DELETE ALL SECRETS from the vault");
  bentley::warn("this action cannot be undone!");

  // If not forced, ask for confirmation
  if !force {
    bentley::info("type 'yes' to confirm vault clearing:");
    print!("> ");
    std::io::stdout().flush()?;
    let mut confirm = String::new();
    std::io::stdin().read_line(&mut confirm)?;
    if confirm.trim().to_lowercase() != "yes" {
      bentley::info("cancelled - vault contents preserved");
      return Ok(());
    }
  }

  // Get master password using daemon integration for verification
  let master_password = get_master_password(secrets).await?;

  // Try to verify the password by attempting to decrypt existing secrets
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  let mut credentials_path = base_path;
  credentials_path.push("persistent");
  credentials_path.push("keeper");
  credentials_path.push("credentials.enc");

  if credentials_path.exists() {
    use crate::PasswordBasedCredentialStore;
    if let Some(store) = PasswordBasedCredentialStore::load_from_file(&credentials_path)? {
      match store.decrypt_credentials(&master_password) {
        Ok(_) => {
          // Password verified successfully
        }
        Err(_) => {
          bentley::error("invalid master password - vault contents preserved");
          return Ok(());
        }
      }
    }
  }

  bentley::verbose("clearing vault...");

  // Get the credentials file path (same logic as PasswordBasedCryptoManager::new)
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  let mut credentials_path = base_path;
  credentials_path.push("persistent");
  credentials_path.push("keeper");
  credentials_path.push("credentials.enc");

  if credentials_path.exists() {
    // Create empty credentials structure
    let empty_credentials = std::collections::HashMap::new();

    // Create a new encrypted store with empty credentials
    use crate::PasswordBasedCredentialStore;
    let empty_store = PasswordBasedCredentialStore::new(&empty_credentials, &master_password)?;
    empty_store.save_to_file(&credentials_path)?;
  } else {
    bentley::info("no action taken - nothing to clear");
  }

  if !quiet {
    bentley::success("vault cleared");
  }

  Ok(())
}

/// Helper function to get master password, first trying daemon, then fallback to direct prompt
async fn get_master_password(_secrets: &Secrets) -> Result<String> {
  // Check if credentials file exists
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  // Existing vault - try to get password from daemon first
  match keeper_client::get(&base_path).await {
    Ok(password) => {
      bentley::verbose("retrieved password from daemon");
      Ok(password)
    }
    Err(_) => {
      // Daemon not available - start it and try again
      bentley::verbose("daemon not available, starting...");
      start_daemon_if_needed(&base_path).await?;

      // Try daemon again after starting
      match keeper_client::get(&base_path).await {
        Ok(password) => {
          bentley::verbose("retrieved password from daemon after startup");
          Ok(password)
        }
        Err(_) => {
          // Last resort - prompt directly
          bentley::verbose("daemon unavailable, prompting directly");
          let cred_path = base_path.join("persistent").join("keeper").join("credentials.enc");
          let password = crate::encryption::EncryptionManager::get_master_password(&cred_path)?;
          Ok(password)
        }
      }
    }
  }
}

/// Start daemon if not running and wait for it to be ready
async fn start_daemon_if_needed(base_path: &Path) -> Result<()> {
  let socket_path = base_path.join("persistent").join("keeper").join("keeper.sock");
  let pid_file = base_path.join("persistent").join("keeper").join("keeper.pid");
  let keeper_path = base_path.join("persistent").join("keeper");

  // If socket already exists, daemon might be running
  if socket_path.exists() {
    return Ok(());
  }

  bentley::info("starting daemon...");
  keeper_client::start(&socket_path, &pid_file, &keeper_path).await?;

  Ok(())
}pub async fn reset_password(secrets: &Secrets, force: bool) -> Result<()> {
  bentley::verbose("resetting master password...");

  // Get the current master password from the daemon
  let current_password = get_master_password(secrets).await?;

  // Load current credentials store
  let base_path = if let Ok(kernelle_dir) = std::env::var("KERNELLE_DIR") {
    PathBuf::from(kernelle_dir)
  } else {
    dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap()).join(".kernelle")
  };

  let mut credentials_path = base_path;
  credentials_path.push("persistent");
  credentials_path.push("keeper");
  credentials_path.push("credentials.enc");

  if !credentials_path.exists() {
    return Err(anyhow::anyhow!("No vault exists to reset password for"));
  }

  // Load existing credentials with current password
  use crate::PasswordBasedCredentialStore;
  let existing_store = match PasswordBasedCredentialStore::load_from_file(&credentials_path)? {
    Some(store) => store,
    None => {
      return Err(anyhow::anyhow!("No vault exists to reset password for"));
    }
  };

  // Decrypt all credentials with current password
  let credentials = match existing_store.decrypt_credentials(&current_password) {
    Ok(creds) => creds,
    Err(_) => {
      return Err(anyhow::anyhow!("Failed to decrypt vault with current password"));
    }
  };

  if !force {
    eprintln!("This will re-encrypt all secrets with a new master password.");
    eprintln!("You currently have {} secret(s) stored.", credentials.len());
    eprint!("Are you sure you want to continue? (y/N): ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input != "y" && input != "yes" {
      bentley::info("password reset cancelled");
      return Ok(());
    }
  }

  // Prompt for new password
  let new_password = crate::encryption::EncryptionManager::prompt_for_password("Enter new master password:")?;

  if new_password.is_empty() {
    return Err(anyhow::anyhow!("Password cannot be empty"));
  }

  // Confirm new password
  let confirm_password = crate::encryption::EncryptionManager::prompt_for_password("Confirm new master password:")?;

  if new_password != confirm_password {
    return Err(anyhow::anyhow!("Passwords do not match"));
  }

  // Create new encrypted store with new password
  let new_store = PasswordBasedCredentialStore::new(&credentials, &new_password)?;
  new_store.save_to_file(&credentials_path)?;

  bentley::success("master password reset successfully");
  bentley::info("please restart the daemon for the new password to take effect");

  Ok(())
}
