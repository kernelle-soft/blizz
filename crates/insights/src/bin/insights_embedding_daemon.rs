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
    Self { success: true, body: embedding, error: None }
  }

  fn error(message: &str, tag: &str) -> Self {
    Self {
      success: false,
      body: Vec::new(),
      error: Some(ErrorInfo { message: message.to_string(), tag: tag.to_string() }),
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
      bentley::warn(&format!("Failed to load embedder model: {e}"));
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

fn spawn_handler(
  socket: &PathBuf,
  embedder: Option<Arc<tokio::sync::Mutex<GTEBase>>>,
) -> JoinHandle<()> {
  let listener = create_listener(socket);
  bentley::info(&format!("listening on socket: {}", socket.display()));
  tokio::spawn(async move {
    handle_connections(listener, embedder).await;
  })
}

fn create_listener(socket: &PathBuf) -> UnixListener {
  match UnixListener::bind(socket) {
    Ok(listener) => listener,
    Err(e) => {
      bentley::error(&format!("failed to bind socket: {e}"));
      std::process::exit(1);
    }
  }
}

async fn handle_connections(
  listener: UnixListener,
  embedder: Option<Arc<tokio::sync::Mutex<GTEBase>>>,
) {
  loop {
    let connection_result = listener.accept().await;
  
    let (mut stream, _) = match connection_result {
      Ok(connection) => connection,
      Err(e) => {
        bentley::warn(&format!("failed to accept connection: {e}"));
        return;
      }
    };
  
    let embedder_for_task = embedder.as_ref().map(Arc::clone);
    tokio::spawn(async move {
      let response = process_client_request(&mut stream, embedder_for_task).await;
      send_response_to_client(&mut stream, response).await;
    });
  }
}

async fn process_client_request(
  stream: &mut tokio::net::UnixStream,
  embedder: Option<Arc<tokio::sync::Mutex<GTEBase>>>,
) -> EmbeddingResponse {
  let request_data = match read_request_data(stream).await {
    Ok(data) => data,
    Err(response) => return response,
  };

  let request_str = match parse_utf8_data(request_data) {
    Ok(str) => str,
    Err(response) => return response,
  };

  let request = match parse_json_request(&request_str) {
    Ok(req) => req,
    Err(response) => return response,
  };

  if request.request != "embed" {
    bentley::warn(&format!("unsupported request type: {}", request.request));
    return EmbeddingResponse::error(
      &format!("Unsupported request type: {}", request.request),
      "unsupported_request",
    );
  }

  process_embedding_request(&request.body, embedder).await
}

async fn read_request_data(
  stream: &mut tokio::net::UnixStream,
) -> Result<Vec<u8>, EmbeddingResponse> {
  let mut buffer = Vec::new();
  match stream.read_to_end(&mut buffer).await {
    Ok(_) => Ok(buffer),
    Err(e) => {
      bentley::warn(&format!("failed to read from client: {e}"));
      Err(EmbeddingResponse::error(&format!("Failed to read request: {e}"), "read_failed"))
    }
  }
}

fn parse_utf8_data(buffer: Vec<u8>) -> Result<String, EmbeddingResponse> {
  match String::from_utf8(buffer) {
    Ok(s) => Ok(s),
    Err(_) => {
      bentley::warn("received invalid UTF-8 data");
      Err(EmbeddingResponse::error("Invalid UTF-8 data in request", "invalid_utf8"))
    }
  }
}

fn parse_json_request(request_str: &str) -> Result<EmbeddingRequest, EmbeddingResponse> {
  match serde_json::from_str::<EmbeddingRequest>(request_str) {
    Ok(request) => Ok(request),
    Err(e) => {
      bentley::warn(&format!("failed to parse JSON request: {e}"));
      Err(EmbeddingResponse::error(&format!("Invalid JSON request: {e}"), "invalid_json"))
    }
  }
}

async fn process_embedding_request(
  text: &str,
  embedder: Option<Arc<tokio::sync::Mutex<GTEBase>>>,
) -> EmbeddingResponse {
  let embedder_arc = match embedder {
    Some(arc) => arc,
    None => {
      bentley::warn("embedding requested but no model loaded");
      return EmbeddingResponse::error("Embedding model not available", "model_not_loaded");
    }
  };

  let mut embedder_guard = embedder_arc.lock().await;
  match embedder_guard.embed(text) {
    Ok(embedding) => {
      bentley::verbose(&format!(
        "generated embedding for text: {}",
        text.chars().take(50).collect::<String>()
      ));
      EmbeddingResponse::success(embedding)
    }
    Err(e) => {
      bentley::warn(&format!("embedding failed: {e}"));
      EmbeddingResponse::error(&format!("Failed to generate embedding: {e}"), "embedding_failed")
    }
  }
}

async fn send_response_to_client(stream: &mut tokio::net::UnixStream, response: EmbeddingResponse) {
  let response_json = match serde_json::to_string(&response) {
    Ok(json) => json,
    Err(e) => {
      bentley::error(&format!("failed to serialize response: {e}"));
      return;
    }
  };

  if let Err(e) = stream.write_all(response_json.as_bytes()).await {
    bentley::warn(&format!("failed to write response: {e}"));
  }
}
