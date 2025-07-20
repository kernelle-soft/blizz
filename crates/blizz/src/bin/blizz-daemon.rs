use anyhow::Result;
use blizz::daemon::{handle_client, EmbeddingService};
use blizz::model::create_production_model;

#[cfg(not(feature = "neural"))]
use blizz::model::MockEmbeddingModel;
use tokio::net::UnixListener;
use tokio::time::{timeout, Duration};

const SOCKET_PATH: &str = "/tmp/blizz-embeddings.sock";

#[tokio::main]
async fn main() -> Result<()> {
  // Clean up any existing socket
  let _ = std::fs::remove_file(SOCKET_PATH);

  // Create the embedding service (loads model)
  #[cfg(feature = "neural")]
  let model = create_production_model().await?;
  #[cfg(not(feature = "neural"))]
  let model = MockEmbeddingModel::new();

  let mut service = EmbeddingService::new(model);

  // Bind to Unix socket
  let listener = UnixListener::bind(SOCKET_PATH)?;
  println!("🚀 Blizz daemon listening on {SOCKET_PATH}");

  // Auto-shutdown after 5 minutes of inactivity
  let inactivity_timeout = Duration::from_secs(300);

  loop {
    // Wait for connection with timeout
    match timeout(inactivity_timeout, listener.accept()).await {
      Ok(Ok((stream, _))) => {
        // Handle request
        if let Err(e) = handle_client(stream, &mut service).await {
          eprintln!("Error handling client: {e}");
        }
      }
      Ok(Err(e)) => {
        eprintln!("Error accepting connection: {e}");
        break;
      }
      Err(_) => {
        // Timeout - shutdown due to inactivity
        println!("💤 Blizz daemon shutting down due to inactivity");
        break;
      }
    }
  }

  // Clean up socket
  let _ = std::fs::remove_file(SOCKET_PATH);

  Ok(())
}
