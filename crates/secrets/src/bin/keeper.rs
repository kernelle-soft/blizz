use anyhow::anyhow;
use anyhow::Result;
use dirs;
use rpassword;
use secrets::encryption::{EncryptedBlob, EncryptionManager};
use serde_json::{self, Value};
use std::path::PathBuf;
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

  let master_password = get_password(&keeper_path);

  let (socket, listener) = create_socket(&keeper_path)?;
  bentley::info("press ctrl+c to exit");

  let ipc_handle = spawn_handler(listener, master_password.unwrap());

  signal::ctrl_c().await?;
  bentley::info("\nshutting down");
  let _ = fs::remove_file(&socket);
  ipc_handle.abort();
  Ok(())
}

fn get_password(keeper_path: &PathBuf) -> Result<String> {
  let cred_path = keeper_path.join("credentials.enc");
  bentley::info("password:");
  let master_password = rpassword::prompt_password("> ")?;

  if !cred_path.exists() {
    bentley::error(&format!("no vault found at {:?}", cred_path));
    std::process::exit(1);
  }

  let data = fs::read_to_string(&cred_path)?;
  let store_json: Value = serde_json::from_str(data.trim())?;
  let blob_val = store_json
    .get("encrypted_data")
    .ok_or_else(|| anyhow!("invalid vault format: missing 'encrypted_data'"))?;
  let blob: EncryptedBlob = serde_json::from_value(blob_val.clone())?;
  if EncryptionManager::decrypt_credentials(&blob, &master_password).is_err() {
    bentley::error("incorrect password");
    std::process::exit(1);
  }

  Ok(master_password)
}

fn create_socket(keeper_path: &PathBuf) -> Result<(PathBuf, UnixListener)> {
  // setup unix socket for IPC
  let sock_path = keeper_path.join("keeper.sock");

  // remove existing socket if any
  let _ = fs::remove_file(&sock_path);
  let listener = UnixListener::bind(&sock_path)?;
  bentley::info(&format!("listening on socket: {:?}", sock_path));

  Ok((sock_path, listener))
}

fn spawn_handler(listener: UnixListener, pwd: String) -> JoinHandle<()> {
  let handler = tokio::spawn(async move {
    loop {
      match listener.accept().await {
        Ok((stream, _)) => {
          let mut reader = BufReader::new(stream);
          let mut line = String::new();
          if let Ok(_) = reader.read_line(&mut line).await {
            if line.trim() == "GET" {
              let mut s = reader.into_inner();
              let _ = s.write_all(pwd.as_bytes()).await;
              let _ = s.write_all(b"\n").await;
            }
          }
        }
        Err(_) => {}
      }
    }
  });

  handler
}
