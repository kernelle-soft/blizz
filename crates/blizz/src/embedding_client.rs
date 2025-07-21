use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::Command;
use tokio::time::{sleep, Duration};
use colored::*;

use crate::insight::Insight;

#[derive(Debug, Clone)]
pub struct Embedding {
  pub version: String,
  pub created_at: DateTime<Utc>,
  pub embedding: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct Embeddings {
  pub embeddings: Vec<Embedding>,
}

// Daemon communication types and constants
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

#[cfg(feature = "neural")]
const SOCKET_PATH: &str = "/tmp/blizz_embeddings.sock";
#[cfg(feature = "neural")]
const STARTUP_DELAY_MS: u64 = 500;

/// Generate a single embedding for the given text
pub async fn generate_embedding(text: &str) -> Result<Embedding> {
  #[cfg(feature = "neural")]
  {
    let embedding_vec = request(text).await?;
    let version = "all-MiniLM-L6-v2".to_string();
    Ok(Embedding {
      version,
      created_at: Utc::now(),
      embedding: embedding_vec,
    })
  }

  #[cfg(not(feature = "neural"))]
  {
    let _ = text;
    Err(anyhow::anyhow!("Neural features not enabled"))
  }
}

/// Main entry point for getting embeddings from daemon (with fallback)
#[cfg(feature = "neural")]
async fn request(text: &str) -> Result<Vec<f32>> {
  if let Ok(embedding) = request_embedding_from_daemon(text).await {
    return Ok(embedding);
  }

  request_embedding_with_retry(text).await
}

// ==== Daemon Communication Implementation ====

#[cfg(feature = "neural")]
fn create_request(text: &str) -> EmbeddingRequest {
  EmbeddingRequest { 
    texts: vec![text.to_string()], 
    id: uuid::Uuid::new_v4().to_string() 
  }
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
async fn send(request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
  let mut stream = connect_to_daemon().await?;
  stream_to(&mut stream, request).await?;
  stream_from(&mut stream).await
}

#[cfg(feature = "neural")]
fn parse_response(response: EmbeddingResponse) -> Result<Vec<f32>> {
  if let Some(error) = response.error {
    return Err(anyhow!("Daemon error: {}", error));
  }
  Ok(response.embeddings.into_iter().next().unwrap_or_default())
}

#[cfg(feature = "neural")]
async fn request_embedding_from_daemon(text: &str) -> Result<Vec<f32>> {
  let request = create_request(text);
  let response = send(&request).await?;
  parse_response(response)
}

#[cfg(feature = "neural")]
fn get_daemon_executable_path() -> Result<std::path::PathBuf> {
  let current_exe = std::env::current_exe()?;
  let exe_dir =
    current_exe.parent().ok_or_else(|| anyhow!("Could not find executable directory"))?;
  Ok(exe_dir.join("blizz_embedding_daemon"))
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
async fn request_embedding_with_retry(text: &str) -> Result<Vec<f32>> {
  start_daemon().await?;
  sleep(Duration::from_millis(STARTUP_DELAY_MS)).await;
  request_embedding_from_daemon(text).await
}

/// Create embedding - now uses daemon for speed!
#[cfg(feature = "neural")]
pub fn create_embedding(_session: &mut ort::session::Session, text: &str) -> Result<Vec<f32>> {
  // Use async runtime to call the new daemon-enabled function
  let rt = tokio::runtime::Runtime::new()?;
  rt.block_on(async {
    if let Ok(embedding) = request(text).await {
      return Ok(embedding);
    }

    Err(anyhow::anyhow!("Failed to compute embedding"))
  })
}

/// Create embedding using only the daemon (no internet fallback)
#[cfg(feature = "neural")]
pub fn create_embedding_daemon_only(text: &str) -> Result<Vec<f32>> {
  let rt = tokio::runtime::Runtime::new()?;
  rt.block_on(async {
    request_embedding_from_daemon(text).await
  })
}

/// Compute embedding for an insight using the daemon
#[cfg(feature = "neural")]
async fn compute_insight_embedding(insight: &Insight) -> Result<Embedding> {
  let embedding_text = insight.get_embedding_text();
  let rt = tokio::runtime::Runtime::new()?;
  let result = rt.block_on(async { 
    request(&embedding_text).await
  });
  let version = "all-MiniLM-L6-v2".to_string();

  match result {
    Ok(embedding) => {
      Ok(Embedding {
        version,
        created_at: Utc::now(),
        embedding,
      })
    },
    Err(e) => {
      eprintln!("  {} Warning: Failed to compute embedding: {}", "⚠".yellow(), e);
      eprintln!(
        "  {} Insight saved without embedding (can be computed later with 'blizz index')",
        "ℹ".blue()
      );
      Err(e)
    }
  }
}


pub fn embed_insight(insight: &mut Insight) -> Embedding {
  #[cfg(feature = "neural")]
  {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async { 
      compute_insight_embedding(insight).await
    }).unwrap();
    result
  }

  #[cfg(not(feature = "neural"))]
  {
    let _ = insight;
    Ok(())
  }
}