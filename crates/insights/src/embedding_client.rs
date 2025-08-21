use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use colored::*;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::{sleep, Duration};

// Platform-specific imports
#[cfg(windows)]
use tokio::net::TcpStream;
#[cfg(unix)]
use tokio::net::UnixStream;

use crate::insight::{self, Insight};

// Platform-specific constants
#[cfg(unix)]
const SOCKET_PATH: &str = "/tmp/insights_embeddings.sock";
#[cfg(windows)]
const TCP_ADDRESS: &str = "127.0.0.1:47291";

const STARTUP_DELAY_MS: u64 = 500;

// Cross-platform stream abstraction
enum InsightsStream {
  #[cfg(unix)]
  Unix(UnixStream),
  #[cfg(windows)]
  Tcp(TcpStream),
}

impl InsightsStream {
  async fn write_all(&mut self, buf: &[u8]) -> Result<(), std::io::Error> {
    match self {
      #[cfg(unix)]
      InsightsStream::Unix(stream) => stream.write_all(buf).await,
      #[cfg(windows)]
      InsightsStream::Tcp(stream) => stream.write_all(buf).await,
    }
  }
}

// Core data structures
#[derive(Debug, Clone)]
pub struct Embedding {
  pub version: String,
  pub created_at: DateTime<Utc>,
  pub embedding: Vec<f32>,
}

#[derive(Serialize, Deserialize)]
struct EmbeddingRequest {
  texts: Vec<String>,
  id: String,
}

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

// Service implementations
pub struct ProductionEmbeddingService;

impl EmbeddingService for ProductionEmbeddingService {
  fn embed_insight(&self, insight: &mut Insight) -> Embedding {
    blocking_embed(insight)
  }
}

#[allow(dead_code)]
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
  real_blocking_embed(insight)
}

fn real_blocking_embed(insight: &mut Insight) -> Embedding {
  let rt = tokio::runtime::Runtime::new().unwrap();
  match rt.block_on(async { compute_insight_embedding(insight).await }) {
    Ok(embedding) => embedding,
    Err(e) => {
      eprintln!("  {} Warning: Failed to compute embedding: {}", "[WARN]".yellow(), e);
      eprintln!("  {} Insight saved without embedding", "ℹ".blue());

      // Return a placeholder embedding instead of panicking
      Embedding { version: "placeholder".to_string(), created_at: Utc::now(), embedding: vec![] }
    }
  }
}

async fn compute_insight_embedding(insight: &Insight) -> Result<Embedding> {
  let embedding_text = insight::get_embedding_text(insight);
  let result = request(&embedding_text).await;
  let version = "all-MiniLM-L6-v2".to_string();

  match result {
    Ok(embedding) => Ok(Embedding { version, created_at: Utc::now(), embedding }),
    Err(e) => Err(e),
  }
}

async fn request(text: &str) -> Result<Vec<f32>> {
  if let Ok(embedding) = request_embedding_from_daemon(text).await {
    return Ok(embedding);
  }

  request_embedding_with_retry(text).await
}

async fn request_embedding_from_daemon(text: &str) -> Result<Vec<f32>> {
  let request = create_request(text);
  let response = send(&request).await?;
  parse_response(response)
}

fn create_request(text: &str) -> EmbeddingRequest {
  EmbeddingRequest { texts: vec![text.to_string()], id: uuid::Uuid::new_v4().to_string() }
}

async fn send(request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
  let mut stream = connect_to_daemon().await?;
  stream_to(&mut stream, request).await?;
  stream_from(&mut stream).await
}

#[cfg(unix)]
async fn connect_to_daemon() -> Result<InsightsStream> {
  let stream = UnixStream::connect(SOCKET_PATH).await.map_err(|_| anyhow!("Daemon not running"))?;
  Ok(InsightsStream::Unix(stream))
}

#[cfg(windows)]
async fn connect_to_daemon() -> Result<InsightsStream> {
  let stream = TcpStream::connect(TCP_ADDRESS).await.map_err(|_| anyhow!("Daemon not running"))?;
  Ok(InsightsStream::Tcp(stream))
}

async fn stream_to(stream: &mut InsightsStream, request: &EmbeddingRequest) -> Result<()> {
  let json = serde_json::to_string(request)?;
  stream.write_all(json.as_bytes()).await?;
  stream.write_all(b"\n").await?;
  Ok(())
}

async fn stream_from(stream: &mut InsightsStream) -> Result<EmbeddingResponse> {
  let reader = match stream {
    #[cfg(unix)]
    InsightsStream::Unix(stream) => BufReader::new(stream),
    #[cfg(windows)]
    InsightsStream::Tcp(stream) => BufReader::new(stream),
  };

  let mut line = String::new();
  let mut reader = reader;
  reader.read_line(&mut line).await?;

  serde_json::from_str(line.trim()).map_err(|e| anyhow!("Invalid response: {}", e))
}

fn parse_response(response: EmbeddingResponse) -> Result<Vec<f32>> {
  if let Some(error) = response.error {
    return Err(anyhow!("Daemon error: {}", error));
  }
  Ok(response.embeddings.into_iter().next().unwrap_or_default())
}

async fn request_embedding_with_retry(text: &str) -> Result<Vec<f32>> {
  start_daemon().await?;
  sleep(Duration::from_millis(STARTUP_DELAY_MS)).await;
  request_embedding_from_daemon(text).await
}

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

#[cfg(windows)]
fn get_daemon_executable_path() -> Result<std::path::PathBuf> {
  let current_exe = std::env::current_exe()?;
  let exe_dir =
    current_exe.parent().ok_or_else(|| anyhow!("Could not find executable directory"))?;
  Ok(exe_dir.join("insights_embedding_daemon.exe"))
}

#[cfg(unix)]
fn get_daemon_executable_path() -> Result<std::path::PathBuf> {
  let current_exe = std::env::current_exe()?;
  let exe_dir =
    current_exe.parent().ok_or_else(|| anyhow!("Could not find executable directory"))?;
  Ok(exe_dir.join("insights_embedding_daemon"))
}
