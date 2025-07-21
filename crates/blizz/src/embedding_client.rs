use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::Command;
use tokio::time::{sleep, Duration};

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

impl Embedding {
  pub fn new(version: String, embedding: Vec<f32>) -> Self {
    Self {
      version,
      created_at: Utc::now(),
      embedding,
    }
  }
}

impl Embeddings {
  pub fn new(embeddings: Vec<Embedding>) -> Self {
    Self { embeddings }
  }
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
    let embedding_vec = get_embedding_from_daemon(text).await?;
    let version = "all-MiniLM-L6-v2".to_string();
    Ok(Embedding::new(version, embedding_vec))
  }

  #[cfg(not(feature = "neural"))]
  {
    let _ = text;
    Err(anyhow::anyhow!("Neural features not enabled"))
  }
}

/// Generate embeddings for multiple texts
pub async fn generate_embeddings(texts: &[&str]) -> Result<Embeddings> {
  #[cfg(feature = "neural")]
  {
    let mut embeddings = Vec::new();
    let version = "all-MiniLM-L6-v2".to_string();
    
    for text in texts {
      let embedding_vec = get_embedding_from_daemon(text).await?;
      embeddings.push(Embedding::new(version.clone(), embedding_vec));
    }
    
    Ok(Embeddings::new(embeddings))
  }

  #[cfg(not(feature = "neural"))]
  {
    let _ = texts;
    Err(anyhow::anyhow!("Neural features not enabled"))
  }
}

/// Compute and set embedding for an insight (convenience function)
pub async fn compute_for_insight(insight: &mut crate::insight::Insight) -> Result<()> {
  #[cfg(feature = "neural")]
  {
    let embedding_text = insight.get_embedding_text();
    let embedding = generate_embedding(&embedding_text).await?;
    
    insight.set_embedding(
      embedding.version,
      embedding.embedding,
      embedding_text,
    );
    
    Ok(())
  }

  #[cfg(not(feature = "neural"))]
  {
    let _ = insight;
    Ok(())
  }
}

/// Main entry point for getting embeddings from daemon (with fallback)
#[cfg(feature = "neural")]
pub async fn get_embedding_from_daemon(text: &str) -> Result<Vec<f32>> {
  if let Ok(embedding) = request_embedding_from_daemon(text).await {
    return Ok(embedding);
  }

  request_embedding_with_retry(text).await
}

// ==== Daemon Communication Implementation ====

#[cfg(feature = "neural")]
fn create_embedding_request(text: &str) -> EmbeddingRequest {
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
async fn send_request_to_stream(stream: &mut UnixStream, request: &EmbeddingRequest) -> Result<()> {
  let json = serde_json::to_string(request)?;
  stream.write_all(json.as_bytes()).await?;
  stream.write_all(b"\n").await?;
  Ok(())
}

#[cfg(feature = "neural")]
async fn read_response_from_stream(stream: &mut UnixStream) -> Result<EmbeddingResponse> {
  let mut reader = BufReader::new(stream);
  let mut line = String::new();
  reader.read_line(&mut line).await?;

  serde_json::from_str(line.trim()).map_err(|e| anyhow!("Invalid response: {}", e))
}

#[cfg(feature = "neural")]
async fn send_embedding_request(request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
  let mut stream = connect_to_daemon().await?;
  send_request_to_stream(&mut stream, request).await?;
  read_response_from_stream(&mut stream).await
}

#[cfg(feature = "neural")]
fn extract_embedding_from_response(response: EmbeddingResponse) -> Result<Vec<f32>> {
  if let Some(error) = response.error {
    return Err(anyhow!("Daemon error: {}", error));
  }
  Ok(response.embeddings.into_iter().next().unwrap_or_default())
}

#[cfg(feature = "neural")]
async fn request_embedding_from_daemon(text: &str) -> Result<Vec<f32>> {
  let request = create_embedding_request(text);
  let response = send_embedding_request(&request).await?;
  extract_embedding_from_response(response)
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

// ==== Direct ONNX Computation (Fallback) ====

#[cfg(feature = "neural")]
async fn create_embedding_async(text: &str) -> Result<Vec<f32>> {
  if let Ok(embedding) = get_embedding_from_daemon(text).await {
    return Ok(embedding);
  }

  create_embedding_direct(text)
}

/// Initialize ONNX Runtime and load model
#[cfg(feature = "neural")]
fn init_onnx_model() -> Result<ort::session::Session> {
  use ort::session::{builder::GraphOptimizationLevel, Session};

  // Initialize ONNX Runtime (required!)
  ort::init()
    .with_name("blizz")
    .commit()
    .map_err(|e| anyhow!("Failed to initialize ONNX Runtime: {}", e))?;

  // Load model
  Session::builder()
    .map_err(|e| anyhow!("Failed to create session builder: {}", e))?
    .with_optimization_level(GraphOptimizationLevel::Level1)
    .map_err(|e| anyhow!("Failed to set optimization level: {}", e))?
    .with_intra_threads(1)
    .map_err(|e| anyhow!("Failed to set thread count: {}", e))?
    .commit_from_url("https://cdn.pyke.io/0/pyke:ort-rs/example-models@0.0.0/all-MiniLM-L6-v2.onnx")
    .map_err(|e| anyhow!("Failed to load model: {}", e))
}

/// Load and initialize tokenizer
#[cfg(feature = "neural")]
fn load_tokenizer() -> Result<tokenizers::Tokenizer> {
  use std::path::Path;

  let tokenizer_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join("tokenizer.json");
  tokenizers::Tokenizer::from_file(&tokenizer_path)
    .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))
}

/// Tokenize text and prepare input sequences
#[cfg(feature = "neural")]
fn prepare_input_sequences(
  text: &str,
  tokenizer: &tokenizers::Tokenizer,
) -> Result<(Vec<i64>, Vec<i64>)> {
  // Encode the text using the real tokenizer
  let encoding =
    tokenizer.encode(text, false).map_err(|e| anyhow!("Failed to encode text: {}", e))?;

  // Get token IDs and attention mask
  let token_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
  let attention_mask: Vec<i64> =
    encoding.get_attention_mask().iter().map(|&mask| mask as i64).collect();

  Ok((token_ids, attention_mask))
}

/// Pad or truncate sequences to correct length for model
#[cfg(feature = "neural")]
fn pad_sequences(
  token_ids: Vec<i64>,
  attention_mask: Vec<i64>,
  max_length: usize,
) -> (Vec<i64>, Vec<i64>) {
  let mut padded_ids = token_ids;
  let mut padded_mask = attention_mask;

  // Truncate if too long
  if padded_ids.len() > max_length {
    padded_ids.truncate(max_length);
    padded_mask.truncate(max_length);
  }

  // Pad if too short
  while padded_ids.len() < max_length {
    padded_ids.push(0); // PAD token
    padded_mask.push(0); // Attention mask 0 for padding
  }

  (padded_ids, padded_mask)
}

#[cfg(feature = "neural")]
fn extract_sentence_embedding(embeddings: ndarray::ArrayView2<f32>) -> Vec<f32> {
  let embedding_view = embeddings.index_axis(ndarray::Axis(0), 0);
  embedding_view.iter().copied().collect()
}

#[cfg(feature = "neural")]
fn run_inference_and_extract(
  session: &mut ort::session::Session,
  padded_ids: Vec<i64>,
  padded_mask: Vec<i64>,
  max_length: usize,
) -> Result<Vec<f32>> {
  use ort::value::TensorRef;

  let ids_tensor = TensorRef::from_array_view(([1, max_length], &*padded_ids))?;
  let mask_tensor = TensorRef::from_array_view(([1, max_length], &*padded_mask))?;

  let outputs = session.run(ort::inputs![ids_tensor, mask_tensor])?;
  let embedding_output = if outputs.len() > 1 { &outputs[1] } else { &outputs[0] };
  let embeddings =
    embedding_output.try_extract_array::<f32>()?.into_dimensionality::<ndarray::Ix2>()?;

  let embedding_vec = extract_sentence_embedding(embeddings);
  Ok(embedding_vec)
}

/// Direct embedding computation (the current slow method)
#[cfg(feature = "neural")]
fn create_embedding_direct(text: &str) -> Result<Vec<f32>> {
  let mut session = init_onnx_model()?;
  let tokenizer = load_tokenizer()?;
  let (token_ids, attention_mask) = prepare_input_sequences(text, &tokenizer)?;

  const MAX_LENGTH: usize = 512;
  let (padded_ids, padded_mask) = pad_sequences(token_ids, attention_mask, MAX_LENGTH);

  run_inference_and_extract(&mut session, padded_ids, padded_mask, MAX_LENGTH)
}

// ==== Helper Functions for Commands ====

/// Create embedding - now uses daemon for speed!
#[cfg(feature = "neural")]
pub fn create_embedding(_session: &mut ort::session::Session, text: &str) -> Result<Vec<f32>> {
  // Use async runtime to call the new daemon-enabled function
  let rt = tokio::runtime::Runtime::new()?;
  rt.block_on(async { create_embedding_async(text).await })
}

/// Calculate cosine similarity between two embeddings
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
  if a.len() != b.len() {
    return 0.0;
  }

  let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
  let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
  let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

  if magnitude_a == 0.0 || magnitude_b == 0.0 {
    0.0
  } else {
    dot_product / (magnitude_a * magnitude_b)
  }
}