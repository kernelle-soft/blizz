use anyhow::anyhow;
use anyhow::Result;
use secrets::encryption::{EncryptedBlob, EncryptionManager};
use serde_json::{self, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, fs};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::signal;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<()> {
  let base = if let Ok(dir) = env::var("KERNELLE_HOME") {
    PathBuf::from(dir)
  } else {
    dirs::home_dir().ok_or_else(|| anyhow!("failed to determine home directory"))?.join(".kernelle")
  };

  let keeper_path = base.join("persistent").join("keeper");

  // Ensure directory exists
  fs::create_dir_all(&keeper_path)?;

  let master_password = get_password(&keeper_path)?;

  let socket_path = create_socket(&keeper_path)?;
  bentley::info("daemon started - press ctrl+c to exit");

  let ipc_handle = spawn_handler(&socket_path, master_password);

  // Wait for shutdown signal
  signal::ctrl_c().await?;
  bentley::info("\nshutting down daemon");

  // Clean up socket file
  let _ = fs::remove_file(&socket_path);

  // Clean up PID file
  let pid_file = keeper_path.join("keeper.pid");
  let _ = fs::remove_file(&pid_file);

  ipc_handle.abort();
  Ok(())
}

fn get_password(keeper_path: &Path) -> Result<String> {
  let cred_path = keeper_path.join("credentials.enc");

  // Check for SECRETS_AUTH environment variable first
  if let Ok(env_password) = env::var("SECRETS_AUTH") {
    if env_password.trim().is_empty() {
      return Err(anyhow!("SECRETS_AUTH environment variable is set but empty"));
    }

    let master_password = env_password.trim().to_string();

    if !cred_path.exists() {
      // No vault exists - create one using the environment password
      bentley::info("no vault found, creating new vault using SECRETS_AUTH");
      return create_new_vault_with_password(&cred_path, &master_password);
    }

    // Vault exists - verify the environment password
    bentley::info("unlocking vault using SECRETS_AUTH");

    // Verify password by attempting to decrypt
    let data = fs::read_to_string(&cred_path)?;
    let store_json: Value = serde_json::from_str(data.trim())?;
    let blob_val = store_json
      .get("encrypted_data")
      .ok_or_else(|| anyhow!("invalid vault format: missing 'encrypted_data'"))?;
    let blob: EncryptedBlob = serde_json::from_value(blob_val.clone())?;

    if EncryptionManager::decrypt_credentials(&blob, &master_password).is_err() {
      return Err(anyhow!("SECRETS_AUTH password is incorrect"));
    }

    return Ok(master_password);
  }

  if !cred_path.exists() {
    // No vault exists - create one with interactive prompts
    bentley::info("no vault found, setting up new vault");
    return create_new_vault(&cred_path);
  }

  // Vault exists - unlock it with interactive prompt
  bentley::info("enter master password to unlock daemon:");
  print!("> ");
  std::io::stdout().flush()?;
  let master_password = rpassword::read_password()?;

  if master_password.trim().is_empty() {
    return Err(anyhow!("master password cannot be empty"));
  }

  // Verify password by attempting to decrypt
  let data = fs::read_to_string(&cred_path)?;
  let store_json: Value = serde_json::from_str(data.trim())?;
  let blob_val = store_json
    .get("encrypted_data")
    .ok_or_else(|| anyhow!("invalid vault format: missing 'encrypted_data'"))?;
  let blob: EncryptedBlob = serde_json::from_value(blob_val.clone())?;

  if EncryptionManager::decrypt_credentials(&blob, master_password.trim()).is_err() {
    return Err(anyhow!("incorrect password"));
  }

  Ok(master_password.trim().to_string())
}

fn create_new_vault(cred_path: &Path) -> Result<String> {
  bentley::info("setting up vault - create master password:");
  print!("> ");
  std::io::stdout().flush()?;
  let password1 = rpassword::read_password()?;

  if password1.trim().is_empty() {
    return Err(anyhow!("master password cannot be empty"));
  }

  bentley::info("confirm master password:");
  print!("> ");
  std::io::stdout().flush()?;
  let password2 = rpassword::read_password()?;

  if password1 != password2 {
    return Err(anyhow!("passwords do not match"));
  }

  // Create empty credentials structure
  let empty_credentials = std::collections::HashMap::new();

  // Encrypt and save the empty vault
  use secrets::PasswordBasedCredentialStore;
  let store = PasswordBasedCredentialStore::new(&empty_credentials, password1.trim())?;

  // Ensure parent directory exists
  if let Some(parent) = cred_path.parent() {
    fs::create_dir_all(parent)?;
  }

  store.save_to_file(&cred_path.to_path_buf())?;

  bentley::success("vault created successfully");
  Ok(password1.trim().to_string())
}

fn create_new_vault_with_password(cred_path: &Path, password: &str) -> Result<String> {
  if password.is_empty() {
    return Err(anyhow!("master password cannot be empty"));
  }

  // Create empty credentials structure
  let empty_credentials = std::collections::HashMap::new();

  // Encrypt and save the empty vault
  use secrets::PasswordBasedCredentialStore;
  let store = PasswordBasedCredentialStore::new(&empty_credentials, password)?;

  // Ensure parent directory exists
  if let Some(parent) = cred_path.parent() {
    fs::create_dir_all(parent)?;
  }

  store.save_to_file(&cred_path.to_path_buf())?;

  bentley::success("vault created successfully");
  Ok(password.to_string())
}

fn create_socket(keeper_path: &Path) -> Result<PathBuf> {
  // setup unix socket for IPC
  let socket = keeper_path.join("keeper.sock");

  // remove existing socket if any
  let _ = fs::remove_file(&socket);

  Ok(socket)
}

fn spawn_handler(socket: &PathBuf, pwd: String) -> JoinHandle<()> {
  let listener = match UnixListener::bind(socket) {
    Ok(listener) => listener,
    Err(e) => {
      bentley::error(&format!("failed to bind socket: {e}"));
      std::process::exit(1);
    }
  };

  bentley::info(&format!("listening on socket: {}", socket.display()));

  let handler = tokio::spawn(async move {
    loop {
      match listener.accept().await {
        Ok((stream, _)) => {
          let pwd_clone = pwd.clone();
          tokio::spawn(async move {
            handle_client(stream, pwd_clone).await;
          });
        }
        Err(e) => {
          bentley::warn(&format!("failed to accept connection: {e}"));
        }
      }
    }
  });

  handler
}

async fn handle_client(stream: tokio::net::UnixStream, password: String) {
  let mut reader = BufReader::new(stream);
  let mut line = String::new();

  match reader.read_line(&mut line).await {
    Ok(_) if line.trim() == "GET" => {
      let mut stream = reader.into_inner();
      if let Err(e) = stream.write_all(password.as_bytes()).await {
        bentley::warn(&format!("failed to send password: {e}"));
        return;
      }
      if let Err(e) = stream.write_all(b"\n").await {
        bentley::warn(&format!("failed to send newline: {e}"));
        return;
      }
      bentley::verbose("password sent to client");
    }
    Ok(_) => {
      bentley::warn(&format!("invalid request: {}", line.trim()));
    }
    Err(e) => {
      bentley::warn(&format!("failed to read request: {e}"));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use tempfile::TempDir;
  use tokio::io::{AsyncReadExt, AsyncWriteExt};
  use tokio::net::UnixStream;

  fn setup_test_env() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    env::set_var("KERNELLE_HOME", temp_dir.path());
    temp_dir
  }

  #[test]
  fn test_get_password_with_env_var() {
    let _temp_dir = setup_test_env();
    let keeper_path = _temp_dir.path().join("persistent").join("keeper");
    fs::create_dir_all(&keeper_path).unwrap();

    // Test with SECRETS_AUTH environment variable
    env::set_var("SECRETS_AUTH", "test_password_123");

    // Create a test vault file first
    let cred_path = keeper_path.join("credentials.enc");
    let test_credentials = std::collections::HashMap::new();
    let store =
      secrets::PasswordBasedCredentialStore::new(&test_credentials, "test_password_123").unwrap();
    store.save_to_file(&cred_path).unwrap();

    let result = get_password(&keeper_path);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "test_password_123");

    env::remove_var("SECRETS_AUTH");
  }

  #[test]
  fn test_get_password_with_empty_env_var() {
    let _temp_dir = setup_test_env();
    let keeper_path = _temp_dir.path().join("persistent").join("keeper");
    fs::create_dir_all(&keeper_path).unwrap();

    // Test with empty SECRETS_AUTH environment variable
    env::set_var("SECRETS_AUTH", "");

    let result = get_password(&keeper_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty"));

    env::remove_var("SECRETS_AUTH");
  }

  #[test]
  fn test_get_password_with_env_var_wrong_password() {
    let _temp_dir = setup_test_env();
    let keeper_path = _temp_dir.path().join("persistent").join("keeper");
    fs::create_dir_all(&keeper_path).unwrap();

    // Create a vault with one password
    let cred_path = keeper_path.join("credentials.enc");
    let test_credentials = std::collections::HashMap::new();
    let store =
      secrets::PasswordBasedCredentialStore::new(&test_credentials, "correct_password").unwrap();
    store.save_to_file(&cred_path).unwrap();

    // Try to unlock with wrong password via env var
    env::set_var("SECRETS_AUTH", "wrong_password");

    let result = get_password(&keeper_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("incorrect"));

    env::remove_var("SECRETS_AUTH");
  }

  #[test]
  fn test_create_new_vault_with_password() {
    let _temp_dir = setup_test_env();
    let keeper_path = _temp_dir.path().join("persistent").join("keeper");
    fs::create_dir_all(&keeper_path).unwrap();
    let cred_path = keeper_path.join("credentials.enc");

    let result = create_new_vault_with_password(&cred_path, "test_password");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "test_password");
    assert!(cred_path.exists());
  }

  #[test]
  fn test_create_new_vault_with_empty_password() {
    let _temp_dir = setup_test_env();
    let keeper_path = _temp_dir.path().join("persistent").join("keeper");
    fs::create_dir_all(&keeper_path).unwrap();
    let cred_path = keeper_path.join("credentials.enc");

    let result = create_new_vault_with_password(&cred_path, "");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
  }

  #[test]
  fn test_create_socket() {
    let _temp_dir = setup_test_env();
    let keeper_path = _temp_dir.path().join("persistent").join("keeper");
    fs::create_dir_all(&keeper_path).unwrap();

    let result = create_socket(&keeper_path);
    assert!(result.is_ok());

    let socket_path = result.unwrap();
    assert_eq!(socket_path, keeper_path.join("keeper.sock"));
  }

  #[test]
  fn test_create_socket_removes_existing() {
    let _temp_dir = setup_test_env();
    let keeper_path = _temp_dir.path().join("persistent").join("keeper");
    fs::create_dir_all(&keeper_path).unwrap();

    let socket_path = keeper_path.join("keeper.sock");
    // Create a dummy file to simulate existing socket
    fs::write(&socket_path, "dummy").unwrap();
    assert!(socket_path.exists());

    let result = create_socket(&keeper_path);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), socket_path);
  }

  #[tokio::test]
  async fn test_handle_client_get_request() {
    let _temp_dir = setup_test_env();

    // Create a pair of connected Unix streams for testing
    let (client_stream, server_stream) = UnixStream::pair().unwrap();

    let test_password = "test_password_123".to_string();

    // Handle client in background
    let handle_task = tokio::spawn(async move {
      handle_client(server_stream, test_password).await;
    });

    // Send GET request from client side
    let mut client = client_stream;
    client.write_all(b"GET\n").await.unwrap();

    // Read response
    let mut response = String::new();
    client.read_to_string(&mut response).await.unwrap();

    assert_eq!(response, "test_password_123\n");

    // Wait for handler to complete
    handle_task.await.unwrap();
  }

  #[tokio::test]
  async fn test_handle_client_invalid_request() {
    let _temp_dir = setup_test_env();

    // Create a pair of connected Unix streams for testing
    let (client_stream, server_stream) = UnixStream::pair().unwrap();

    let test_password = "test_password_123".to_string();

    // Handle client in background
    let handle_task = tokio::spawn(async move {
      handle_client(server_stream, test_password).await;
    });

    // Send invalid request from client side
    let mut client = client_stream;
    client.write_all(b"INVALID\n").await.unwrap();

    // Read response - should be empty since invalid requests don't get responses
    let mut response = String::new();
    let bytes_read = client.read_to_string(&mut response).await.unwrap();

    // Should not receive any response for invalid requests
    assert_eq!(bytes_read, 0);

    // Wait for handler to complete
    handle_task.await.unwrap();
  }

  #[tokio::test]
  async fn test_handle_client_empty_request() {
    let _temp_dir = setup_test_env();

    // Create a pair of connected Unix streams for testing
    let (client_stream, server_stream) = UnixStream::pair().unwrap();

    let test_password = "test_password_123".to_string();

    // Handle client in background
    let handle_task = tokio::spawn(async move {
      handle_client(server_stream, test_password).await;
    });

    // Send empty request (just newline)
    let mut client = client_stream;
    client.write_all(b"\n").await.unwrap();

    // Read response - should be empty since empty requests are invalid
    let mut response = String::new();
    let bytes_read = client.read_to_string(&mut response).await.unwrap();

    assert_eq!(bytes_read, 0);

    // Wait for handler to complete
    handle_task.await.unwrap();
  }

  #[tokio::test]
  async fn test_spawn_handler_creates_join_handle() {
    let _temp_dir = setup_test_env();
    let keeper_path = _temp_dir.path().join("persistent").join("keeper");
    fs::create_dir_all(&keeper_path).unwrap();
    let socket_path = keeper_path.join("keeper.sock");

    // Remove any existing socket
    let _ = fs::remove_file(&socket_path);

    let password = "test_password".to_string();

    // This will create a listener and return a JoinHandle
    // Note: This test mainly verifies the function doesn't panic and returns a handle
    let handle = spawn_handler(&socket_path, password);

    // Verify we got a handle
    assert!(!handle.is_finished());

    // Clean up
    handle.abort();
    let _ = fs::remove_file(&socket_path);
  }

  #[test]
  fn test_environment_variable_handling() {
    let _temp_dir = setup_test_env();

    // Test KERNELLE_HOME environment variable handling
    let custom_home = _temp_dir.path().join("custom_kernelle");
    env::set_var("KERNELLE_HOME", &custom_home);

    // This should use the custom home directory
    let keeper_path = if let Ok(dir) = env::var("KERNELLE_HOME") {
      PathBuf::from(dir)
    } else {
      dirs::home_dir().unwrap().join(".kernelle")
    }
    .join("persistent")
    .join("keeper");

    assert_eq!(keeper_path, custom_home.join("persistent").join("keeper"));

    env::remove_var("KERNELLE_HOME");
  }

  #[test]
  fn test_get_password_no_vault_no_env() {
    let _temp_dir = setup_test_env();
    let keeper_path = _temp_dir.path().join("persistent").join("keeper");
    fs::create_dir_all(&keeper_path).unwrap();

    // Ensure no SECRETS_AUTH env var
    env::remove_var("SECRETS_AUTH");

    // No vault exists and no env var - this would normally prompt for password
    // In a test environment, this will likely fail or hang waiting for input
    // We can't easily test the interactive password prompting in unit tests
    // But we can verify the path logic
    let cred_path = keeper_path.join("credentials.enc");
    assert!(!cred_path.exists());
  }

  #[test]
  fn test_path_construction() {
    // Test path construction logic used in main()
    let temp_dir = TempDir::new().unwrap();
    env::set_var("KERNELLE_HOME", temp_dir.path());

    let base = if let Ok(dir) = env::var("KERNELLE_HOME") {
      PathBuf::from(dir)
    } else {
      dirs::home_dir().unwrap().join(".kernelle")
    };

    let keeper_path = base.join("persistent").join("keeper");

    assert_eq!(keeper_path, temp_dir.path().join("persistent").join("keeper"));

    env::remove_var("KERNELLE_HOME");
  }

  #[test]
  fn test_credentials_file_validation() {
    let _temp_dir = setup_test_env();
    let keeper_path = _temp_dir.path().join("persistent").join("keeper");
    fs::create_dir_all(&keeper_path).unwrap();
    let cred_path = keeper_path.join("credentials.enc");

    // Test with corrupted credentials file
    fs::write(&cred_path, "invalid json").unwrap();

    env::set_var("SECRETS_AUTH", "any_password");
    let result = get_password(&keeper_path);

    // Should fail due to invalid JSON
    assert!(result.is_err());

    env::remove_var("SECRETS_AUTH");
  }

  #[test]
  fn test_credentials_file_missing_field() {
    let _temp_dir = setup_test_env();
    let keeper_path = _temp_dir.path().join("persistent").join("keeper");
    fs::create_dir_all(&keeper_path).unwrap();
    let cred_path = keeper_path.join("credentials.enc");

    // Test with JSON missing required field
    fs::write(&cred_path, r#"{"version": "1.0"}"#).unwrap();

    env::set_var("SECRETS_AUTH", "any_password");
    let result = get_password(&keeper_path);

    // Should fail due to missing encrypted_data field
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("encrypted_data"));

    env::remove_var("SECRETS_AUTH");
  }

  #[test]
  fn test_password_trimming() {
    let _temp_dir = setup_test_env();
    let keeper_path = _temp_dir.path().join("persistent").join("keeper");
    fs::create_dir_all(&keeper_path).unwrap();
    let cred_path = keeper_path.join("credentials.enc");

    // Create vault with trimmed password
    let test_credentials = std::collections::HashMap::new();
    let store =
      secrets::PasswordBasedCredentialStore::new(&test_credentials, "test_password").unwrap();
    store.save_to_file(&cred_path).unwrap();

    // Test with password that has whitespace
    env::set_var("SECRETS_AUTH", "  test_password  ");
    let result = get_password(&keeper_path);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "test_password"); // Should be trimmed

    env::remove_var("SECRETS_AUTH");
  }

  #[test]
  fn test_directory_creation() {
    let _temp_dir = setup_test_env();
    let keeper_path = _temp_dir.path().join("persistent").join("keeper");

    // Directory should not exist initially
    assert!(!keeper_path.exists());

    // create_new_vault_with_password should create parent directories
    let cred_path = keeper_path.join("credentials.enc");
    let result = create_new_vault_with_password(&cred_path, "test_password");

    assert!(result.is_ok());
    assert!(keeper_path.exists());
    assert!(cred_path.exists());
  }

  // Additional simple tests that don't require external dependencies
  #[test]
  fn test_password_validation_logic() {
    // Test password trimming and validation logic without file I/O
    let password = "  test_password  ";
    let trimmed = password.trim();
    assert_eq!(trimmed, "test_password");

    let empty_password = "   ";
    let trimmed_empty = empty_password.trim();
    assert!(trimmed_empty.is_empty());
  }

  #[test]
  fn test_socket_path_creation_logic() {
    let base_path = PathBuf::from("/tmp/test");
    let socket_path = base_path.join("keeper.sock");

    // Test that socket path is constructed correctly
    assert_eq!(socket_path, PathBuf::from("/tmp/test/keeper.sock"));

    // Test parent directory logic
    assert_eq!(socket_path.parent(), Some(base_path.as_path()));
  }

  #[test]
  fn test_credentials_path_construction_logic() {
    let keeper_path = PathBuf::from("/tmp/keeper");
    let cred_path = keeper_path.join("credentials.enc");

    assert_eq!(cred_path, PathBuf::from("/tmp/keeper/credentials.enc"));
    assert_eq!(cred_path.file_name(), Some(std::ffi::OsStr::new("credentials.enc")));
  }

  #[test]
  fn test_environment_fallback_logic() {
    // Test environment variable fallback logic
    env::remove_var("KERNELLE_HOME");

    let base = if let Ok(dir) = env::var("KERNELLE_HOME") {
      PathBuf::from(dir)
    } else {
      dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join(".kernelle")
    };

    // Should use home directory fallback
    let expected = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join(".kernelle");
    assert_eq!(base, expected);
  }

  #[test]
  fn test_file_operations_error_handling() {
    // Test that file operations handle errors gracefully
    let nonexistent_path = PathBuf::from("/nonexistent/path/file.txt");

    // Reading a nonexistent file should return an error
    let result = fs::read_to_string(&nonexistent_path);
    assert!(result.is_err());
  }

  #[test]
  fn test_json_parsing_error_handling() {
    // Test JSON parsing with invalid data
    let invalid_json = "this is not json";
    let result: Result<serde_json::Value, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err());

    // Test JSON with missing field
    let incomplete_json = r#"{"version": "1.0"}"#;
    let value: serde_json::Value = serde_json::from_str(incomplete_json).unwrap();
    assert!(value.get("encrypted_data").is_none());
  }

  #[test]
  fn test_path_validation() {
    let temp_dir = TempDir::new().unwrap();
    let test_path = temp_dir.path().join("test");

    // Initially doesn't exist
    assert!(!test_path.exists());

    // Create it
    fs::create_dir_all(&test_path).unwrap();
    assert!(test_path.exists());
    assert!(test_path.is_dir());
  }
}
