use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use crate::model::EmbeddingModel;

/// Request to compute embeddings (supports batching!)
#[derive(Serialize, Deserialize)]
pub struct EmbeddingRequest {
  pub texts: Vec<String>, // Changed to support multiple texts
  pub id: String,
}

/// Response with computed embeddings (supports batching!)
#[derive(Serialize, Deserialize)]
pub struct EmbeddingResponse {
  pub embeddings: Vec<Vec<f32>>, // Changed to support multiple embeddings
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
      Ok(embeddings) => EmbeddingResponse { embeddings, id: request.id, error: None },
      Err(e) => {
        EmbeddingResponse { embeddings: vec![], id: request.id, error: Some(e.to_string()) }
      }
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
