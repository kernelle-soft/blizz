use anyhow::Result;
use blizz::daemon::{handle_client, EmbeddingService};
use blizz::model::create_production_model;

#[cfg(not(feature = "neural"))]
use blizz::model::MockEmbeddingModel;
use tokio::net::UnixListener;
use tokio::time::{timeout, Duration};

const SOCKET_PATH: &str = "/tmp/blizz-embeddings.sock";
const INACTIVITY_TIMEOUT_SECS: u64 = 300;

fn cleanup_existing_socket() {
  let _ = std::fs::remove_file(SOCKET_PATH);
}

#[cfg(feature = "neural")]
async fn create_embedding_service() -> Result<EmbeddingService<blizz::model::OnnxEmbeddingModel>> {
  let model = create_production_model().await?;
  Ok(EmbeddingService::new(model))
}

#[cfg(not(feature = "neural"))]
async fn create_embedding_service() -> Result<EmbeddingService<MockEmbeddingModel>> {
  let model = MockEmbeddingModel::new();
  Ok(EmbeddingService::new(model))
}

async fn setup_listener() -> Result<UnixListener> {
  let listener = UnixListener::bind(SOCKET_PATH)?;
  println!("🚀 Blizz daemon listening on {SOCKET_PATH}");
  Ok(listener)
}

async fn handle_connection_result<M: blizz::model::EmbeddingModel>(
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

async fn wait_for_connection_with_timeout(
  listener: &UnixListener,
  timeout_duration: Duration,
) -> Result<(tokio::net::UnixStream, tokio::net::unix::SocketAddr), String> {
  match timeout(timeout_duration, listener.accept()).await {
    Ok(connection_result) => connection_result.map_err(|e| format!("Connection error: {e}")),
    Err(_) => Err("Timeout".to_string()),
  }
}

async fn handle_single_connection<M: blizz::model::EmbeddingModel>(
  listener: &UnixListener,
  service: &mut EmbeddingService<M>,
  timeout_duration: Duration,
) -> Result<bool, ()> {
  match wait_for_connection_with_timeout(listener, timeout_duration).await {
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

async fn run_server_loop<M: blizz::model::EmbeddingModel>(
  listener: UnixListener,
  mut service: EmbeddingService<M>,
) -> Result<()> {
  let timeout = Duration::from_secs(INACTIVITY_TIMEOUT_SECS);

  while let Ok(true) = handle_single_connection(&listener, &mut service, timeout).await {
    // Continue processing connections
  }

  Ok(())
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
