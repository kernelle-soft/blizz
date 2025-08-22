use anyhow::{anyhow, Result};
use hf_hub::api::tokio::Api;
use ndarray::Array2;
use std::collections::HashMap;
use tokenizers::Tokenizer;

#[cfg(target_os = "linux")]
use ort::{
  execution_providers::{CPUExecutionProvider, CUDAExecutionProvider, ExecutionProviderDispatch},
  session::Session,
  value::Value,
};

#[cfg(target_os = "macos")]
use ort::{
  execution_providers::{CPUExecutionProvider, CoreMLExecutionProvider, ExecutionProviderDispatch},
  session::Session,
  value::Value,
};

pub struct GTEBase {
  session: Session,
  tokenizer: Tokenizer,
}

struct ModelFiles {
  tokenizer_file: std::path::PathBuf,
  model_path: std::path::PathBuf,
}

// Public API
impl GTEBase {
  /// Load the GTE-Base model from HuggingFace
  pub async fn load() -> Result<Self> {
    bentley::info("Loading GTE-Base model...");
    
    let model_files = Self::download_model().await?;
    let tokenizer = Self::load_tokenizer(model_files.tokenizer_file)?;
    let session = Self::create_session(model_files.model_path)?;
    
    bentley::info("✓ GTE-Base model loaded successfully");
    Ok(Self { session, tokenizer })
  }

  /// Generate embeddings for a single text
  pub fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
    let tokens = Self::tokenize(text, &self.tokenizer)?;
    let input = Self::prepare(&tokens, &self.session)?;
    let output = self.session.run(input)?;
    Self::extract_embedding(output)
  }
}

// Model initialization
impl GTEBase {
  /// Download required model files from HuggingFace
  async fn download_model() -> Result<ModelFiles> {
    let api = Api::new().map_err(|e| anyhow!("HF API initialization failed: {}", e))?;
    let repo = api.model("Alibaba-NLP/gte-base-en-v1.5".to_string());

    let tokenizer_file = repo
      .get("tokenizer.json")
      .await
      .map_err(|e| anyhow!("Failed to download tokenizer: {}", e))?;

    let model_path = repo
      .get("onnx/model.onnx")
      .await
      .map_err(|e| anyhow!("Failed to download ONNX model: {}", e))?;

    Ok(ModelFiles { tokenizer_file, model_path })
  }

  /// Load the tokenizer from the downloaded file
  fn load_tokenizer(tokenizer_file: std::path::PathBuf) -> Result<Tokenizer> {
    Tokenizer::from_file(tokenizer_file)
      .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))
  }

  /// Create the ONNX Runtime session with hardware-optimized execution providers
  fn create_session(model_path: std::path::PathBuf) -> Result<Session> {
    let providers = Self::get_execution_providers();
    
    let session = Session::builder()?.with_execution_providers(providers)?.commit_from_file(model_path)?;    
    Ok(session)
  }
}

// Hardware detection
impl GTEBase {
  /// Detect and configure the best available execution providers for the current platform
  fn get_execution_providers() -> Vec<ExecutionProviderDispatch> {
    let mut providers = Vec::new();

    // Platform-specific hardware acceleration
    #[cfg(target_os = "macos")]
    {
      providers.push(CoreMLExecutionProvider::default().into());
    }

    #[cfg(target_os = "linux")]
    {
      if Self::is_cuda_available() {
        bentley::info("CUDA detected - adding CUDA provider");
        providers.push(CUDAExecutionProvider::default().build().error_on_failure());
      }
    }

    // Always fallback to CPU
    providers.push(CPUExecutionProvider::default().into());

    providers
  }

  /// Check if CUDA is available using ONNX Runtime's ExecutionProvider::is_available()
  #[cfg(target_os = "linux")]
  fn is_cuda_available() -> bool {
    // First check if nvidia-smi exists (hardware level)

    std::process::Command::new("nvidia-smi")
      .output()
      .map(|output| output.status.success())
      .unwrap_or(false)
  }
}

// Embedding processing
impl GTEBase {
  /// Tokenize input text and validate length constraints
  fn tokenize(text: &str, tokenizer: &Tokenizer) -> Result<tokenizers::Encoding> {
    let tokens = tokenizer.encode(text, true).map_err(|e| anyhow!("Tokenization failed: {}", e))?;
    
    let input_ids = tokens.get_ids();
    let token_count = input_ids.len();
    const MAX_SEQUENCE_LENGTH: usize = 511; // GTE-Base limit is 512
    
    bentley::info(&format!(
      "Token count check: {token_count} tokens (limit: {MAX_SEQUENCE_LENGTH})"
    ));
    
    if token_count > MAX_SEQUENCE_LENGTH {
      let error_msg = format!(
        "Input text contains {token_count} tokens, which exceeds the model's maximum sequence length of {MAX_SEQUENCE_LENGTH}. Please reduce the input size."
      );
      bentley::warn(&error_msg);
      return Err(anyhow!(error_msg));
    }
    
    bentley::info(&format!(
      "✓ Processing {token_count} tokens (within {MAX_SEQUENCE_LENGTH} limit)"
    ));
    
    Ok(tokens)
  }

  /// Prepare model input tensors from tokenization tokens
  fn prepare(tokens: &tokenizers::Encoding, session: &Session) -> Result<std::collections::HashMap<String, Value>> {
    let input_ids = tokens.get_ids();
    let attention_mask = tokens.get_attention_mask();
    let token_type_ids = tokens.get_type_ids();
    
    // Convert to the format expected by ort v2
    let input_ids: Vec<i64> = input_ids.iter().map(|&x| x as i64).collect();
    let attention_mask: Vec<i64> = attention_mask.iter().map(|&x| x as i64).collect();
    let token_type_ids: Vec<i64> = token_type_ids.iter().map(|&x| x as i64).collect();
    
    let seq_len = input_ids.len();
    
    // Create ndarray arrays and convert to ort Values
    let input_ids_array: Array2<i64> = Array2::from_shape_vec((1, seq_len), input_ids)?;
    let attention_mask_array: Array2<i64> = Array2::from_shape_vec((1, seq_len), attention_mask)?;
    let token_type_ids_array: Array2<i64> = Array2::from_shape_vec((1, seq_len), token_type_ids)?;
    
    let input_ids_tensor: Value = Value::from_array(input_ids_array)?.into();
    let attention_mask_tensor: Value = Value::from_array(attention_mask_array)?.into();
    let token_type_ids_tensor: Value = Value::from_array(token_type_ids_array)?.into();
    
    // Create input based on what the model expects
    let mut input = HashMap::new();
    input.insert("input_ids".to_string(), input_ids_tensor);
    input.insert("attention_mask".to_string(), attention_mask_tensor);
    
    // Only include token_type_ids if the model expects it
    let model_input_names: Vec<String> =
      session.inputs.iter().map(|input| input.name.to_string()).collect();
    
    if model_input_names.contains(&"token_type_ids".to_string()) {
      input.insert("token_type_ids".to_string(), token_type_ids_tensor);
    } else {
      bentley::verbose("Model doesn't expect token_type_ids, skipping");
    }
    
    Ok(input)
  }

  /// Extract and process embeddings from model output  
  fn extract_embedding<'s>(output: ort::session::SessionOutputs<'s>) -> Result<Vec<f32>> {
    let tensor = output
      .get("last_hidden_state")
      .or_else(|| output.get("0"))
      .ok_or_else(|| anyhow!("No output found from model"))?;

    let (shape, data) = tensor.try_extract_tensor::<f32>()?;
    let shape_slice = shape.as_ref();

    Self::mean_pool((shape_slice, data))
  }

  /// Perform mean pooling over sequence dimension for sentence embeddings
  fn mean_pool(embedding: (&[i64], &[f32])) -> Result<Vec<f32>> {
    let (shape, data) = embedding;
    
    let seq_length = shape[1] as usize;
    let hidden_size = shape[2] as usize;
    
    let mut embedding = vec![0.0f32; hidden_size];
    for token_idx in 0..seq_length {
      let start = token_idx * hidden_size;
      let end = start + hidden_size;
      for (i, &value) in data[start..end].iter().enumerate() {
        embedding[i] += value;
      }
    }

    for value in embedding.iter_mut() {
      *value /= seq_length as f32;
    }

    Ok(embedding)
  }
}
