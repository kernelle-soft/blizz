use anyhow::anyhow;
use anyhow::Result;

use std::path::{Path, PathBuf};
use std::{env, fs};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixListener;
use tokio::signal;
use tokio::task::JoinHandle;

use insights::gte_base::GTEBase;

#[tokio::main]
async fn main() -> Result<()> {
  let insights_path = get_base()?;

  // Ensure directory exists
  fs::create_dir_all(&insights_path)?;

  // Load the embedder model
  let embedder = match GTEBase::load().await {
    Ok(embedder) => Some(embedder),
    Err(e) => {
      bentley::warn(&format!("Failed to load embedder model: {}", e));
      bentley::warn("Daemon will run without embedding capabilities");
      None
    }
  };

  let socket_path = create_socket(&insights_path)?;
  bentley::info("daemon started - press ctrl+c to exit");

  let ipc_handle = spawn_handler(&socket_path);

  // Wait for shutdown signal
  signal::ctrl_c().await?;
  bentley::info("\nshutting down daemon");

  // Unload the model if it was loaded
  if let Some(ref embedder) = embedder {
    embedder.unload();
  }

  // Clean up socket file
  let _ = fs::remove_file(&socket_path);

  // Clean up PID file
  let pid_file = insights_path.join("daemon.pid");
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

  let insights_path = base.join("persistent").join("insights");

  Ok(insights_path)
}

fn create_socket(insights_path: &Path) -> Result<PathBuf> {
  let socket = insights_path.join("daemon.sock");
  let _ = fs::remove_file(&socket);
  Ok(socket)
}

fn spawn_handler(socket: &PathBuf) -> JoinHandle<()> {
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
          tokio::spawn(async move {
            handle_client(stream).await;
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

async fn handle_client(stream: tokio::net::UnixStream) {
  let mut reader = BufReader::new(stream);
  let mut line = String::new();

  match reader.read_line(&mut line).await {
    Ok(_) if line.trim() == "GET" => {
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
