use anyhow::{anyhow, Result};
use std::path::Path;

#[cfg(feature = "neural")]
use ort::session::{builder::GraphOptimizationLevel, Session};
#[cfg(feature = "neural")]
use tokenizers::Tokenizer;

pub trait EmbeddingModel {
  fn compute_embeddings(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

#[cfg(feature = "neural")]
pub struct OnnxEmbeddingModel {
  session: Session,
  tokenizer: Tokenizer,
}

pub struct MockEmbeddingModel {
  pub fail_on_texts: Vec<String>,
  pub response_embeddings: Vec<Vec<f32>>,
}

#[cfg(feature = "neural")]
pub async fn create_model() -> Result<OnnxEmbeddingModel> {
  initialize_onnx_runtime()?;
  let session = create_model_session()?;
  let tokenizer = load_tokenizer()?;

  Ok(OnnxEmbeddingModel { session, tokenizer })
}

#[cfg(feature = "neural")]
fn initialize_onnx_runtime() -> Result<()> {
  ort::init()
    .with_name("blizz-model")
    .commit()
    .map_err(|e| anyhow!("Failed to initialize ONNX Runtime: {}", e))
    .map(|_| ())
}

#[cfg(feature = "neural")]
fn create_model_session() -> Result<Session> {
  let session = Session::builder()
    .map_err(|e| anyhow!("Failed to create session builder: {}", e))?
    .with_optimization_level(GraphOptimizationLevel::Level1)
    .map_err(|e| anyhow!("Failed to set optimization level: {}", e))?
    .with_intra_threads(1)
    .map_err(|e| anyhow!("Failed to set thread count: {}", e))?
    .commit_from_url("https://cdn.pyke.io/0/pyke:ort-rs/example-models@0.0.0/all-MiniLM-L6-v2.onnx")
    .map_err(|e| anyhow!("Failed to load model: {}", e))?;

  Ok(session)
}

#[cfg(feature = "neural")]
fn load_tokenizer() -> Result<Tokenizer> {
  let tokenizer_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join("tokenizer.json");

  Tokenizer::from_file(&tokenizer_path).map_err(|e| anyhow!("Failed to load tokenizer: {}", e))
}

#[cfg(feature = "neural")]
pub fn compute_onnx_embeddings(model: &mut OnnxEmbeddingModel, texts: &[String]) -> Result<Vec<Vec<f32>>> {
  if texts.is_empty() {
    return Ok(vec![]);
  }

  let encodings = tokenize_texts(&mut model.tokenizer, texts)?;
  let (ids, mask, batch, length) = convert_encodings_to_tensor_data(&encodings);

  let ids_tensor = ort::value::TensorRef::from_array_view(([batch, length], &*ids))?;
  let mask_tensor = ort::value::TensorRef::from_array_view(([batch, length], &*mask))?;

  let outputs = model.session.run(ort::inputs![ids_tensor, mask_tensor])?;
  let output = if outputs.len() > 1 { &outputs[1] } else { &outputs[0] };
  let embeddings = output.try_extract_array::<f32>()?.into_dimensionality::<ndarray::Ix2>()?;

  let results = extract_embedding_vectors(embeddings, texts.len());
  Ok(results)
}

#[cfg(feature = "neural")]
fn tokenize_texts(
  tokenizer: &mut Tokenizer,
  texts: &[String],
) -> Result<Vec<tokenizers::Encoding>> {
  let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
  tokenizer.encode_batch(text_refs, true).map_err(|e| anyhow!("Failed to encode texts: {}", e))
}

#[cfg(feature = "neural")]
fn convert_encodings_to_tensor_data(
  encodings: &[tokenizers::Encoding],
) -> (Vec<i64>, Vec<i64>, usize, usize) {
  let batch = encodings.len();
  let length = encodings[0].len();

  let ids: Vec<i64> =
    encodings.iter().flat_map(|e| e.get_ids().iter().map(|&id| id as i64)).collect();

  let mask: Vec<i64> =
    encodings.iter().flat_map(|e| e.get_attention_mask().iter().map(|&mask| mask as i64)).collect();

  (ids, mask, batch, length)
}

#[cfg(feature = "neural")]
fn extract_embedding_vectors(
  embeddings: ndarray::ArrayView2<f32>,
  batch_size: usize,
) -> Vec<Vec<f32>> {
  let mut results = Vec::new();
  for i in 0..batch_size {
    let view = embeddings.index_axis(ndarray::Axis(0), i);
    let vector: Vec<f32> = view.iter().copied().collect();
    results.push(vector);
  }
  results
}

pub fn create_mock_model() -> MockEmbeddingModel {
  MockEmbeddingModel {
    fail_on_texts: vec![],
    response_embeddings: vec![vec![0.1, 0.2, 0.3]; 10], // Default mock embeddings
  }
}

pub fn with_failure_on(mut model: MockEmbeddingModel, text: String) -> MockEmbeddingModel {
  model.fail_on_texts.push(text);
  model
}

pub fn with_embeddings(mut model: MockEmbeddingModel, embeddings: Vec<Vec<f32>>) -> MockEmbeddingModel {
  model.response_embeddings = embeddings;
  model
}

pub fn compute_mock_embeddings(model: &mut MockEmbeddingModel, texts: &[String]) -> Result<Vec<Vec<f32>>> {
  // Check if we should fail for any of these texts
  for text in texts {
    if model.fail_on_texts.contains(text) {
      return Err(anyhow!("Mock failure for text: {}", text));
    }
  }

  // Return mock embeddings (cycle through available ones)
  let mut result = Vec::new();
  for (i, _text) in texts.iter().enumerate() {
    let embedding_index = i % model.response_embeddings.len();
    result.push(model.response_embeddings[embedding_index].clone());
  }

  Ok(result)
}

pub fn compute_embeddings(model: &mut dyn EmbeddingModel, texts: &[String]) -> Result<Vec<Vec<f32>>> {
  model.compute_embeddings(texts)
}

// Trait implementations for compatibility
#[cfg(feature = "neural")]
impl EmbeddingModel for OnnxEmbeddingModel {
  fn compute_embeddings(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
    compute_onnx_embeddings(self, texts)
  }
}

impl EmbeddingModel for MockEmbeddingModel {
  fn compute_embeddings(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
    compute_mock_embeddings(self, texts)
  }
}

impl Default for MockEmbeddingModel {
  fn default() -> Self {
    create_mock_model()
  }
}

#[cfg(feature = "neural")]
pub async fn create_production_model() -> Result<OnnxEmbeddingModel> {
  create_model().await
}