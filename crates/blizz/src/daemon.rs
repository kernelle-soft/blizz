use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::time::{timeout, Duration};

use crate::model::{EmbeddingModel, create_production_model};

/// Request to compute embeddings (supports batching!)
#[derive(Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub texts: Vec<String>,  // Changed to support multiple texts
    pub id: String,
}

/// Response with computed embeddings (supports batching!)
#[derive(Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embeddings: Vec<Vec<f32>>,  // Changed to support multiple embeddings
    pub id: String,
    pub error: Option<String>,
}

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
            Ok(embeddings) => EmbeddingResponse {
                embeddings,
                id: request.id,
                error: None,
            },
            Err(e) => EmbeddingResponse {
                embeddings: vec![],
                id: request.id,
                error: Some(e.to_string()),
            },
        }
    }
}

/// Handle a client connection
async fn handle_client<M: EmbeddingModel>(mut stream: UnixStream, service: &mut EmbeddingService<M>) -> Result<()> {
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

/// Socket path for IPC
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