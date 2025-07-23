use anyhow::{anyhow, Result};
use std::path::Path;

#[cfg(feature = "neural")]
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::{session::SessionOutputs, value::Tensor};

pub trait EmbeddingModel {
  #[allow(dead_code)]
  fn compute_embeddings(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

#[cfg(feature = "neural")]
pub struct OnnxEmbeddingModel {
  session: Session,
  tokenizer: tokenizers::Tokenizer,
}

#[cfg(feature = "neural")]
impl EmbeddingModel for OnnxEmbeddingModel {
  fn compute_embeddings(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
    compute_onnx_embeddings(self, texts)
  }
}

pub struct MockEmbeddingModel {
  pub fail_on_texts: Vec<String>,
  pub response_embeddings: Vec<Vec<f32>>,
}

impl MockEmbeddingModel {
  pub fn new() -> Self {
    Self {
      fail_on_texts: vec![],
      response_embeddings: vec![vec![0.1, 0.2, 0.3]; 10], // Default mock embeddings
    }
  }
}

impl Default for MockEmbeddingModel {
  fn default() -> Self {
    Self::new()
  }
}

impl EmbeddingModel for MockEmbeddingModel {
  fn compute_embeddings(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
    // Check if we should fail for any of these texts
    for text in texts {
      if self.fail_on_texts.contains(text) {
        return Err(anyhow!("Mock failure for text: {}", text));
      }
    }

    // Return mock embeddings
    let mut result = Vec::new();
    for (i, _text) in texts.iter().enumerate() {
      let embedding_index = i % self.response_embeddings.len();
      result.push(self.response_embeddings[embedding_index].clone());
    }

    Ok(result)
  }
}

#[cfg(feature = "neural")]
#[allow(dead_code)]
pub async fn create_production_model() -> Result<OnnxEmbeddingModel> {
  initialize_onnx_runtime()?;
  let session = create_model_session()?;
  let tokenizer = load_tokenizer()?;

  Ok(OnnxEmbeddingModel { session, tokenizer })
}

#[cfg(feature = "neural")]
#[allow(dead_code)] // Used by daemon binary
fn initialize_onnx_runtime() -> Result<()> {
  ort::init()
    .with_name("blizz-model")
    .commit()
    .map_err(|e| anyhow!("Failed to initialize ONNX Runtime: {}", e))
    .map(|_| ())
}

#[cfg(feature = "neural")]
#[allow(dead_code)] // Used by daemon binary
fn create_model_session() -> Result<Session> {
  // Try to load from URL if local file doesn't exist
  let local_model_path = Path::new("all-MiniLM-L6-v2.onnx");

  let session = if local_model_path.exists() {
    Session::builder()?
      .with_optimization_level(GraphOptimizationLevel::Level1)?
      .commit_from_file(local_model_path)
      .map_err(|e| anyhow!("Failed to load local ONNX model: {}", e))?
  } else {
    // Load from remote URL
    Session::builder()?
      .with_optimization_level(GraphOptimizationLevel::Level1)?
      .commit_from_url("https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx")
      .map_err(|e| anyhow!("Failed to load ONNX model from URL: {}", e))?
  };

  Ok(session)
}

#[cfg(feature = "neural")]
#[allow(dead_code)] // Used by daemon binary
fn load_tokenizer() -> Result<tokenizers::Tokenizer> {
  // Load tokenizer from embedded data - no files, no downloads, no dependencies!
  let tokenizer_bytes = include_bytes!("../data/tokenizer.json");
  tokenizers::Tokenizer::from_bytes(tokenizer_bytes)
    .map_err(|e| anyhow!("Failed to load embedded tokenizer: {}", e))
}

#[cfg(feature = "neural")]
#[allow(dead_code)] // Used by daemon binary
pub fn compute_onnx_embeddings(
  model: &mut OnnxEmbeddingModel,
  texts: &[String],
) -> Result<Vec<Vec<f32>>> {
  if texts.is_empty() {
    return Ok(vec![]);
  }

  validate_inputs(texts)?;

  let encodings = tokenize_texts(&mut model.tokenizer, texts)?;
  let (ids, mask, token_type_ids, batch, length) = batch_tokens(&encodings);

  // Create input tensors
  let ids = Tensor::from_array(([batch, length], ids.into_boxed_slice()))?;
  let mask = Tensor::from_array(([batch, length], mask.into_boxed_slice()))?;
  let token_type_ids = Tensor::from_array(([batch, length], token_type_ids.into_boxed_slice()))?;

  // Run inference with all required inputs
  let outputs = model.session.run(ort::inputs![
    "input_ids" => ids,
    "attention_mask" => mask,
    "token_type_ids" => token_type_ids
  ])?;

  // Extract embeddings from the output
  let output = get_session_outputs(&outputs)?;

  let (_shape, data) = output.try_extract_tensor::<f32>()?;

  let results = extract_embeddings(data, texts.len());
  Ok(results)
}

fn get_session_outputs<'a>(outputs: &'a SessionOutputs<'_>) -> Result<&'a ort::value::Value> {
  let output = outputs
    .get("last_hidden_state")
    .or_else(|| outputs.get("output_0"))
    .or_else(|| outputs.get("logits"))
    .ok_or_else(|| {
      anyhow!(
        "No output tensor found - available outputs: {:?}",
        outputs.keys().collect::<Vec<_>>()
      )
    })?;

  Ok(output)
}

#[cfg(feature = "neural")]
#[allow(dead_code)] // Used by daemon binary
fn tokenize_texts(
  tokenizer: &mut tokenizers::Tokenizer,
  texts: &[String],
) -> Result<Vec<tokenizers::Encoding>> {
  let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
  tokenizer.encode_batch(text_refs, true).map_err(|e| anyhow!("Failed to encode texts: {}", e))
}

#[cfg(feature = "neural")]
#[allow(dead_code)] // Used by daemon binary
fn batch_tokens(
  encodings: &[tokenizers::Encoding],
) -> (Vec<i64>, Vec<i64>, Vec<i64>, usize, usize) {
  let batch = encodings.len();
  let length = encodings.iter().map(|e| e.len()).max().unwrap_or(0);

  let mut ids = Vec::with_capacity(batch * length);
  let mut mask = Vec::with_capacity(batch * length);
  let mut token_type_ids = Vec::with_capacity(batch * length);

  for encoding in encodings {
    let encoding_ids = encoding.get_ids();
    let encoding_mask = encoding.get_attention_mask();

    // Pad to max length
    for i in 0..length {
      if i < encoding_ids.len() {
        ids.push(encoding_ids[i] as i64);
        mask.push(encoding_mask[i] as i64);
        token_type_ids.push(0); // All zeros for sentence transformers
      } else {
        ids.push(0); // PAD token
        mask.push(0); // No attention
        token_type_ids.push(0); // PAD token type
      }
    }
  }

  (ids, mask, token_type_ids, batch, length)
}

#[cfg(feature = "neural")]
#[allow(dead_code)] // Used by daemon binary
fn extract_embeddings(data: &[f32], batch_size: usize) -> Vec<Vec<f32>> {
  let mut results = Vec::new();

  // For now, assume the data is already in [batch, features] format (384-dim embeddings)
  // This works for most sentence transformer models that output pooled embeddings
  let features = data.len() / batch_size.max(1);

  for i in 0..batch_size {
    let start = i * features;
    let end = start + features;
    if end <= data.len() {
      let vector: Vec<f32> = data[start..end].to_vec();
      let normalized = normalize_vector(vector);
      results.push(normalized);
    } else {
      // Fallback: create a zero vector if dimensions don't match
      results.push(vec![0.0; 384]);
    }
  }

  results
}

#[cfg(feature = "neural")]
#[allow(dead_code)] // Used by daemon binary
fn normalize_vector(vector: Vec<f32>) -> Vec<f32> {
  let magnitude: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
  if magnitude > 0.0 {
    vector.into_iter().map(|x| x / magnitude).collect()
  } else {
    vector
  }
}

fn validate_inputs(texts: &[String]) -> Result<()> {
  if texts.is_empty() {
    return Err(anyhow!("Input texts cannot be empty"));
  }

  for text in texts {
    if text.len() > 8192 {
      return Err(anyhow!("Text too long: {} characters (max 8192)", text.len()));
    }
  }

  Ok(())
}
