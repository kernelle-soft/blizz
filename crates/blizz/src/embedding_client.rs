use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use colored::*;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::Command;
use tokio::time::{sleep, Duration};

use crate::insight::{self, Insight};

#[cfg(feature = "neural")]
const SOCKET_PATH: &str = "/tmp/blizz_embeddings.sock";
#[cfg(feature = "neural")]
const STARTUP_DELAY_MS: u64 = 500;

// Core data structures
#[derive(Debug, Clone)]
pub struct Embedding {
  pub version: String,
  pub created_at: DateTime<Utc>,
  pub embedding: Vec<f32>,
}

#[cfg(feature = "neural")]
#[derive(Serialize, Deserialize)]
struct EmbeddingRequest {
  texts: Vec<String>,
  id: String,
}

#[cfg(feature = "neural")]
#[derive(Serialize, Deserialize)]
struct EmbeddingResponse {
  embeddings: Vec<Vec<f32>>,
  id: String,
  error: Option<String>,
}

// Service trait for dependency injection
pub trait EmbeddingService {
  fn embed_insight(&self, insight: &mut Insight) -> Embedding;
}

// Main EmbeddingClient struct (the "class")
pub struct EmbeddingClient {
  service: Box<dyn EmbeddingService>,
}

// Constructor functions
/// Create a new embedding client with production service (default)
pub fn create() -> EmbeddingClient {
  EmbeddingClient { service: Box::new(ProductionEmbeddingService) }
}

/// Create a new embedding client with injected service (for testing)
#[allow(dead_code)] // used for dependency injection during testing
pub fn with_service(service: Box<dyn EmbeddingService>) -> EmbeddingClient {
  EmbeddingClient { service }
}

// Client functions (operate on the client instance as first parameter)
pub fn embed_insight(client: &EmbeddingClient, insight: &mut Insight) -> Embedding {
  client.service.embed_insight(insight)
}

pub fn create_embedding(_client: &EmbeddingClient, text: &str) -> Result<Vec<f32>> {
  #[cfg(feature = "neural")]
  {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { request_embedding_from_daemon(text).await })
  }

  #[cfg(not(feature = "neural"))]
  {
    let _ = (_client, text);
    Err(anyhow!("Neural features not enabled"))
  }
}

// Service implementations
pub struct ProductionEmbeddingService;

impl EmbeddingService for ProductionEmbeddingService {
  fn embed_insight(&self, insight: &mut Insight) -> Embedding {
    blocking_embed(insight)
  }
}

pub struct MockEmbeddingService;

impl EmbeddingService for MockEmbeddingService {
  fn embed_insight(&self, _insight: &mut Insight) -> Embedding {
    Embedding {
      version: "test-mock".to_string(),
      created_at: Utc::now(),
      embedding: vec![0.1; 384], // Mock 384-dimensional embedding
    }
  }
}

// Private implementation functions
fn blocking_embed(insight: &mut Insight) -> Embedding {
  #[cfg(feature = "neural")]
  {
    real_blocking_embed(insight)
  }

  #[cfg(not(feature = "neural"))]
  {
    let _ = insight;
    Embedding { version: "mock".to_string(), created_at: Utc::now(), embedding: vec![0.0; 384] }
  }
}

fn real_blocking_embed(insight: &mut Insight) -> Embedding {
  let rt = tokio::runtime::Runtime::new().unwrap();
  match rt.block_on(async { compute_insight_embedding(insight).await }) {
    Ok(embedding) => embedding,
    Err(e) => {
      eprintln!("  {} Warning: Failed to compute embedding: {}", "⚠".yellow(), e);
      eprintln!("  {} Insight saved without embedding", "ℹ".blue());

      // Return a placeholder embedding instead of panicking
      Embedding { version: "placeholder".to_string(), created_at: Utc::now(), embedding: vec![] }
    }
  }
}

#[cfg(feature = "neural")]
async fn compute_insight_embedding(insight: &Insight) -> Result<Embedding> {
  let embedding_text = insight::get_embedding_text(insight);
  let result = request(&embedding_text).await;
  let version = "all-MiniLM-L6-v2".to_string();

  match result {
    Ok(embedding) => Ok(Embedding { version, created_at: Utc::now(), embedding }),
    Err(e) => Err(e),
  }
}

#[cfg(feature = "neural")]
async fn request(text: &str) -> Result<Vec<f32>> {
  if let Ok(embedding) = request_embedding_from_daemon(text).await {
    return Ok(embedding);
  }

  request_embedding_with_retry(text).await
}

#[cfg(feature = "neural")]
async fn request_embedding_from_daemon(text: &str) -> Result<Vec<f32>> {
  let request = create_request(text);
  let response = send(&request).await?;
  parse_response(response)
}

#[cfg(feature = "neural")]
fn create_request(text: &str) -> EmbeddingRequest {
  EmbeddingRequest { texts: vec![text.to_string()], id: uuid::Uuid::new_v4().to_string() }
}

#[cfg(feature = "neural")]
async fn send(request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
  let mut stream = connect_to_daemon().await?;
  stream_to(&mut stream, request).await?;
  stream_from(&mut stream).await
}

#[cfg(feature = "neural")]
async fn connect_to_daemon() -> Result<UnixStream> {
  UnixStream::connect(SOCKET_PATH).await.map_err(|_| anyhow!("Daemon not running"))
}

#[cfg(feature = "neural")]
async fn stream_to(stream: &mut UnixStream, request: &EmbeddingRequest) -> Result<()> {
  let json = serde_json::to_string(request)?;
  stream.write_all(json.as_bytes()).await?;
  stream.write_all(b"\n").await?;
  Ok(())
}

#[cfg(feature = "neural")]
async fn stream_from(stream: &mut UnixStream) -> Result<EmbeddingResponse> {
  let mut reader = BufReader::new(stream);
  let mut line = String::new();
  reader.read_line(&mut line).await?;

  serde_json::from_str(line.trim()).map_err(|e| anyhow!("Invalid response: {}", e))
}

#[cfg(feature = "neural")]
fn parse_response(response: EmbeddingResponse) -> Result<Vec<f32>> {
  if let Some(error) = response.error {
    return Err(anyhow!("Daemon error: {}", error));
  }
  Ok(response.embeddings.into_iter().next().unwrap_or_default())
}

#[cfg(feature = "neural")]
async fn request_embedding_with_retry(text: &str) -> Result<Vec<f32>> {
  start_daemon().await?;
  sleep(Duration::from_millis(STARTUP_DELAY_MS)).await;
  request_embedding_from_daemon(text).await
}

#[cfg(feature = "neural")]
async fn start_daemon() -> Result<()> {
  let daemon_path = get_daemon_executable_path()?;

  Command::new(daemon_path)
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .stdin(Stdio::null())
    .spawn()
    .map_err(|e| anyhow!("Failed to start daemon: {}", e))?;

  Ok(())
}

#[cfg(feature = "neural")]
fn get_daemon_executable_path() -> Result<std::path::PathBuf> {
  let current_exe = std::env::current_exe()?;
  let exe_dir =
    current_exe.parent().ok_or_else(|| anyhow!("Could not find executable directory"))?;
  Ok(exe_dir.join("blizz_embedding_daemon"))
}
