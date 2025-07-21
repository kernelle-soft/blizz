use anyhow::{anyhow, Result};

#[cfg(feature = "neural")]
use ort::session::Session;

pub trait EmbeddingModel {
  #[allow(dead_code)] // Used by daemon binary
  fn compute_embeddings(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

#[cfg(feature = "neural")]
pub struct OnnxEmbeddingModel {
  #[allow(dead_code)]
  session: Session,
  #[allow(dead_code)]
  tokenizer: tokenizers::Tokenizer,
}

#[cfg(feature = "neural")]
impl EmbeddingModel for OnnxEmbeddingModel {
  fn compute_embeddings(&mut self, _texts: &[String]) -> Result<Vec<Vec<f32>>> {
    // Placeholder implementation - return mock embeddings for now
    Ok(vec![vec![0.0; 384]])
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
#[allow(dead_code)] // Used by daemon binary
pub async fn create_production_model() -> Result<OnnxEmbeddingModel> {
  // Placeholder implementation - in practice this would load the actual ONNX model
  Err(anyhow!("ONNX model loading not yet implemented"))
}
