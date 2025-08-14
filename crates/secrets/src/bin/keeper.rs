use anyhow::anyhow;
use anyhow::Result;
use dirs;
use rpassword;
use secrets::encryption::{EncryptedBlob, EncryptionManager};
use serde_json::{self, Value};
use std::path::PathBuf;
use std::{env, fs};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
  let base = if let Ok(dir) = env::var("KERNELLE_HOME") {
    PathBuf::from(dir)
  } else {
    dirs::home_dir().ok_or_else(|| anyhow!("Failed to determine home directory"))?.join(".kernelle")
  };

  let cred_path = base.join("persistent").join("keeper").join("credentials.enc");
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

  bentley::info("press ctrl+c to exit");

  signal::ctrl_c().await?;

  bentley::info("\nshutting down");
  Ok(())
}
