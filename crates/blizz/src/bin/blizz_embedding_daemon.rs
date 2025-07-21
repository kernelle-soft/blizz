use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[cfg(feature = "neural")]
use blizz::embedding_model::{EmbeddingModel, create_production_model};

#[cfg(not(feature = "neural"))]
use blizz::embedding_model::MockEmbeddingModel;


use tokio::net::UnixListener;
use tokio::time::{timeout, Duration};

const SOCKET_PATH: &str = "/tmp/blizz_embeddings.sock";
const INACTIVITY_TIMEOUT_SECS: u64 = 300;

/// Embedding service that keeps model loaded in memory
pub struct EmbeddingService<M: EmbeddingModel> {
  model: M,
}

impl<M: EmbeddingModel> EmbeddingService<M> {
  /// Initialize the service with a provided model
  pub fn new(model: M) -> Self {
    Self { model }
  }

  /// Handle incoming embedding request
  pub async fn handle_request(&mut self, request: EmbeddingRequest) -> EmbeddingResponse {
    match self.model.compute_embeddings(&request.texts) {
      Ok(embeddings) => EmbeddingResponse { embeddings, id: request.id, error: None },
      Err(e) => {
        EmbeddingResponse { embeddings: vec![], id: request.id, error: Some(e.to_string()) }
      }
    }
  }
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


#[cfg(feature = "neural")]
async fn create_embedding_service() -> Result<EmbeddingService<blizz::embedding_model::OnnxEmbeddingModel>> {
  let model = create_production_model().await?;
  Ok(EmbeddingService::new(model))
}

#[cfg(not(feature = "neural"))]
async fn create_embedding_service() -> Result<EmbeddingService<MockEmbeddingModel>> {
  let model = MockEmbeddingModel::new();
  Ok(EmbeddingService::new(model))
}

/// Cleanup existing socket
fn cleanup_existing_socket() {
  let _ = std::fs::remove_file(SOCKET_PATH);
}

/// Setup listener
async fn setup_listener() -> Result<UnixListener> {
  let listener = UnixListener::bind(SOCKET_PATH)?;
  println!("🚀 Blizz daemon listening on {SOCKET_PATH}");
  Ok(listener)
}

async fn handle_connection_result<M: EmbeddingModel>(
  connection_result: Result<(tokio::net::UnixStream, tokio::net::unix::SocketAddr), std::io::Error>,
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

async fn wait_for_connections(
  listener: &UnixListener,
  timeout_duration: Duration,
) -> Result<(tokio::net::UnixStream, tokio::net::unix::SocketAddr), String> {
  match timeout(timeout_duration, listener.accept()).await {
    Ok(connection_result) => connection_result.map_err(|e| format!("Connection error: {e}")),
    Err(_) => Err("Timeout".to_string()),
  }
}

async fn handle_single_connection<M: blizz::embedding_model::EmbeddingModel>(
  listener: &UnixListener,
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

/// Handle a client connection
pub async fn handle_client<M: EmbeddingModel>(
  mut stream: UnixStream,
  service: &mut EmbeddingService<M>,
) -> Result<()> {
  let mut reader = BufReader::new(&mut stream);
  let mut line = String::new();

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

/// Run the server loop
async fn run_server_loop<M: EmbeddingModel>(
  listener: UnixListener,
  mut service: EmbeddingService<M>,
) -> Result<()> {
  let timeout = Duration::from_secs(INACTIVITY_TIMEOUT_SECS);

  loop {
    let should_continue = handle_single_connection(&listener, &mut service, timeout).await.unwrap_or(false);
    if !should_continue {
      break;
    }
  }

  Ok(())
}

#[tokio::main]
/// Main Entrypoint
async fn main() -> Result<()> {
  cleanup_existing_socket();

  let service = create_embedding_service().await?;
  let listener = setup_listener().await?;

  run_server_loop(listener, service).await?;

  cleanup_existing_socket();
  Ok(())
}