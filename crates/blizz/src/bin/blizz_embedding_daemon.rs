use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::{timeout, Duration};

// Platform-specific imports
#[cfg(windows)]
use tokio::net::{TcpListener, TcpStream};
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};

#[cfg(feature = "neural")]
use blizz::embedding_model::{create_production_model, EmbeddingModel};

#[cfg(not(feature = "neural"))]
use blizz::embedding_model::MockEmbeddingModel;

// Platform-specific constants
#[cfg(unix)]
const SOCKET_PATH: &str = "/tmp/blizz_embeddings.sock";
#[cfg(windows)]
const TCP_ADDRESS: &str = "127.0.0.1:47291";

const INACTIVITY_TIMEOUT_SECS: u64 = 300;

// Cross-platform listener abstraction
pub enum BlizzListener {
  #[cfg(unix)]
  Unix(UnixListener),
  #[cfg(windows)]
  Tcp(TcpListener),
}

// Cross-platform stream abstraction
pub enum BlizzStream {
  #[cfg(unix)]
  Unix(UnixStream),
  #[cfg(windows)]
  Tcp(TcpStream),
}

impl BlizzListener {
  async fn accept(&self) -> std::io::Result<(BlizzStream, String)> {
    match self {
      #[cfg(unix)]
      BlizzListener::Unix(listener) => {
        let (stream, addr) = listener.accept().await?;
        Ok((BlizzStream::Unix(stream), format!("{addr:?}")))
      }
      #[cfg(windows)]
      BlizzListener::Tcp(listener) => {
        let (stream, addr) = listener.accept().await?;
        Ok((BlizzStream::Tcp(stream), addr.to_string()))
      }
    }
  }
}

impl BlizzStream {
  async fn write_all(&mut self, buf: &[u8]) -> Result<(), std::io::Error> {
    match self {
      #[cfg(unix)]
      BlizzStream::Unix(stream) => stream.write_all(buf).await,
      #[cfg(windows)]
      BlizzStream::Tcp(stream) => stream.write_all(buf).await,
    }
  }
}

/// Embedding service that keeps model loaded in memory
pub struct EmbeddingService<M: EmbeddingModel> {
  model: M,
}

/// Request to compute embeddings (supports batching)
#[derive(Serialize, Deserialize)]
pub struct EmbeddingRequest {
  pub texts: Vec<String>,
  pub id: String,
}

/// Response with computed embeddings (supports batching)
#[derive(Serialize, Deserialize)]
pub struct EmbeddingResponse {
  pub embeddings: Vec<Vec<f32>>,
  pub id: String,
  pub error: Option<String>,
}

impl<M: EmbeddingModel> EmbeddingService<M> {
  pub fn new(model: M) -> Self {
    Self { model }
  }

  pub async fn handle_request(&mut self, request: EmbeddingRequest) -> EmbeddingResponse {
    match self.model.compute_embeddings(&request.texts) {
      Ok(embeddings) => EmbeddingResponse { embeddings, id: request.id, error: None },
      Err(e) => {
        EmbeddingResponse { embeddings: vec![], id: request.id, error: Some(e.to_string()) }
      }
    }
  }
}

#[cfg(feature = "neural")]
async fn create_embedding_service(
) -> Result<EmbeddingService<blizz::embedding_model::OnnxEmbeddingModel>> {
  let model = create_production_model().await?;
  Ok(EmbeddingService::new(model))
}

#[cfg(not(feature = "neural"))]
async fn create_embedding_service() -> Result<EmbeddingService<MockEmbeddingModel>> {
  let model = MockEmbeddingModel::new();
  Ok(EmbeddingService::new(model))
}

#[tokio::main]
async fn main() -> Result<()> {
  cleanup_existing_socket();

  let service = create_embedding_service().await?;
  let listener = setup_listener().await?;

  run_server_loop(listener, service).await?;

  cleanup_existing_socket();
  Ok(())
}

async fn run_server_loop<M: EmbeddingModel>(
  listener: BlizzListener,
  mut service: EmbeddingService<M>,
) -> Result<()> {
  let timeout = Duration::from_secs(INACTIVITY_TIMEOUT_SECS);

  loop {
    let should_continue =
      handle_single_connection(&listener, &mut service, timeout).await.unwrap_or(false);
    if !should_continue {
      break;
    }
  }

  Ok(())
}

async fn handle_single_connection<M: blizz::embedding_model::EmbeddingModel>(
  listener: &BlizzListener,
  service: &mut EmbeddingService<M>,
  timeout_duration: Duration,
) -> Result<bool, ()> {
  match wait_for_connections(listener, timeout_duration).await {
    Ok(connection_result) => {
      let should_continue = handle_connection_result(Ok(connection_result), service).await;
      Ok(should_continue)
    }
    Err(error_msg) => {
      if error_msg == "Timeout" {
        println!("💤 Blizz daemon shutting down due to inactivity");
      }
      Ok(false)
    }
  }
}

async fn wait_for_connections(
  listener: &BlizzListener,
  timeout_duration: Duration,
) -> Result<(BlizzStream, String), String> {
  match timeout(timeout_duration, listener.accept()).await {
    Ok(connection_result) => connection_result.map_err(|e| format!("Connection error: {e}")),
    Err(_) => Err("Timeout".to_string()),
  }
}

async fn handle_connection_result<M: EmbeddingModel>(
  connection_result: Result<(BlizzStream, String), std::io::Error>,
  service: &mut EmbeddingService<M>,
) -> bool {
  match connection_result {
    Ok((stream, _)) => {
      if let Err(e) = handle_client(stream, service).await {
        eprintln!("Error handling client: {e}");
      }
      true // Continue running
    }
    Err(e) => {
      eprintln!("Error accepting connection: {e}");
      false // Stop running
    }
  }
}

pub async fn handle_client<M: EmbeddingModel>(
  mut stream: BlizzStream,
  service: &mut EmbeddingService<M>,
) -> Result<()> {
  let reader = match &mut stream {
    #[cfg(unix)]
    BlizzStream::Unix(stream) => BufReader::new(stream),
    #[cfg(windows)]
    BlizzStream::Tcp(stream) => BufReader::new(stream),
  };

  let mut line = String::new();
  let mut reader = reader;

  // Read request line
  reader.read_line(&mut line).await?;

  // Parse request
  let request: EmbeddingRequest = serde_json::from_str(line.trim())?;

  // Compute embedding
  let response = service.handle_request(request).await;

  // Send response
  let response_json = serde_json::to_string(&response)?;
  stream.write_all(response_json.as_bytes()).await?;
  stream.write_all(b"\n").await?;

  Ok(())
}

// Shared utility functions

#[cfg(unix)]
fn cleanup_existing_socket() {
  let _ = std::fs::remove_file(SOCKET_PATH);
}

#[cfg(windows)]
fn cleanup_existing_socket() {
  // TCP sockets don't require cleanup
}

#[cfg(unix)]
async fn setup_listener() -> Result<BlizzListener> {
  let listener = UnixListener::bind(SOCKET_PATH)?;
  println!("🚀 Blizz daemon listening on {SOCKET_PATH}");
  Ok(BlizzListener::Unix(listener))
}

#[cfg(windows)]
async fn setup_listener() -> Result<BlizzListener> {
  let listener = TcpListener::bind(TCP_ADDRESS).await?;
  println!("🚀 Blizz daemon listening on {TCP_ADDRESS}");
  Ok(BlizzListener::Tcp(listener))
}
