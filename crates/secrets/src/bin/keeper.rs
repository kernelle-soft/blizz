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
    return Err(anyhow!("master password cannot be empty"));
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
  let password1 = prompt_for_password("enter new master password:")?;
  if password1.trim().is_empty() {
    return Err(anyhow!("master password cannot be empty"));
  }

  let password2 = prompt_for_password("confirm master password:")?;
  if password1 != password2 {
    return Err(anyhow!("passwords do not match"));
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
  bentley::info(message);
  print!("> ");
  std::io::stdout().flush()?;
  let password = rpassword::read_password()?;
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
      assert!(keeper_path.exists());
    });
  }

  #[test]
  fn test_get_master_password_uses_secrets_auth_var() {
    with_temp_env(|temp_dir| {
      let test_password = "test_password_123";
      
      // First, create a vault interactively (daemon will start, so we expect it to timeout but succeed)
      let mut cmd = Command::cargo_bin("keeper").unwrap();
      let assert = cmd.env("KERNELLE_HOME", temp_dir.path())
         .write_stdin(format!("{}\n{}\n", test_password, test_password))
         .timeout(std::time::Duration::from_millis(2000)) // Short timeout since daemon will run
         .assert();
         
      // The command should timeout (daemon runs indefinitely) but should have created vault
      // We'll check the output contains vault creation success
      let output = assert.get_output();
      assert!(
        String::from_utf8_lossy(&output.stdout).contains("vault created successfully"),
        "Should have created vault successfully"
      );
      
      // Now test that SECRETS_AUTH is used (daemon should start without prompting)
      temp_env::with_var("SECRETS_AUTH", Some(test_password), || {
        let mut cmd = Command::cargo_bin("keeper").unwrap();
        let assert = cmd.env("KERNELLE_HOME", temp_dir.path())
           .timeout(std::time::Duration::from_millis(1000)) // Short timeout since daemon will run
           .assert();
           
        // Should timeout but successfully start daemon with SECRETS_AUTH
        let output = assert.get_output();
        assert!(
          String::from_utf8_lossy(&output.stdout).contains("daemon started"),
          "Should have started daemon using SECRETS_AUTH"
        );
      });
    });
  }

  #[test]
  fn test_get_master_password_uses_secrets_auth_validates_non_empty() {
    with_temp_env(|temp_dir| {
      let test_password = "valid_password_123";
      
      // First, create a vault interactively (will timeout due to daemon)
      let mut cmd = Command::cargo_bin("keeper").unwrap();
      let assert = cmd.env("KERNELLE_HOME", temp_dir.path())
         .write_stdin(format!("{}\n{}\n", test_password, test_password))
         .timeout(std::time::Duration::from_millis(2000))
         .assert();
         
      let output = assert.get_output();
      assert!(
        String::from_utf8_lossy(&output.stdout).contains("vault created successfully"),
        "Should have created vault successfully"
      );
      
      // Test that empty SECRETS_AUTH is rejected
      temp_env::with_var("SECRETS_AUTH", Some(""), || {
        let mut cmd = Command::cargo_bin("keeper").unwrap();
        cmd.env("KERNELLE_HOME", temp_dir.path())
           .timeout(std::time::Duration::from_secs(2))
           .assert()
           .failure()
           .stderr(predicate::str::contains("master password cannot be empty"));
      });
      
      // Test that whitespace-only SECRETS_AUTH is rejected
      temp_env::with_var("SECRETS_AUTH", Some("   \n  \t  "), || {
        let mut cmd = Command::cargo_bin("keeper").unwrap();
        cmd.env("KERNELLE_HOME", temp_dir.path())
           .timeout(std::time::Duration::from_secs(2))
           .assert()
           .failure()
           .stderr(predicate::str::contains("master password cannot be empty"));
      });
    });
  }

  #[test]
  fn test_create_vault_throws_if_password_is_empty() {
    with_temp_env(|temp_dir| {
      // Test empty password input during vault creation
      let mut cmd = Command::cargo_bin("keeper").unwrap();
      cmd.env("KERNELLE_HOME", temp_dir.path())
         .write_stdin("\n\n") // Empty password inputs
         .timeout(std::time::Duration::from_secs(5))
         .assert()
         .failure()
         .stderr(predicate::str::contains("master password cannot be empty"));
      
      // Test whitespace-only password input during vault creation
      let mut cmd = Command::cargo_bin("keeper").unwrap();
      cmd.env("KERNELLE_HOME", temp_dir.path())
         .write_stdin("   \n   \n") // Whitespace-only password inputs
         .timeout(std::time::Duration::from_secs(5))
         .assert()
         .failure()
         .stderr(predicate::str::contains("master password cannot be empty"));
    });
  }

  #[test]
  fn test_create_vault_creates_vault() {
    with_temp_env(|temp_dir| {
      let test_password = "strong_test_password_123";
      
      // Test successful vault creation with matching passwords (will timeout due to daemon)
      let mut cmd = Command::cargo_bin("keeper").unwrap();
      let assert = cmd.env("KERNELLE_HOME", temp_dir.path())
         .write_stdin(format!("{}\n{}\n", test_password, test_password))
         .timeout(std::time::Duration::from_millis(2000))
         .assert();
         
      let output = assert.get_output();
      let stdout = String::from_utf8_lossy(&output.stdout);
      assert!(stdout.contains("vault created successfully"), "Should have created vault successfully");
      assert!(stdout.contains("daemon started"), "Should have started daemon");
      
      // Verify the vault file was actually created
      let vault_path = temp_dir.path().join("persistent").join("keeper").join("credentials.enc");
      assert!(vault_path.exists(), "Vault file should exist after creation");
      
      // Test password mismatch during vault creation
      let mut cmd = Command::cargo_bin("keeper").unwrap();
      cmd.env("KERNELLE_HOME", temp_dir.path().join("mismatch_test"))
         .write_stdin("password1\npassword2\n") // Mismatched passwords
         .timeout(std::time::Duration::from_secs(5))
         .assert()
         .failure()
         .stderr(predicate::str::contains("passwords do not match"));
    });
  }

  #[test]
  fn test_create_vault_creates_parent_dir_if_needed() {
    with_temp_env(|temp_dir| {
      let test_password = "test_password_123";
      
      // Ensure parent directories don't exist initially
      let keeper_dir = temp_dir.path().join("persistent").join("keeper");
      assert!(!keeper_dir.exists(), "Keeper directory should not exist initially");
      
      // Create vault - should create parent directories (will timeout due to daemon)
      let mut cmd = Command::cargo_bin("keeper").unwrap();
      let assert = cmd.env("KERNELLE_HOME", temp_dir.path())
         .write_stdin(format!("{}\n{}\n", test_password, test_password))
         .timeout(std::time::Duration::from_millis(2000))
         .assert();
         
      let output = assert.get_output();
      assert!(
        String::from_utf8_lossy(&output.stdout).contains("vault created successfully"),
        "Should have created vault successfully"
      );
      
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
      
      // Create vault (will timeout due to daemon)
      let mut cmd = Command::cargo_bin("keeper").unwrap();
      let assert = cmd.env("KERNELLE_HOME", temp_dir.path())
         .write_stdin(format!("{}\n{}\n", test_password, test_password))
         .timeout(std::time::Duration::from_millis(2000))
         .assert();
         
      let output = assert.get_output();
      assert!(
        String::from_utf8_lossy(&output.stdout).contains("vault created successfully"),
        "Should have created vault successfully"
      );
      
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
        let assert = cmd.env("KERNELLE_HOME", temp_dir.path())
           .timeout(std::time::Duration::from_millis(1000))
           .assert();
           
        let output = assert.get_output();
        assert!(
          String::from_utf8_lossy(&output.stdout).contains("daemon started"),
          "Should have started daemon using saved vault"
        );
      });
    });
  }


  #[test]
  fn test_master_password_throws_if_password_is_incorrect() {
    with_temp_env(|temp_dir| {
      let correct_password = "correct_password_123";
      let wrong_password = "definitely_wrong_password";
      
      // First, create a vault with a known password (will timeout due to daemon)
      let mut cmd = Command::cargo_bin("keeper").unwrap();
      let assert = cmd.env("KERNELLE_HOME", temp_dir.path())
         .write_stdin(format!("{}\n{}\n", correct_password, correct_password))
         .timeout(std::time::Duration::from_millis(2000))
         .assert();
         
      let output = assert.get_output();
      assert!(
        String::from_utf8_lossy(&output.stdout).contains("vault created successfully"),
        "Should have created vault successfully"
      );
      
      // Test that keeper fails with wrong password from SECRETS_AUTH
      temp_env::with_var("SECRETS_AUTH", Some(wrong_password), || {
        let mut cmd = Command::cargo_bin("keeper").unwrap();
        cmd.env("KERNELLE_HOME", temp_dir.path())
           .timeout(std::time::Duration::from_secs(2))
           .assert()
           .failure()
           .stderr(predicate::str::contains("incorrect password"));
      });
      
      // Verify that correct password still works
      temp_env::with_var("SECRETS_AUTH", Some(correct_password), || {
        let mut cmd = Command::cargo_bin("keeper").unwrap();
        let assert = cmd.env("KERNELLE_HOME", temp_dir.path())
           .timeout(std::time::Duration::from_millis(1000))
           .assert();
           
        let output = assert.get_output();
        assert!(
          String::from_utf8_lossy(&output.stdout).contains("daemon started"),
          "Should have started daemon with correct password"
        );
      });
    });
  }
}
