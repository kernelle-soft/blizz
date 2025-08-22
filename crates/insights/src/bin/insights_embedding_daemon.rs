use anyhow::anyhow;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use std::path::{Path, PathBuf};
use std::{env, fs};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::signal;
use tokio::task::JoinHandle;

use insights::gte_base::GTEBase;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct EmbeddingRequest {
    request: String,
    body: String,
}

#[derive(Debug, Serialize)]
struct EmbeddingResponse {
    success: bool,
    body: Vec<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorInfo>,
}

#[derive(Debug, Serialize)]
struct ErrorInfo {
    message: String,
    tag: String,
}

impl EmbeddingResponse {
    fn success(embedding: Vec<f32>) -> Self {
        Self {
            success: true,
            body: embedding,
            error: None,
        }
    }

    fn error(message: &str, tag: &str) -> Self {
        Self {
            success: false,
            body: Vec::new(),
            error: Some(ErrorInfo {
                message: message.to_string(),
                tag: tag.to_string(),
            }),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
  let insights_path = get_base()?;

  // Ensure directory exists
  fs::create_dir_all(&insights_path)?;

  // Load the embedder model
  let embedder = match GTEBase::load().await {
    Ok(embedder) => Some(Arc::new(tokio::sync::Mutex::new(embedder))),
    Err(e) => {
      bentley::warn(&format!("Failed to load embedder model: {}", e));
      bentley::warn("Daemon will run without embedding capabilities");
      None
    }
  };

  let socket_path = create_socket(&insights_path)?;
  bentley::info("daemon started - press ctrl+c to exit");

  let ipc_handle = spawn_handler(&socket_path, embedder.map(|e| Arc::clone(&e)));

  // Wait for shutdown signal
  signal::ctrl_c().await?;
  bentley::verbose("\nshutting down daemon");

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

fn spawn_handler(socket: &PathBuf, embedder: Option<Arc<tokio::sync::Mutex<GTEBase>>>) -> JoinHandle<()> {
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
          let embedder_for_task = embedder.as_ref().map(|e| Arc::clone(e));
          tokio::spawn(async move {
            handle_client(stream, embedder_for_task).await;
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

async fn handle_client(mut stream: tokio::net::UnixStream, embedder: Option<Arc<tokio::sync::Mutex<GTEBase>>>) {
  // Read the entire request as JSON
  let mut buffer = Vec::new();
  let response = match stream.read_to_end(&mut buffer).await {
    Ok(_) => {
      match String::from_utf8(buffer) {
        Ok(request_str) => {
          match serde_json::from_str::<EmbeddingRequest>(&request_str) {
            Ok(request) => {
              if request.request == "embed" {
                match embedder {
                  Some(ref embedder_arc) => {
                    let mut embedder_guard = embedder_arc.lock().await;
                    match embedder_guard.embed(&request.body) {
                      Ok(embedding) => {
                        bentley::verbose(&format!("generated embedding for text: {}", 
                          request.body.chars().take(50).collect::<String>()));
                        EmbeddingResponse::success(embedding)
                      }
                      Err(e) => {
                        bentley::warn(&format!("embedding failed: {}", e));
                        EmbeddingResponse::error(&format!("Failed to generate embedding: {}", e), "embedding_failed")
                      }
                    }
                  }
                  None => {
                    bentley::warn("embedding requested but no model loaded");
                    EmbeddingResponse::error("Embedding model not available", "model_not_loaded")
                  }
                }
              } else {
                bentley::warn(&format!("unsupported request type: {}", request.request));
                EmbeddingResponse::error(&format!("Unsupported request type: {}", request.request), "unsupported_request")
              }
            }
            Err(e) => {
              bentley::warn(&format!("failed to parse JSON request: {}", e));
              EmbeddingResponse::error(&format!("Invalid JSON request: {}", e), "invalid_json")
            }
          }
        }
        Err(_) => {
          bentley::warn("received invalid UTF-8 data");
          EmbeddingResponse::error("Invalid UTF-8 data in request", "invalid_utf8")
        }
      }
    }
    Err(e) => {
      bentley::warn(&format!("failed to read from client: {}", e));
      EmbeddingResponse::error(&format!("Failed to read request: {}", e), "read_failed")
    }
  };

  // Send JSON response back to client
  match serde_json::to_string(&response) {
    Ok(response_json) => {
      if let Err(e) = stream.write_all(response_json.as_bytes()).await {
        bentley::warn(&format!("failed to write response: {}", e));
      }
    }
    Err(e) => {
      bentley::error(&format!("failed to serialize response: {}", e));
    }
  }
}
