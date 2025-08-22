use anyhow::{anyhow, Result};
use hf_hub::api::tokio::Api;
use ndarray::Array2;
use ort::{session::Session, value::Value};
use std::collections::HashMap;
use tokenizers::Tokenizer;

pub struct GTEBase {
    session: Session,
    tokenizer: Tokenizer,
}

// #[derive(Debug, thiserror::Error)]
// enum Errors {
//   #[error("HF API initialization failed: {0}")]
//   HfApiInitializationFailed(String),

//   #[error("Failed to download tokenizer: {0}")]
//   FailedToDownloadTokenizer(String),
  
//   #[error("Failed to download ONNX model: {0}")]
//   FailedToDownloadOnnxModel(String),
  
//   #[error("Failed to load tokenizer: {0}")]
//   FailedToLoadTokenizer(String),
// }

impl GTEBase {
    /// Load the GTE-Base model from HuggingFace
    pub async fn load() -> Result<Self> {
      bentley::info("Loading GTE-Base model...");
      let api = Api::new()
        .map_err(|e| anyhow!("HF API initialization failed: {}", e))?;

      let repo = api.model("Alibaba-NLP/gte-base-en-v1.5".to_string());

      let tokenizer_file = repo
        .get("tokenizer.json").await
        .map_err(|e| anyhow!("Failed to download tokenizer: {}", e))?;

      let model_path = repo
        .get("onnx/model.onnx").await
        .map_err(|e| anyhow!("Failed to download ONNX model: {}", e))?;

      let tokenizer = Tokenizer::from_file(tokenizer_file)
        .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;

      let session = Session::builder()?
        .commit_from_file(model_path)?;

      Ok(Self {session, tokenizer})
    }
    
    /// Generate embeddings for a single text
    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
      bentley::verbose(&format!("Generating embedding for: {}", 
          text.chars().take(50).collect::<String>()));
      
      // Tokenize the input text
      let encoding = self.tokenizer
          .encode(text, true)
          .map_err(|e| anyhow!("Tokenization failed: {}", e))?;
      
      let input_ids = encoding.get_ids();
      let attention_mask = encoding.get_attention_mask();
      let token_type_ids = encoding.get_type_ids();
      
      // Convert to the format expected by ort v2
      let input_ids: Vec<i64> = input_ids.iter().map(|&x| x as i64).collect();
      let attention_mask: Vec<i64> = attention_mask.iter().map(|&x| x as i64).collect();
      let token_type_ids: Vec<i64> = token_type_ids.iter().map(|&x| x as i64).collect();
      
      let seq_len = input_ids.len();
      
      // Create ndarray arrays and convert to ort Values
      let input_ids_array: Array2<i64> = Array2::from_shape_vec((1, seq_len), input_ids)?;
      let attention_mask_array: Array2<i64> = Array2::from_shape_vec((1, seq_len), attention_mask)?;
      let token_type_ids_array: Array2<i64> = Array2::from_shape_vec((1, seq_len), token_type_ids)?;
      
      let input_ids_tensor = Value::from_array(input_ids_array)?;
      let attention_mask_tensor = Value::from_array(attention_mask_array)?;
      let token_type_ids_tensor = Value::from_array(token_type_ids_array)?;
      
      // Create inputs
      let inputs = HashMap::from([
          ("input_ids", input_ids_tensor),
          ("attention_mask", attention_mask_tensor),
          ("token_type_ids", token_type_ids_tensor),
      ]);
      
      // Run inference
      let outputs = self.session.run(inputs)?;
      
      // Extract the last hidden state output
      let embedding = outputs
          .get("last_hidden_state")
          .or_else(|| outputs.get("0"))
          .ok_or_else(|| anyhow!("No output found from model"))?
          .try_extract_tensor::<f32>()?;
      
      // Perform mean pooling over sequence dimension (typical for sentence embeddings)
      // Shape should be [batch_size, sequence_length, hidden_size]
      let (shape, data) = embedding;
      
      let seq_length = shape[1] as usize; 
      let hidden_size = shape[2] as usize;
      
      // Mean pool over the sequence dimension for the first batch
      let mut embedding_vec = vec![0.0f32; hidden_size];
      for token_idx in 0..seq_length {
          let start_idx = token_idx * hidden_size;
          let end_idx = start_idx + hidden_size;
          for (i, &value) in data[start_idx..end_idx].iter().enumerate() {
              embedding_vec[i] += value;
          }
      }
      
      // Average the pooled values
      for value in embedding_vec.iter_mut() {
          *value /= seq_length as f32;
      }
      
      bentley::verbose(&format!("Generated {}-dimensional embedding", embedding_vec.len()));
      
      Ok(embedding_vec)
    }
}
