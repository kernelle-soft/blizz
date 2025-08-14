use anyhow::Result;
use secrets::Secrets;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
  // Create a Secrets instance
  let secrets = Secrets::new();

  // Attempt to decrypt the vault by requesting a dummy secret
  // This will prompt for the master password and perform decryption
  match secrets.get_secret_raw_no_setup("", "") {
    Ok(_) => {
      // Found a secret with empty keys (unlikely), treat as success
      println!("✅ Master password verified.");
    }
    Err(err) => {
      let msg = err.to_string();
      if msg.contains("Decryption failed") {
        eprintln!("❌ Master password incorrect");
        std::process::exit(1);
      } else {
        // Decryption succeeded but dummy key not found
        println!("✅ Master password verified.");
      }
    }
  }

  println!("Keeper is running. Press Ctrl+C to exit and keep the key cached.");

  // Hold the process alive until Ctrl+C
  signal::ctrl_c().await?;
  println!("Shutting down keeper.");

  Ok(())
}
