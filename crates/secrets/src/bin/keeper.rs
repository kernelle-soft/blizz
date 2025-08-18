use anyhow::anyhow;
use anyhow::Result;
use secrets::encryption::{EncryptedBlob, EncryptionManager};
use serde_json::{self, Value};

use std::path::{Path, PathBuf};
use std::{env, fs};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::signal;
use tokio::task::JoinHandle;

// Prompt and error message constants
const PROMPT_ENTER_NEW_PASSWORD: &str = "enter new master password:";
const PROMPT_CONFIRM_PASSWORD: &str = "confirm master password:";
const ERROR_PASSWORD_EMPTY: &str = "master password cannot be empty";
const ERROR_PASSWORDS_DONT_MATCH: &str = "passwords do not match";

// Additional test constants
#[cfg(test)]
const PROMPT_NO_VAULT_FOUND: &str = "no vault found";
#[cfg(test)]
const PROMPT_VAULT_CREATED: &str = "vault created successfully";
#[cfg(test)]
const PROMPT_DAEMON_STARTED: &str = "daemon started";
#[cfg(test)]
const ERROR_INCORRECT_PASSWORD: &str = "incorrect password";

#[tokio::main]
async fn main() -> Result<()> {
  let keeper_path = get_base()?;

  // Ensure directory exists
  fs::create_dir_all(&keeper_path)?;
  let cred_path = keeper_path.join("credentials.enc");
  let master_password = if !cred_path.exists() {
    create_new_vault(&cred_path)?
  } else {
    get_master_password(&cred_path)?
  };

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

fn get_base() -> Result<PathBuf> {
  let base = if let Ok(dir) = env::var("KERNELLE_HOME") {
    PathBuf::from(dir)
  } else {
    dirs::home_dir().ok_or_else(|| anyhow!("failed to determine home directory"))?.join(".kernelle")
  };

  let keeper_path = base.join("persistent").join("keeper");

  Ok(keeper_path)
}

fn get_master_password(cred_path: &Path) -> Result<String> {
  let master_password: String = if let Ok(password) =   env::var("SECRETS_AUTH") {
    password.trim().to_string()
  } else {
    let master_password = prompt_for_password("enter master password:")?;
    master_password.trim().to_string()
  };

  if master_password.trim().is_empty() {
    return Err(anyhow!(ERROR_PASSWORD_EMPTY));
  }

  verify_password(&cred_path, &master_password)?;
  Ok(master_password.trim().to_string())
}

fn verify_password(cred_path: &Path, master_password: &str) -> Result<()> {
  let data = fs::read_to_string(&cred_path)?;
  let store_json: Value = serde_json::from_str(data.trim())?;
  let blob_val = store_json
    .get("encrypted_data")
    .ok_or_else(|| anyhow!("invalid vault format: missing 'encrypted_data'"))?;
  let blob: EncryptedBlob = serde_json::from_value(blob_val.clone())?;

  if let Err(e) = EncryptionManager::decrypt_credentials(&blob, master_password.trim()) {
    return Err(anyhow!("incorrect password: {e}"));
  }

  Ok(())
}

fn create_new_vault(cred_path: &Path) -> Result<String> {
  bentley::info("no vault found. creating new vault...");
  let password1 = prompt_for_password(PROMPT_ENTER_NEW_PASSWORD)?;
  if password1.trim().is_empty() {
    return Err(anyhow!(ERROR_PASSWORD_EMPTY));
  }

  let password2 = prompt_for_password(PROMPT_CONFIRM_PASSWORD)?;
  if password1 != password2 {
    return Err(anyhow!(ERROR_PASSWORDS_DONT_MATCH));
  }

  let empty_credentials = std::collections::HashMap::new();
  use secrets::PasswordBasedCredentialStore;
  let store = PasswordBasedCredentialStore::new(&empty_credentials, password1.trim())?;

  if let Some(parent) = cred_path.parent() {
    fs::create_dir_all(parent)?;
  }

  store.save_to_file(&cred_path.to_path_buf())?;

  bentley::success("vault created successfully");
  Ok(password1.trim().to_string())
}

fn prompt_for_password(message: &str) -> Result<String> {
  use dialoguer::Password;
  
  let password = Password::new()
    .with_prompt(message)
    .interact()?;
    
  Ok(password.trim().to_string())
}

fn create_socket(keeper_path: &Path) -> Result<PathBuf> {
  let socket = keeper_path.join("keeper.sock");
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
  use temp_env;
  use tempfile::TempDir;
  use assert_cmd::Command;
  use predicates::prelude::*;
  use rexpect::session::spawn_command;
  use std::process::Command as StdCommand;
  use assert_cmd::cargo::CommandCargoExt;


  fn with_temp_env<F, R>(f: F) -> R
  where
    F: FnOnce(&TempDir) -> R,
  {
    let temp_dir = TempDir::new().unwrap();
    temp_env::with_var("KERNELLE_HOME", Some(temp_dir.path().to_str().unwrap()), || f(&temp_dir))
  }

  #[test]
  fn test_get_base_gets_correct_base() {
    with_temp_env(|temp_dir| {
      let keeper_path = get_base().unwrap();
      assert_eq!(keeper_path, temp_dir.path().join("persistent").join("keeper"));
    });
  }

  #[test]
  fn test_get_master_password_uses_secrets_auth_var() {
    with_temp_env(|temp_dir| {
      let test_password = "test_password_123";
      
      // First, create a vault interactively using rexpect
      let mut cmd = StdCommand::cargo_bin("keeper").unwrap();
      cmd.env("KERNELLE_HOME", temp_dir.path());
      
      let mut session = spawn_command(cmd, Some(5000)).unwrap();
      
      // Expect the initial vault setup prompts
      session.exp_string(PROMPT_NO_VAULT_FOUND).unwrap();
      session.exp_string(PROMPT_ENTER_NEW_PASSWORD).unwrap();
      session.send_line(test_password).unwrap();
      
      session.exp_string(PROMPT_CONFIRM_PASSWORD).unwrap();
      session.send_line(test_password).unwrap();
      
      // Expect successful vault creation
      session.exp_string(PROMPT_VAULT_CREATED).unwrap();
      session.exp_string(PROMPT_DAEMON_STARTED).unwrap();
      
      // Terminate the first daemon before testing SECRETS_AUTH
      drop(session);
      
      // Give the daemon time to shut down completely
      std::thread::sleep(std::time::Duration::from_millis(100));
      
      // Now test that SECRETS_AUTH is used (daemon should start without prompting)
      temp_env::with_var("SECRETS_AUTH", Some(test_password), || {
        let mut cmd = Command::cargo_bin("keeper").unwrap();
        let output = cmd.env("KERNELLE_HOME", temp_dir.path())
           .timeout(std::time::Duration::from_secs(2))
           .output()
           .expect("Failed to execute keeper command");
           
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
          stderr.contains(PROMPT_DAEMON_STARTED) || stdout.contains(PROMPT_DAEMON_STARTED),
          "Should have started daemon using SECRETS_AUTH. STDERR: '{}', STDOUT: '{}'", stderr, stdout
        );
      });
    });
  }

  #[test]
  fn test_get_master_password_uses_secrets_auth_validates_non_empty() {
    with_temp_env(|temp_dir| {
      let test_password = "valid_password_123";
      
      // First, create a vault interactively using rexpect
      let mut cmd = StdCommand::cargo_bin("keeper").unwrap();
      cmd.env("KERNELLE_HOME", temp_dir.path());
      
      let mut session = spawn_command(cmd, Some(5000)).unwrap();
      
      session.exp_string(PROMPT_NO_VAULT_FOUND).unwrap();
      session.exp_string(PROMPT_ENTER_NEW_PASSWORD).unwrap();
      session.send_line(test_password).unwrap();
      
      session.exp_string(PROMPT_CONFIRM_PASSWORD).unwrap();
      session.send_line(test_password).unwrap();
      
      session.exp_string(PROMPT_VAULT_CREATED).unwrap();
      session.exp_string(PROMPT_DAEMON_STARTED).unwrap();
      
      // Let the daemon run - test validation is complete
      
      // Test that empty SECRETS_AUTH is rejected
      temp_env::with_var("SECRETS_AUTH", Some(""), || {
        let mut cmd = Command::cargo_bin("keeper").unwrap();
        cmd.env("KERNELLE_HOME", temp_dir.path())
           .timeout(std::time::Duration::from_secs(2))
           .assert()
           .failure()
           .stderr(predicate::str::contains(ERROR_PASSWORD_EMPTY));
      });
      
      // Test that whitespace-only SECRETS_AUTH is rejected
      temp_env::with_var("SECRETS_AUTH", Some("   \n  \t  "), || {
        let mut cmd = Command::cargo_bin("keeper").unwrap();
        cmd.env("KERNELLE_HOME", temp_dir.path())
           .timeout(std::time::Duration::from_secs(2))
           .assert()
           .failure()
           .stderr(predicate::str::contains(ERROR_PASSWORD_EMPTY));
      });
    });
  }

  #[test]
  fn test_create_vault_handles_non_terminal_environment() {
    with_temp_env(|temp_dir| {
      // Test that the program handles non-terminal environments gracefully
      // (e.g., when run from a script or test environment)
      let mut cmd = Command::cargo_bin("keeper").unwrap();
      let output = cmd.env("KERNELLE_HOME", temp_dir.path())
         .timeout(std::time::Duration::from_secs(3))
         .output()
         .expect("Failed to execute keeper command");
         
      let stderr = String::from_utf8_lossy(&output.stderr);
      let stdout = String::from_utf8_lossy(&output.stdout);
      
      // The program should either start successfully or fail gracefully with a terminal-related error
      assert!(
        stderr.contains(PROMPT_NO_VAULT_FOUND) || 
        stderr.contains("not a terminal") || 
        stderr.contains("IO error") ||
        stdout.contains(PROMPT_NO_VAULT_FOUND),
        "Program should handle non-terminal environment gracefully. STDERR: '{}', STDOUT: '{}'", 
        stderr, stdout
      );
    });
  }

  #[test]
  fn test_create_vault_creates_vault() {
    with_temp_env(|temp_dir| {
      let test_password = "strong_test_password_123";
      
      // Test successful vault creation with matching passwords
      let mut cmd = StdCommand::cargo_bin("keeper").unwrap();
      cmd.env("KERNELLE_HOME", temp_dir.path());
      
      let mut session = spawn_command(cmd, Some(5000)).unwrap();
      
      session.exp_string(PROMPT_NO_VAULT_FOUND).unwrap();
      session.exp_string(PROMPT_ENTER_NEW_PASSWORD).unwrap();
      session.send_line(test_password).unwrap();
      
      session.exp_string(PROMPT_CONFIRM_PASSWORD).unwrap();
      session.send_line(test_password).unwrap();
      
      session.exp_string(PROMPT_VAULT_CREATED).unwrap();
      session.exp_string(PROMPT_DAEMON_STARTED).unwrap();
      
      // Let the daemon run - test validation is complete
      
      // Verify the vault file was actually created
      let vault_path = temp_dir.path().join("persistent").join("keeper").join("credentials.enc");
      assert!(vault_path.exists(), "Vault file should exist after creation");
      
      // Test password mismatch during vault creation
      let mut cmd2 = StdCommand::cargo_bin("keeper").unwrap();
      cmd2.env("KERNELLE_HOME", temp_dir.path().join("mismatch_test"));
      
      let mut session2 = spawn_command(cmd2, Some(5000)).unwrap();
      
      session2.exp_string("no vault found").unwrap();
      session2.exp_string(PROMPT_ENTER_NEW_PASSWORD).unwrap();
      session2.send_line("password1").unwrap();
      
      session2.exp_string(PROMPT_CONFIRM_PASSWORD).unwrap();
      session2.send_line("password2").unwrap(); // Different password
      
      session2.exp_string(ERROR_PASSWORDS_DONT_MATCH).unwrap();
    });
  }

  #[test]
  fn test_create_vault_creates_parent_dir_if_needed() {
    with_temp_env(|temp_dir| {
      let test_password = "test_password_123";
      
      // Ensure parent directories don't exist initially
      let keeper_dir = temp_dir.path().join("persistent").join("keeper");
      assert!(!keeper_dir.exists(), "Keeper directory should not exist initially");
      
      // Create vault - should create parent directories
      let mut cmd = StdCommand::cargo_bin("keeper").unwrap();
      cmd.env("KERNELLE_HOME", temp_dir.path());
      
      let mut session = spawn_command(cmd, Some(5000)).unwrap();
      
      session.exp_string(PROMPT_NO_VAULT_FOUND).unwrap();
      session.exp_string(PROMPT_ENTER_NEW_PASSWORD).unwrap();
      session.send_line(test_password).unwrap();
      
      session.exp_string(PROMPT_CONFIRM_PASSWORD).unwrap();
      session.send_line(test_password).unwrap();
      
      session.exp_string(PROMPT_VAULT_CREATED).unwrap();
      session.exp_string(PROMPT_DAEMON_STARTED).unwrap();
      
      // Let the daemon run - test validation is complete
      
      // Verify parent directories were created
      assert!(keeper_dir.exists(), "Keeper directory should be created");
      assert!(keeper_dir.is_dir(), "Keeper path should be a directory");
      
      // Verify vault file exists in the created directory
      let vault_path = keeper_dir.join("credentials.enc");
      assert!(vault_path.exists(), "Vault file should exist in created directory");
      assert!(vault_path.is_file(), "Vault path should be a file");
    });
  }

  #[test]
  fn test_create_vault_saves_to_file() {
    with_temp_env(|temp_dir| {
      let test_password = "file_save_password_123";
      let vault_path = temp_dir.path().join("persistent").join("keeper").join("credentials.enc");
      
      // Ensure file doesn't exist initially
      assert!(!vault_path.exists(), "Vault file should not exist initially");
      
      // Create vault
      let mut cmd = StdCommand::cargo_bin("keeper").unwrap();
      cmd.env("KERNELLE_HOME", temp_dir.path());
      
      let mut session = spawn_command(cmd, Some(5000)).unwrap();
      
      session.exp_string(PROMPT_NO_VAULT_FOUND).unwrap();
      session.exp_string(PROMPT_ENTER_NEW_PASSWORD).unwrap();
      session.send_line(test_password).unwrap();
      
      session.exp_string(PROMPT_CONFIRM_PASSWORD).unwrap();
      session.send_line(test_password).unwrap();
      
      session.exp_string(PROMPT_VAULT_CREATED).unwrap();
      session.exp_string(PROMPT_DAEMON_STARTED).unwrap();
      
      // Terminate the first daemon before testing vault reuse
      drop(session);
      
      // Give the daemon time to shut down completely
      std::thread::sleep(std::time::Duration::from_millis(100));
      
      // Verify file was saved
      assert!(vault_path.exists(), "Vault file should exist after saving");
      assert!(vault_path.is_file(), "Saved vault should be a file");
      
      // Verify file content is not empty
      let file_contents = std::fs::read_to_string(&vault_path).unwrap();
      assert!(!file_contents.is_empty(), "Saved vault file should not be empty");
      
      // Verify file contains valid JSON structure
      let json_data: serde_json::Value = serde_json::from_str(&file_contents).unwrap();
      assert!(json_data.get("encrypted_data").is_some(), "Saved vault should contain encrypted_data field");
      
      // Verify we can start the daemon again using the saved vault
      temp_env::with_var("SECRETS_AUTH", Some(test_password), || {
        let mut cmd = Command::cargo_bin("keeper").unwrap();
        let output = cmd.env("KERNELLE_HOME", temp_dir.path())
           .timeout(std::time::Duration::from_secs(2))
           .output()
           .expect("Failed to execute keeper command");
           
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
          stderr.contains(PROMPT_DAEMON_STARTED) || stdout.contains(PROMPT_DAEMON_STARTED),
          "Should have started daemon using saved vault. STDERR: '{}', STDOUT: '{}'", stderr, stdout
        );
      });
    });
  }


  #[test]
  fn test_master_password_throws_if_password_is_incorrect() {
    with_temp_env(|temp_dir| {
      let correct_password = "correct_password_123";
      let wrong_password = "definitely_wrong_password";
      
      // First, create a vault with a known password
      let mut cmd = StdCommand::cargo_bin("keeper").unwrap();
      cmd.env("KERNELLE_HOME", temp_dir.path());
      
      let mut session = spawn_command(cmd, Some(5000)).unwrap();
      
      session.exp_string(PROMPT_NO_VAULT_FOUND).unwrap();
      session.exp_string(PROMPT_ENTER_NEW_PASSWORD).unwrap();
      session.send_line(correct_password).unwrap();
      
      session.exp_string(PROMPT_CONFIRM_PASSWORD).unwrap();
      session.send_line(correct_password).unwrap();
      
      session.exp_string(PROMPT_VAULT_CREATED).unwrap();
      session.exp_string(PROMPT_DAEMON_STARTED).unwrap();
      
      // Terminate the first daemon before testing incorrect password
      drop(session);
      
      // Give the daemon time to shut down completely
      std::thread::sleep(std::time::Duration::from_millis(100));
      
      // Test that keeper fails with wrong password from SECRETS_AUTH
      temp_env::with_var("SECRETS_AUTH", Some(wrong_password), || {
        let mut cmd = Command::cargo_bin("keeper").unwrap();
        cmd.env("KERNELLE_HOME", temp_dir.path())
           .timeout(std::time::Duration::from_secs(2))
           .assert()
           .failure()
           .stderr(predicate::str::contains(ERROR_INCORRECT_PASSWORD));
      });
      
      // Verify that correct password still works
      temp_env::with_var("SECRETS_AUTH", Some(correct_password), || {
        let mut cmd = Command::cargo_bin("keeper").unwrap();
        let output = cmd.env("KERNELLE_HOME", temp_dir.path())
           .timeout(std::time::Duration::from_secs(2))
           .output()
           .expect("Failed to execute keeper command");
           
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
          stderr.contains(PROMPT_DAEMON_STARTED) || stdout.contains(PROMPT_DAEMON_STARTED),
          "Should have started daemon with correct password. STDERR: '{}', STDOUT: '{}'", stderr, stdout
        );
      });
    });
  }

  #[test]
  fn test_keeper_ipc_password_retrieval() {
    with_temp_env(|temp_dir| {
      let test_password = "ipc_test_password_123";
      
      // First, create a vault with a known password
      let mut cmd = StdCommand::cargo_bin("keeper").unwrap();
      cmd.env("KERNELLE_HOME", temp_dir.path());
      
      let mut session = spawn_command(cmd, Some(5000)).unwrap();
      
      session.exp_string(PROMPT_NO_VAULT_FOUND).unwrap();
      session.exp_string(PROMPT_ENTER_NEW_PASSWORD).unwrap();
      session.send_line(test_password).unwrap();
      
      session.exp_string(PROMPT_CONFIRM_PASSWORD).unwrap();
      session.send_line(test_password).unwrap();
      
      session.exp_string(PROMPT_VAULT_CREATED).unwrap();
      session.exp_string(PROMPT_DAEMON_STARTED).unwrap();
      
      // Give the daemon a moment to fully start
      std::thread::sleep(std::time::Duration::from_millis(500));
      
      // Now test the IPC functionality - connect as a client and get the password
      let rt = tokio::runtime::Runtime::new().unwrap();
      let test_result = rt.block_on(async {
        let socket_path = temp_dir.path().join("persistent").join("keeper").join("keeper.sock");
        
        // Wait for socket to be available (up to 5 seconds)
        let mut attempts = 0;
        while !socket_path.exists() && attempts < 50 {
          tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
          attempts += 1;
        }
        
        if !socket_path.exists() {
          return Err(format!("Socket not found at: {}", socket_path.display()));
        }
        
        // Connect to the keeper daemon
        let mut stream = match tokio::net::UnixStream::connect(&socket_path).await {
          Ok(stream) => stream,
          Err(e) => return Err(format!("Failed to connect to keeper socket: {}", e)),
        };
        
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        
        // Send GET request
        if let Err(e) = stream.write_all(b"GET\n").await {
          return Err(format!("Failed to send GET request: {}", e));
        }
        
        // Read the password response
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        if let Err(e) = reader.read_line(&mut response).await {
          return Err(format!("Failed to read password response: {}", e));
        }
        
        let received_password = response.trim();
        if received_password == test_password {
          Ok(received_password.to_string())
        } else {
          Err(format!("Password mismatch: expected '{}', got '{}'", test_password, received_password))
        }
      });
      
      // Clean up - terminate the daemon
      drop(session);
      
      // Check the result
      match test_result {
        Ok(retrieved_password) => {
          assert_eq!(retrieved_password, test_password,
            "Retrieved password should match the original");
        }
        Err(error_msg) => {
          panic!("IPC test failed: {}", error_msg);
        }
      }
    });
  }

  #[test]  
  fn test_keeper_ipc_invalid_request() {
    with_temp_env(|temp_dir| {
      let test_password = "invalid_request_test_123";
      
      // Create a vault and start daemon
      let mut cmd = StdCommand::cargo_bin("keeper").unwrap();
      cmd.env("KERNELLE_HOME", temp_dir.path());
      
      let mut session = spawn_command(cmd, Some(5000)).unwrap();
      
      session.exp_string(PROMPT_NO_VAULT_FOUND).unwrap();
      session.exp_string(PROMPT_ENTER_NEW_PASSWORD).unwrap();
      session.send_line(test_password).unwrap();
      
      session.exp_string(PROMPT_CONFIRM_PASSWORD).unwrap();
      session.send_line(test_password).unwrap();
      
      session.exp_string(PROMPT_VAULT_CREATED).unwrap();
      session.exp_string(PROMPT_DAEMON_STARTED).unwrap();
      
      std::thread::sleep(std::time::Duration::from_millis(500));
      
      // Test invalid request handling  
      let rt = tokio::runtime::Runtime::new().unwrap();
      let test_result = rt.block_on(async {
        let socket_path = temp_dir.path().join("persistent").join("keeper").join("keeper.sock");
        
        // Wait for socket
        let mut attempts = 0;
        while !socket_path.exists() && attempts < 50 {
          tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
          attempts += 1;
        }
        
        if !socket_path.exists() {
          return Err("Socket not found".to_string());
        }
        
        // Send invalid request
        let mut stream = tokio::net::UnixStream::connect(&socket_path).await
          .map_err(|e| format!("Connection failed: {}", e))?;
          
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        
        stream.write_all(b"INVALID_COMMAND\n").await
          .map_err(|e| format!("Write failed: {}", e))?;
          
        // The connection should close or we should get no meaningful response
        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        
        // Try to read - this should either fail or return empty/invalid response
        match tokio::time::timeout(
          tokio::time::Duration::from_secs(2), 
          reader.read_line(&mut response)
        ).await {
          Ok(Ok(0)) => Ok("Connection closed as expected".to_string()), // EOF
          Ok(Ok(_)) => {
            // Got some response - check it's not the password
            if response.trim() == test_password {
              Err("Invalid request returned password - security issue!".to_string())
            } else {
              Ok(format!("Got non-password response: {}", response.trim()))
            }
          }
          Ok(Err(_)) => Ok("Read failed as expected".to_string()),
          Err(_) => Ok("Request timed out as expected".to_string()),
        }
      });
      
      drop(session);
      
      // Check result - we expect the invalid request to NOT return the password
      match test_result {
        Ok(_) => {
          // Good - invalid request was handled properly
        }
        Err(error_msg) => {
          panic!("Invalid request test failed: {}", error_msg);
        }
      }
    });
  }

  #[tokio::test]
  async fn test_handle_client_get_request() {
    // Test the handle_client function directly for coverage
    use tokio::net::UnixStream;
    
    let test_password = "unit_test_password_456";
    
    // Create a Unix socket pair for testing
    let (client_stream, server_stream) = UnixStream::pair().expect("Failed to create socket pair");
    
    // Test the handle_client function directly
    let client_task = tokio::spawn(async move {
      use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
      
      let mut client = client_stream;
      
      // Send GET request
      client.write_all(b"GET\n").await.expect("Failed to send GET request");
      
      // Read response
      let mut reader = BufReader::new(client);
      let mut response = String::new();
      reader.read_line(&mut response).await.expect("Failed to read response");
      
      response.trim().to_string()
    });
    
    // Handle the server side
    let server_task = tokio::spawn(async move {
      handle_client(server_stream, test_password.to_string()).await;
    });
    
    // Wait for client to get response
    let received_password = client_task.await.expect("Client task failed");
    
    // Wait for server to finish
    let _ = server_task.await;
    
    // Verify we got the correct password
    assert_eq!(received_password, test_password);
  }

  #[tokio::test]
  async fn test_handle_client_invalid_request() {
    // Test invalid request handling for coverage
    use tokio::net::UnixStream;
    
    let test_password = "unit_test_password_789";
    
    // Create socket pair
    let (client_stream, server_stream) = UnixStream::pair().expect("Failed to create socket pair");
    
    let client_task = tokio::spawn(async move {
      use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
      
      let mut client = client_stream;
      
      // Send invalid request
      client.write_all(b"INVALID\n").await.expect("Failed to send invalid request");
      
      // Try to read response - should get nothing or connection should close
      let mut reader = BufReader::new(client);
      let mut response = String::new();
      
      match tokio::time::timeout(
        tokio::time::Duration::from_millis(500),
        reader.read_line(&mut response)
      ).await {
        Ok(Ok(0)) => "EOF".to_string(), // Connection closed
        Ok(Ok(_)) => response.trim().to_string(), // Got some response
        Ok(Err(_)) => "READ_ERROR".to_string(), // Read error
        Err(_) => "TIMEOUT".to_string(), // Timeout
      }
    });
    
    let server_task = tokio::spawn(async move {
      handle_client(server_stream, test_password.to_string()).await;
    });
    
    let result = client_task.await.expect("Client task failed");
    let _ = server_task.await;
    
    // For invalid requests, we should NOT get the password back
    assert_ne!(result, test_password, "Invalid request should not return the password");
    
    // The result should be empty, EOF, or some error - not the actual password
    assert!(
      result.is_empty() || result == "EOF" || result == "READ_ERROR" || result == "TIMEOUT",
      "Invalid request should not leak the password, got: '{}'", result
    );
  }

  #[tokio::test]
  async fn test_handle_client_connection_error() {
    // Test connection error handling for coverage
    let test_password = "connection_error_test_999";
    
    let (client_stream, server_stream) = tokio::net::UnixStream::pair()
      .expect("Failed to create socket pair");
    
    // Close client side immediately to simulate connection error
    drop(client_stream);
    
    // This should handle the error gracefully and not panic
    let server_task = tokio::spawn(async move {
      handle_client(server_stream, test_password.to_string()).await;
    });
    
    // Should complete without panicking
    let _ = server_task.await.expect("Server should handle connection error gracefully");
  }
}
