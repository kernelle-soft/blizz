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

async fn run_server_loop<M: blizz::model::EmbeddingModel>(listener: UnixListener, mut service: EmbeddingService<M>) -> Result<()> {
  let inactivity_timeout = Duration::from_secs(INACTIVITY_TIMEOUT_SECS);

  loop {
    match timeout(inactivity_timeout, listener.accept()).await {
      Ok(connection_result) => {
        let should_continue = handle_connection_result(connection_result, &mut service).await;
        if !should_continue {
          break;
        }
      }
      Err(_) => {
        println!("💤 Blizz daemon shutting down due to inactivity");
        break;
      }
    }
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
