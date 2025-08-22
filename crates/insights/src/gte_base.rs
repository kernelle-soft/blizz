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

impl GTEBase {
    /// Load the GTE-Base model from Hugging Face
    pub async fn load() -> Result<Self> {        
        // Download model files from Hugging Face Hub
        let api = Api::new()?;
        let repo = api.model("thenlper/gte-base".to_string());
        
        bentley::info("Downloading model files...");
        let tokenizer_filename = repo.get("tokenizer.json").await?;
        let model_filename = repo.get("model.onnx").await?;
        
        bentley::info("Loading tokenizer...");
        let tokenizer = Tokenizer::from_file(tokenizer_filename)
            .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;
        
        bentley::info("Loading ONNX model...");
        let session = Session::builder()?
            .commit_from_file(model_filename)?;
        
        bentley::info("✓ embeddings model loaded successfully");
        
        Ok(Self {
            session,
            tokenizer,
        })
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
    
    /// Cleanup resources (called on daemon shutdown)
    pub fn unload(&self) {
        bentley::info("Unloading GTE-Base model...");
        // ONNX session and tokenizer will be dropped automatically
        bentley::info("✓ Model unloaded successfully");
    }
}
