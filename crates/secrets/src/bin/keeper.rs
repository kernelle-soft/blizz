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

  if !cred_path.exists() {
    // No vault exists - create one
    bentley::info("no vault found, setting up new vault");
    return create_new_vault(&cred_path);
  }

  // Vault exists - unlock it
  bentley::info("enter master password to unlock daemon:");
  print!("> ");
  std::io::stdout().flush()?;
  let master_password = rpassword::read_password()?;

  if master_password.trim().is_empty() {
    bentley::error("master password cannot be empty");
    std::process::exit(1);
  }

  // Verify password by attempting to decrypt
  let data = fs::read_to_string(&cred_path)?;
  let store_json: Value = serde_json::from_str(data.trim())?;
  let blob_val = store_json
    .get("encrypted_data")
    .ok_or_else(|| anyhow!("invalid vault format: missing 'encrypted_data'"))?;
  let blob: EncryptedBlob = serde_json::from_value(blob_val.clone())?;

  if EncryptionManager::decrypt_credentials(&blob, master_password.trim()).is_err() {
    bentley::error("incorrect password");
    std::process::exit(1);
  }

  Ok(master_password.trim().to_string())
}

fn create_new_vault(cred_path: &Path) -> Result<String> {
  bentley::info("setting up vault - create master password:");
  print!("> ");
  std::io::stdout().flush()?;
  let password1 = rpassword::read_password()?;

  if password1.trim().is_empty() {
    bentley::error("master password cannot be empty");
    std::process::exit(1);
  }

  bentley::info("confirm master password:");
  print!("> ");
  std::io::stdout().flush()?;
  let password2 = rpassword::read_password()?;

  if password1 != password2 {
    bentley::error("passwords do not match");
    std::process::exit(1);
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
      bentley::error(&format!("failed to bind socket: {}", e));
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
          bentley::warn(&format!("failed to accept connection: {}", e));
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
        bentley::warn(&format!("failed to send password: {}", e));
        return;
      }
      if let Err(e) = stream.write_all(b"\n").await {
        bentley::warn(&format!("failed to send newline: {}", e));
        return;
      }
      bentley::verbose("password sent to client");
    }
    Ok(_) => {
      bentley::warn(&format!("invalid request: {}", line.trim()));
    }
    Err(e) => {
      bentley::warn(&format!("failed to read request: {}", e));
    }
  }
}
