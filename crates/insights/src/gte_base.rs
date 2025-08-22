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

impl GTEBase {
  /// Load the GTE-Base model from HuggingFace
  pub async fn load() -> Result<Self> {
    bentley::info("Loading GTE-Base model...");
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

    let tokenizer = Tokenizer::from_file(tokenizer_file)
      .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;

    // Configure hardware-specific execution providers
    let providers = Self::get_execution_providers();
    bentley::info(&format!(
      "Using execution providers: {:?}",
      providers.iter().map(|p| format!("{p:?}")).collect::<Vec<_>>()
    ));

    let session =
      Session::builder()?.with_execution_providers(providers)?.commit_from_file(model_path)?;

    // Debug: Log the expected inputs
    let inputs = &session.inputs;
    bentley::info(&format!("Model expects {} inputs:", inputs.len()));
    for (i, input) in inputs.iter().enumerate() {
      bentley::info(&format!("  Input {}: {} (shape: {:?})", i, input.name, input.input_type));
    }

    bentley::info("✓ GTE-Base model loaded successfully");
    Ok(Self { session, tokenizer })
  }

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

  /// Generate embeddings for a single text
  pub fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
    bentley::verbose(&format!(
      "Generating embedding for: {}",
      text.chars().take(50).collect::<String>()
    ));

    // Tokenize the input text (without truncation to detect overlong inputs)
    let encoding =
      self.tokenizer.encode(text, true).map_err(|e| anyhow!("Tokenization failed: {}", e))?;

    // Check if input exceeds model's maximum sequence length
    let input_ids = encoding.get_ids();
    let token_count = input_ids.len();
    const MAX_SEQUENCE_LENGTH: usize = 511; // GTE-Base limit is 512. Since tokenization truncates longer, we reject at 511 to guarantee it's under the limit.

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

    // Create inputs based on what the model expects
    let mut inputs = HashMap::new();
    inputs.insert("input_ids", input_ids_tensor);
    inputs.insert("attention_mask", attention_mask_tensor);

    // Only include token_type_ids if the model expects it
    let model_input_names: Vec<String> =
      self.session.inputs.iter().map(|input| input.name.to_string()).collect();

    if model_input_names.contains(&"token_type_ids".to_string()) {
      inputs.insert("token_type_ids", token_type_ids_tensor);
    } else {
      bentley::verbose("Model doesn't expect token_type_ids, skipping");
    }

    // Run inference
    let outputs = self.session.run(inputs)?;

    // Debug: Log available outputs
    bentley::verbose(&format!("Model returned {} outputs:", outputs.len()));
    for (name, tensor) in outputs.iter() {
      bentley::verbose(&format!("  Output: {} (shape: {:?})", name, tensor.shape()));
    }

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
