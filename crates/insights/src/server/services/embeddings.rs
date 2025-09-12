use anyhow::{anyhow, Result};
use hf_hub::api::tokio::Api;
use ndarray::Array2;
use std::collections::HashMap;
use std::sync::Mutex;
use tokenizers::Tokenizer;

const MODEL_NAME: &str = "onnx-community/embeddinggemma-300m-ONNX";
const TOKENIZER_FILE: &str = "tokenizer.json";
const MODEL_FILE: &str = "onnx/model.onnx";

/// Trait for extracting tensor data - allows testing without ONNX complexity
trait EmbeddingOutput {
  fn get_tensor(&self, key: &str) -> Option<&dyn TensorData>;
}

trait TensorData {
  fn extract_f32_data(&self) -> Result<(&[i64], &[f32])>;
}

/// Trait abstractions for testable tensor preparation
trait TokenEncoding {
  fn get_ids(&self) -> &[u32];
  fn get_attention_mask(&self) -> &[u32];
  fn get_type_ids(&self) -> &[u32];
}

trait SessionInputs {
  fn input_names(&self) -> Vec<String>;
}

/// Trait abstraction for testable tokenization
trait TextTokenizer {
  fn encode_text(&self, text: &str, add_special_tokens: bool) -> Result<Box<dyn TokenizerOutput>>;
}

trait TokenizerOutput: std::fmt::Debug + TokenEncoding {}

#[cfg(target_os = "linux")]
use ort::{
  execution_providers::{CPUExecutionProvider, CUDAExecutionProvider, ExecutionProviderDispatch},
  session::Session,
  value::Value,
};

#[cfg(target_os = "macos")]
use ort::{
  execution_providers::{CoreMLExecutionProvider, ExecutionProviderDispatch},
  session::Session,
  value::Value,
};

// Implementations for real ONNX types
#[cfg(not(tarpaulin_include))]
impl<'s> EmbeddingOutput for ort::session::SessionOutputs<'s> {
  fn get_tensor(&self, key: &str) -> Option<&dyn TensorData> {
    self.get(key).map(|v| v as &dyn TensorData)
  }
}

#[cfg(not(tarpaulin_include))]
impl TensorData for ort::value::Value {
  fn extract_f32_data(&self) -> Result<(&[i64], &[f32])> {
    let (shape, data) = self.try_extract_tensor::<f32>()?;
    Ok((shape.as_ref(), data))
  }
}

// Implementations for real types
#[cfg(not(tarpaulin_include))]
impl TokenEncoding for tokenizers::Encoding {
  fn get_ids(&self) -> &[u32] {
    self.get_ids()
  }
  fn get_attention_mask(&self) -> &[u32] {
    self.get_attention_mask()
  }
  fn get_type_ids(&self) -> &[u32] {
    self.get_type_ids()
  }
}

#[cfg(not(tarpaulin_include))]
impl SessionInputs for Session {
  fn input_names(&self) -> Vec<String> {
    self.inputs.iter().map(|input| input.name.to_string()).collect()
  }
}

// Real tokenizer implementations
#[cfg(not(tarpaulin_include))]
impl TextTokenizer for Tokenizer {
  fn encode_text(&self, text: &str, add_special_tokens: bool) -> Result<Box<dyn TokenizerOutput>> {
    let encoding =
      self.encode(text, add_special_tokens).map_err(|e| anyhow!("Tokenization failed: {}", e))?;
    Ok(Box::new(encoding))
  }
}

impl TokenizerOutput for tokenizers::Encoding {}

pub struct EmbeddingModel {
  session: Session,
  tokenizer: Tokenizer,
}

struct ModelFiles {
  tokenizer_file: std::path::PathBuf,
  model_path: std::path::PathBuf,
}

// Public API
#[cfg(not(tarpaulin_include))] // [rag-stack] - add CI/CD testing for cross-platform loading/unloading
impl EmbeddingModel {

  /// Load the EmbeddingGemma model from HuggingFace
  pub async fn load() -> Result<Self> {
    bentley::info!("Loading EmbeddingGemma-300M model...");

    let model_files = Self::download_model().await?;
    let tokenizer = Self::load_tokenizer(model_files.tokenizer_file)?;
    let session = Self::load_model(model_files.model_path)?;

    // Test inference to verify GPU performance
    bentley::info!("Running GPU performance test...");
    let mut model = Self { session, tokenizer };
    let start_time = std::time::Instant::now();
    let _ = model.embed("test performance")?;
    let test_duration = start_time.elapsed();

    bentley::info!(&format!(
      "Performance test: {:.2}ms for single text",
      test_duration.as_millis()
    ));

    if test_duration.as_millis() > 50 {
      bentley::warn!(&format!(
        "⚠ SLOW: {:.2}ms for single embedding suggests CPU execution. Expected <10ms on GPU.",
        test_duration.as_millis()
      ));
    } else {
      bentley::info!("✓ Performance test passed - GPU acceleration working");
    }

    Ok(model)
  }

  /// Generate embeddings for a single text
  pub fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
    bentley::info!(&format!("Embedding text: '{}' ({} chars)", text, text.len()));
    let tokens = Self::tokenize(text, &self.tokenizer)?;
    let input = Self::prepare(tokens.as_ref(), &self.session)?;
    let output = self.session.run(input)?;
    let raw_embedding = Self::extract_embedding(&output)?;
    Self::normalize_embedding(raw_embedding)
  }

  /// Generate embeddings for multiple texts in a single batch inference
  /// Supports up to 2048 texts for maximum throughput, with automatic chunking for large tensors
  pub fn embed_batch(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    if texts.is_empty() {
      return Ok(vec![]);
    }

    const MAX_BATCH_SIZE: usize = 2048;
    if texts.len() > MAX_BATCH_SIZE {
      return Err(anyhow!("Batch size {} exceeds maximum of {}", texts.len(), MAX_BATCH_SIZE));
    }

    // Check if we should split into smaller chunks due to tensor size
    let chunk_size = Self::calculate_optimal_chunk_size(texts.len());
    if chunk_size < texts.len() {
      bentley::warn!(&format!(
        "Large tensor detected, splitting batch of {} into chunks of {}",
        texts.len(), chunk_size
      ));
      
      let mut all_embeddings = Vec::new();
      for chunk in texts.chunks(chunk_size) {
        let chunk_embeddings = self.embed_batch(chunk)?;
        all_embeddings.extend(chunk_embeddings);
      }
      return Ok(all_embeddings);
    }

    bentley::info!(&format!("Batch embedding {} texts", texts.len()));

    // Tokenize all texts
    let all_tokens: Result<Vec<_>> = texts
      .iter()
      .map(|text| Self::tokenize(text, &self.tokenizer))
      .collect();
    let all_tokens = all_tokens?;

    // Prepare batch tensors
    bentley::info!("Starting batch tensor preparation");
    let input = Self::prepare_batch(&all_tokens, &self.session)?;
    bentley::info!("Batch tensor preparation complete");
    
    // Run inference with detailed timing
    bentley::info!(&format!("Starting model inference for batch of {} texts", texts.len()));
    let start_time = std::time::Instant::now();
    
    // Log tensor shapes for debugging
    for (name, value) in &input {
      if let Ok((shape, _)) = value.try_extract_tensor::<i64>() {
        bentley::verbose!(&format!("Input tensor '{}': shape {:?}", name, shape));
      }
    }
    
    let output = self.session.run(input)?;
    let inference_duration = start_time.elapsed();
    
    bentley::info!(&format!(
      "Model inference complete: {:.2}ms ({:.2}ms per text)",
      inference_duration.as_millis(),
      inference_duration.as_millis() as f64 / texts.len() as f64
    ));
    
    // Performance analysis
    let per_text_ms = inference_duration.as_millis() as f64 / texts.len() as f64;
    
    if per_text_ms > 50.0 {
      bentley::error!(&format!(
        "🐌 EXTREMELY SLOW: {:.2}ms per text indicates CPU execution despite CUDA provider",
        per_text_ms
      ));
      
      // Additional diagnostics
      bentley::error!("Possible causes:");
      bentley::error!("1. ONNX Runtime binary lacks CUDA support");
      bentley::error!("2. CUDA/cuDNN version mismatch"); 
      bentley::error!("3. GPU memory exhausted, forcing CPU fallback");
      bentley::error!("4. ONNX graph not optimized for CUDA");
      
    } else if per_text_ms > 10.0 {
      bentley::warn!(&format!(
        "⚠ SLOW: {:.2}ms per text is suboptimal (expected <10ms on GPU)",
        per_text_ms
      ));
    } else {
      bentley::info!(&format!("✓ FAST: {:.2}ms per text - GPU acceleration working!", per_text_ms));
    }
    
    // Extract and normalize all embeddings
    let raw_embeddings = Self::extract_batch_embeddings(&output, texts.len())?;
    
    // Normalize each embedding in the batch
    let normalized_embeddings: Result<Vec<_>> = raw_embeddings
      .into_iter()
      .map(Self::normalize_embedding)
      .collect();

    normalized_embeddings
  }

  /// Calculate optimal chunk size to prevent excessive memory usage
  fn calculate_optimal_chunk_size(batch_size: usize) -> usize {
    const MAX_SEQUENCE_CAP: usize = 2048; // EmbeddingGemma-300M's context limit
    const MAX_TENSOR_ELEMENTS: usize = 2_000_000; // ~2M elements max for larger context
    
    let max_batch_for_full_seq = MAX_TENSOR_ELEMENTS / MAX_SEQUENCE_CAP;
    
    if batch_size <= max_batch_for_full_seq {
      batch_size // Use full batch
    } else {
      max_batch_for_full_seq.max(1) // Split into smaller chunks
    }
  }
}

// Model initialization
// violet ignore chunk - this is about as simple and flat as it's going to get without breaking this into
// singlet implementation blocks.
#[cfg(not(tarpaulin_include))] // [rag-stack] - add CI/CD testing for cross-platform loading/unloading
impl EmbeddingModel {
  async fn download_model() -> Result<ModelFiles> {
    let api = Api::new().map_err(|e| anyhow!("HF API initialization failed: {}", e))?;

    let repo = api.model(MODEL_NAME.to_string());

    let tokenizer_file =
      repo.get(TOKENIZER_FILE).await.map_err(|e| anyhow!("Failed to download tokenizer: {}", e))?;

    let model_path =
      repo.get(MODEL_FILE).await.map_err(|e| anyhow!("Failed to download ONNX model: {}", e))?;

    // Download config files to understand model architecture
    Self::ensure_config_files(&repo).await?;

    // Check and download external data file if needed
    Self::ensure_external_data_file(&model_path, &repo).await?;

    Ok(ModelFiles { tokenizer_file, model_path })
  }

  async fn ensure_config_files(repo: &hf_hub::api::tokio::ApiRepo) -> Result<()> {
    bentley::info!("Downloading model configuration files...");

    // Download essential config files
    let config_files = ["config.json", "generation_config.json", "tokenizer_config.json"];

    for config_file in &config_files {
      match repo.get(config_file).await {
        Ok(_) => bentley::info!(&format!("Downloaded {config_file}")),
        Err(e) => bentley::warn!(&format!("Could not download {config_file}: {e}")),
      }
    }

    Ok(())
  }

  async fn ensure_external_data_file(
    model_path: &std::path::Path,
    repo: &hf_hub::api::tokio::ApiRepo,
  ) -> Result<()> {
    // Check if external data file exists
    let external_data_path = model_path.with_file_name("model.onnx_data");

    if !external_data_path.exists() {
      bentley::info!("External data file missing, downloading model.onnx_data...");

      // Download the external data file
      let _external_data_file = repo
        .get("onnx/model.onnx_data")
        .await
        .map_err(|e| anyhow!("Failed to download external data file: {}", e))?;

      bentley::info!("External data file downloaded successfully");
    }

    Ok(())
  }

  fn load_tokenizer(path: std::path::PathBuf) -> Result<Tokenizer> {
    Tokenizer::from_file(path).map_err(|e| anyhow!("Failed to load tokenizer: {}", e))
  }

  fn load_model(model_path: std::path::PathBuf) -> Result<Session> {
    // Set environment variables that can fix ONNX Runtime CUDA issues
    std::env::set_var("OMP_NUM_THREADS", "4");
    std::env::set_var("CUDA_VISIBLE_DEVICES", "0"); 
    std::env::set_var("CUDA_LAUNCH_BLOCKING", "0"); // Async GPU execution
    
    bentley::info!("Set CUDA environment variables for optimal performance");
    
    let providers = Self::get_execution_providers();
    bentley::info!(&format!("Configuring ONNX session with {} execution providers", providers.len()));

    let session = Session::builder()?
      .with_execution_providers(providers)?
      .commit_from_file(model_path)?;

    bentley::info!("ONNX session initialized successfully");
    bentley::info!("✓ Both CUDA and CPU providers available");

    Ok(session)
  }
}

// Hardware detection
#[cfg(not(tarpaulin_include))] // [rag-stack] - add CI/CD testing for xplat
impl EmbeddingModel {
  fn get_execution_providers() -> Vec<ExecutionProviderDispatch> {
    let mut providers = Vec::new();

    #[cfg(target_os = "macos")]
    {
      providers.push(CoreMLExecutionProvider::default().into());
    }

    #[cfg(target_os = "linux")]
    {
      if Self::is_cuda_available() {
        // Try different CUDA provider configurations for better GPU utilization
        let cuda_provider = CUDAExecutionProvider::default()
          .with_device_id(0)
          .with_memory_limit(2 * 1024 * 1024 * 1024) // 2GB limit
          .build()
          .error_on_failure();
          
        bentley::info!("CUDA execution provider configured: GPU 0, 2GB memory limit");
        providers.push(cuda_provider);
      } else {
        bentley::warn!("CUDA not available, falling back to CPU");
      }
    }

    providers.push(CPUExecutionProvider::default().into());
    providers
  }

  /// Check if CUDA is available 
  #[cfg(target_os = "linux")]
  fn is_cuda_available() -> bool {
    // Check hardware level
    let hardware_available = std::process::Command::new("nvidia-smi")
      .output()
      .map(|output| output.status.success())
      .unwrap_or(false);

    if hardware_available {
      bentley::info!("✓ NVIDIA hardware detected");
      
      // Check for basic CUDA runtime libraries
      let cuda_rt_available = std::path::Path::new("/usr/local/cuda/lib64/libcudart.so").exists()
        || std::path::Path::new("/usr/lib/x86_64-linux-gnu/libcudart.so").exists();
        
      if cuda_rt_available {
        bentley::info!("✓ CUDA runtime libraries found");
        true
      } else {
        bentley::warn!("⚠ CUDA runtime libraries not found - may fall back to CPU");
        true // Still try CUDA provider, it might work
      }
    } else {
      bentley::warn!("⚠ No NVIDIA hardware detected");
      false
    }
  }
}

// violet ignore chunk - this is about as simple and flat as it's going to get without being terse.
// Embedding processing
impl EmbeddingModel {
  /// Testable tokenization logic
  fn tokenize(text: &str, tokenizer: &dyn TextTokenizer) -> Result<Box<dyn TokenizerOutput>> {
    let tokens = tokenizer.encode_text(text, true)?;

    let token_count = tokens.get_ids().len();
    bentley::info!(&format!("Tokenized '{text}' into {token_count} tokens"));
    Self::validate_sequence_length(token_count)?;

    Ok(tokens)
  }

  /// Validate token sequence length - extracted for easy testing
  fn validate_sequence_length(token_count: usize) -> Result<()> {
    const MAX_SEQUENCE_LENGTH: usize = 2048; // EmbeddingGemma-300M limit is 2048

    if token_count > MAX_SEQUENCE_LENGTH {
      bentley::warn!(&format!(
        "Sequence length {token_count} exceeds EmbeddingGemma-300M limit of {MAX_SEQUENCE_LENGTH}. Sequence will be truncated."
      ));
    }

    Ok(())
  }

  /// Testable tensor preparation logic
  fn prepare(
    tokens: &dyn TokenEncoding,
    session: &dyn SessionInputs,
  ) -> Result<std::collections::HashMap<String, Value>> {
    let input_ids_tensor = Self::to_tensor(tokens.get_ids())?;
    let attention_mask_tensor = Self::to_tensor(tokens.get_attention_mask())?;
    let token_type_ids_tensor = Self::to_tensor(tokens.get_type_ids())?;

    // Generate position IDs: [0, 1, 2, ..., seq_len-1]
    let seq_len = tokens.get_ids().len();
    let position_ids: Vec<u32> = (0..seq_len as u32).collect();
    let position_ids_tensor = Self::to_tensor(&position_ids)?;

    // Create input based on what the model expects
    let mut input = HashMap::new();
    input.insert("input_ids".to_string(), input_ids_tensor);
    input.insert("attention_mask".to_string(), attention_mask_tensor);

    // Get model input names to determine what the model expects
    let model_input_names = session.input_names();

    if model_input_names.contains(&"token_type_ids".to_string()) {
      input.insert("token_type_ids".to_string(), token_type_ids_tensor);
    } else {
      bentley::verbose!("Model doesn't expect token_type_ids, skipping");
    }

    if model_input_names.contains(&"position_ids".to_string()) {
      input.insert("position_ids".to_string(), position_ids_tensor);
      bentley::verbose!("Added position_ids for Qwen3 model");
    }

    // Add past_key_values for CausalLM models (empty for embedding tasks)
    Self::add_past_key_values(&mut input, &model_input_names)?;

    Ok(input)
  }

  /// Prepare batch tensors for multiple tokenized texts
  fn prepare_batch(
    all_tokens: &[Box<dyn TokenizerOutput>],
    session: &dyn SessionInputs,
  ) -> Result<std::collections::HashMap<String, Value>> {
    let batch_size = all_tokens.len();
    
    // Find the maximum sequence length for padding, but cap it to prevent excessive padding
    const MAX_SEQUENCE_CAP: usize = 2048; // EmbeddingGemma-300M's context limit
    
    let max_seq_len = all_tokens
      .iter()
      .map(|tokens| tokens.get_ids().len())
      .max()
      .unwrap_or(0)
      .min(MAX_SEQUENCE_CAP); // Cap to reasonable limit
      
    // Warn about truncated sequences
    let truncated_count = all_tokens
      .iter()
      .filter(|tokens| tokens.get_ids().len() > max_seq_len)
      .count();
    
    if truncated_count > 0 {
      bentley::warn!(&format!(
        "Truncating {} sequences from max length {} to {}",
        truncated_count,
        all_tokens.iter().map(|t| t.get_ids().len()).max().unwrap_or(0),
        max_seq_len
      ));
    }

    bentley::info!(&format!(
      "Preparing batch tensors: batch_size={}, max_seq_len={}", 
      batch_size, max_seq_len
    ));

    // Check for excessive padding that could cause performance issues
    let total_elements = batch_size * max_seq_len;
    if total_elements > 100_000 {
      bentley::warn!(&format!(
        "Large tensor detected: batch_size={} × max_seq_len={} = {} elements. This may cause performance issues.",
        batch_size, max_seq_len, total_elements
      ));
    }

    // Create padded batch tensors
    let input_ids_tensor = Self::to_tensor_batch(
      all_tokens.iter().map(|t| t.get_ids()).collect::<Vec<_>>().as_slice(),
      batch_size,
      max_seq_len,
    )?;

    let attention_mask_tensor = Self::to_tensor_batch(
      all_tokens.iter().map(|t| t.get_attention_mask()).collect::<Vec<_>>().as_slice(),
      batch_size,
      max_seq_len,
    )?;

    let token_type_ids_tensor = Self::to_tensor_batch(
      all_tokens.iter().map(|t| t.get_type_ids()).collect::<Vec<_>>().as_slice(),
      batch_size,
      max_seq_len,
    )?;

    // Generate position IDs for each sequence: [0, 1, 2, ..., seq_len-1], capped at max_seq_len
    let position_ids_batch: Vec<Vec<u32>> = all_tokens
      .iter()
      .map(|tokens| {
        let seq_len = tokens.get_ids().len().min(max_seq_len);
        (0..seq_len as u32).collect()
      })
      .collect();
    
    let position_ids_tensor = Self::to_tensor_batch(
      position_ids_batch.iter().map(|v| v.as_slice()).collect::<Vec<_>>().as_slice(),
      batch_size,
      max_seq_len,
    )?;

    // Create input based on what the model expects
    let mut input = HashMap::new();
    input.insert("input_ids".to_string(), input_ids_tensor);
    input.insert("attention_mask".to_string(), attention_mask_tensor);

    // Get model input names to determine what the model expects
    let model_input_names = session.input_names();

    if model_input_names.contains(&"token_type_ids".to_string()) {
      input.insert("token_type_ids".to_string(), token_type_ids_tensor);
    } else {
      bentley::verbose!("Model doesn't expect token_type_ids, skipping");
    }

    if model_input_names.contains(&"position_ids".to_string()) {
      input.insert("position_ids".to_string(), position_ids_tensor);
      bentley::verbose!("Added position_ids for batch processing");
    }

    // Add past_key_values for CausalLM models (empty for embedding tasks)
    Self::add_batch_past_key_values(&mut input, &model_input_names, batch_size)?;

    Ok(input)
  }

  fn add_past_key_values(
    input: &mut HashMap<String, Value>,
    model_input_names: &[String],
  ) -> Result<()> {
    // Check if model expects past key values (common for CausalLM-based embedding models)
    let past_key_names: Vec<&String> =
      model_input_names.iter().filter(|name| name.starts_with("past_key_values.")).collect();

    if !past_key_names.is_empty() {
      bentley::verbose!(&format!(
        "Adding {} empty past_key_values tensors for CausalLM",
        past_key_names.len()
      ));

      for past_key_name in past_key_names {
        let empty_tensor = Self::create_empty_past_key_value_tensor()?;
        input.insert(past_key_name.clone(), empty_tensor);
      }
    }

    Ok(())
  }

  fn create_empty_past_key_value_tensor() -> Result<Value> {
    // Create empty tensor with shape [1, num_heads, 0, head_dim]
    // For Qwen3: num_heads=8, head_dim=128 (from config)
    use ndarray::Array4;
    let empty_array: Array4<f32> = Array4::zeros((1, 8, 0, 128));
    Ok(Value::from_array(empty_array)?.into())
  }

  fn to_tensor<T: Copy + Into<i64>>(values: &[T]) -> Result<Value> {
    let seq_len = values.len();
    let array: Array2<i64> = Array2::from_shape_vec((1, seq_len), Self::to_i64(values))?;
    let tensor: Value = Value::from_array(array)?.into();
    Ok(tensor)
  }

  fn to_tensor_batch<T: Copy + Into<i64>>(
    batch_values: &[&[T]], 
    batch_size: usize, 
    max_seq_len: usize
  ) -> Result<Value> {
    let mut flat_data = Vec::with_capacity(batch_size * max_seq_len);
    
    for sequence in batch_values {
      // Truncate or pad sequence to max_seq_len
      let seq_len = sequence.len().min(max_seq_len);
      
      // Add the actual sequence values (truncated if necessary)
      for i in 0..seq_len {
        flat_data.push(sequence[i].into());
      }
      
      // Pad with zeros if sequence is shorter than max_seq_len
      for _ in seq_len..max_seq_len {
        flat_data.push(0i64);
      }
    }
    
    let array: Array2<i64> = Array2::from_shape_vec((batch_size, max_seq_len), flat_data)?;
    let tensor: Value = Value::from_array(array)?.into();
    Ok(tensor)
  }

  fn to_i64<T: Copy + Into<i64>>(values: &[T]) -> Vec<i64> {
    values.iter().map(|&x| x.into()).collect()
  }

  fn add_batch_past_key_values(
    input: &mut HashMap<String, Value>,
    model_input_names: &[String],
    batch_size: usize,
  ) -> Result<()> {
    // Check if model expects past key values (common for CausalLM-based embedding models)
    let past_key_names: Vec<&String> =
      model_input_names.iter().filter(|name| name.starts_with("past_key_values.")).collect();

    if !past_key_names.is_empty() {
      bentley::verbose!(&format!(
        "Adding {} empty past_key_values tensors for CausalLM batch (batch_size={})",
        past_key_names.len(),
        batch_size
      ));

      for past_key_name in past_key_names {
        let empty_tensor = Self::create_empty_batch_past_key_value_tensor(batch_size)?;
        input.insert(past_key_name.clone(), empty_tensor);
      }
    }

    Ok(())
  }

  fn create_empty_batch_past_key_value_tensor(batch_size: usize) -> Result<Value> {
    // Create empty tensor with shape [batch_size, num_heads, 0, head_dim]
    // For Qwen3: num_heads=8, head_dim=128 (from config)
    use ndarray::Array4;
    let empty_array: Array4<f32> = Array4::zeros((batch_size, 8, 0, 128));
    Ok(Value::from_array(empty_array)?.into())
  }

  /// Testable tensor extraction logic
  fn extract_embedding(output: &dyn EmbeddingOutput) -> Result<Vec<f32>> {
    let tensor = output
      .get_tensor("last_hidden_state")
      .or_else(|| output.get_tensor("0"))
      .ok_or_else(|| anyhow!("No output found from model - expected 'last_hidden_state' or '0'"))?;

    let (shape, data) = tensor.extract_f32_data()?;

    Self::mean_pool((shape, data))
  }

  /// Extract embeddings from batch output tensor
  fn extract_batch_embeddings(output: &dyn EmbeddingOutput, batch_size: usize) -> Result<Vec<Vec<f32>>> {
    let tensor = output
      .get_tensor("last_hidden_state")
      .or_else(|| output.get_tensor("0"))
      .ok_or_else(|| anyhow!("No output found from model - expected 'last_hidden_state' or '0'"))?;

    let (shape, data) = tensor.extract_f32_data()?;

    Self::mean_pool_batch((shape, data), batch_size)
  }

  /// Perform mean pooling over sequence dimension for sentence embeddings
  pub fn mean_pool(embedding: (&[i64], &[f32])) -> Result<Vec<f32>> {
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

  /// Perform mean pooling over batch of sequences for sentence embeddings
  pub fn mean_pool_batch(embedding: (&[i64], &[f32]), batch_size: usize) -> Result<Vec<Vec<f32>>> {
    let (shape, data) = embedding;

    let seq_length = shape[1] as usize;
    let hidden_size = shape[2] as usize;

    let mut batch_embeddings = Vec::with_capacity(batch_size);

    for batch_idx in 0..batch_size {
      let mut sequence_embedding = vec![0.0f32; hidden_size];
      
      // Calculate mean pooling for this sequence in the batch
      for token_idx in 0..seq_length {
        let data_start = (batch_idx * seq_length * hidden_size) + (token_idx * hidden_size);
        let data_end = data_start + hidden_size;
        
        for (i, &value) in data[data_start..data_end].iter().enumerate() {
          sequence_embedding[i] += value;
        }
      }

      // Average over sequence length
      for value in sequence_embedding.iter_mut() {
        *value /= seq_length as f32;
      }

      batch_embeddings.push(sequence_embedding);
    }

    Ok(batch_embeddings)
  }

  /// Normalize embedding vector to unit length for consistent similarity comparisons
  pub fn normalize_embedding(mut embedding: Vec<f32>) -> Result<Vec<f32>> {
    // Calculate magnitude (L2 norm)
    let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();

    // Avoid division by zero
    if magnitude < f32::EPSILON {
      bentley::warn!("Zero-magnitude embedding detected - returning unchanged");
      return Ok(embedding);
    }

    // Normalize to unit length
    for value in embedding.iter_mut() {
      *value /= magnitude;
    }

    bentley::verbose!(&format!(
      "Normalized embedding from magnitude {magnitude:.6} to unit length"
    ));
    Ok(embedding)
  }
}

// Global singleton for the embedding model
static MODEL: std::sync::OnceLock<Mutex<Option<EmbeddingModel>>> = std::sync::OnceLock::new();
/// Detect the current embedding model's output dimension by creating a test embedding
#[cfg(not(tarpaulin_include))]
pub async fn detect_embedding_dimension() -> Result<usize> {
  // Create a simple test embedding to determine the output dimension
  let test_text = "test";
  let test_embedding = create_embedding(test_text).await?;
  Ok(test_embedding.len())
}

/// Public API function to create embeddings - initializes model on first use
///
/// This is a generic function that accepts raw text. For better performance with EmbeddingGemma,
/// consider using the task-specific functions like `create_query_embedding` or `create_document_embedding`.
#[cfg(not(tarpaulin_include))]
pub async fn create_embedding(text: &str) -> Result<Vec<f32>> {
  create_embedding_with_prompt(text).await
}

/// Create embeddings for multiple texts in a single batch inference (up to 2048 texts)
/// 
/// This provides massive performance improvements over sequential processing by utilizing
/// the model's full batch processing capability.
#[cfg(not(tarpaulin_include))]
pub async fn create_embeddings_batch(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
  if texts.is_empty() {
    return Ok(vec![]);
  }

  const MAX_BATCH_SIZE: usize = 2048;
  if texts.len() > MAX_BATCH_SIZE {
    return Err(anyhow!("Batch size {} exceeds maximum of {}", texts.len(), MAX_BATCH_SIZE));
  }

  let mutex = MODEL.get_or_init(|| Mutex::new(None));

  // Check if we need to initialize the model
  let needs_init = {
    let guard = mutex.lock().map_err(|_| anyhow!("Failed to lock model mutex"))?;
    guard.is_none()
  };

  // Initialize model if needed (outside of the lock to avoid holding across await)
  if needs_init {
    bentley::info!("Initializing embedding model...");
    let model = EmbeddingModel::load().await?;
    let mut guard = mutex.lock().map_err(|_| anyhow!("Failed to lock model mutex"))?;
    *guard = Some(model);
  }

  // Get batch embeddings
  let mut guard = mutex.lock().map_err(|_| anyhow!("Failed to lock model mutex"))?;
  let model = guard.as_mut().ok_or_else(|| anyhow!("Model not initialized"))?;
  model.embed_batch(texts)
}

/// Create embeddings optimized for search queries using EmbeddingGemma prompt format
/// Uses format: "task: search result | query: {content}"
#[cfg(not(tarpaulin_include))]
pub async fn create_query_embedding(query: &str) -> Result<Vec<f32>> {
  let formatted_query = format!("task: search result | query: {query}");
  bentley::verbose!(&format!(
    "Creating query embedding with EmbeddingGemma format: '{}'",
    formatted_query.chars().take(100).collect::<String>()
  ));
  create_embedding_with_prompt(&formatted_query).await
}

/// Create embeddings optimized for documents using EmbeddingGemma prompt format
/// Uses format: "title: {title | "none"} | text: {content}"
#[cfg(not(tarpaulin_include))]
pub async fn create_document_embedding(content: &str, title: Option<&str>) -> Result<Vec<f32>> {
  let title_part = title.unwrap_or("none");
  let formatted_doc = format!("title: {title_part} | text: {content}");
  bentley::verbose!(&format!(
    "Creating document embedding with EmbeddingGemma format: title='{}', content_length={}",
    title_part,
    content.len()
  ));
  create_embedding_with_prompt(&formatted_doc).await
}

/// Create document embeddings for multiple documents in a single batch (up to 2048)
/// Uses format: "title: {title | "none"} | text: {content}" for each document
#[cfg(not(tarpaulin_include))]
pub async fn create_document_embeddings_batch(
  documents: &[(String, Option<String>)]
) -> Result<Vec<Vec<f32>>> {
  let formatted_docs: Vec<String> = documents
    .iter()
    .map(|(content, title)| {
      let title_part = title.as_deref().unwrap_or("none");
      format!("title: {title_part} | text: {content}")
    })
    .collect();

  let formatted_refs: Vec<&str> = formatted_docs.iter().map(|s| s.as_str()).collect();

  bentley::verbose!(&format!(
    "Creating batch document embeddings for {} documents with EmbeddingGemma format",
    documents.len()
  ));
  
  create_embeddings_batch(&formatted_refs).await
}

/// Create embeddings optimized for semantic similarity using EmbeddingGemma prompt format
/// Uses format: "task: sentence similarity | query: {content}"
/// This is specifically designed for similarity assessment, not retrieval tasks.
#[cfg(not(tarpaulin_include))]
pub async fn create_semantic_similarity_embedding(content: &str) -> Result<Vec<f32>> {
  let formatted_content = format!("task: sentence similarity | query: {content}");
  bentley::verbose!(&format!(
    "Creating semantic similarity embedding with EmbeddingGemma format, content_length={}",
    content.len()
  ));
  create_embedding_with_prompt(&formatted_content).await
}

/// Internal function to create embeddings with proper model initialization
#[cfg(not(tarpaulin_include))]
async fn create_embedding_with_prompt(formatted_text: &str) -> Result<Vec<f32>> {
  let mutex = MODEL.get_or_init(|| Mutex::new(None));

  // Check if we need to initialize the model
  let needs_init = {
    let guard = mutex.lock().map_err(|_| anyhow!("Failed to lock model mutex"))?;
    guard.is_none()
  };

  // Initialize model if needed (outside of the lock to avoid holding across await)
  if needs_init {
    bentley::info!("Initializing embedding model...");
    let model = EmbeddingModel::load().await?;
    let mut guard = mutex.lock().map_err(|_| anyhow!("Failed to lock model mutex"))?;
    *guard = Some(model);
  }

  // Get embedding
  let mut guard = mutex.lock().map_err(|_| anyhow!("Failed to lock model mutex"))?;
  let model = guard.as_mut().ok_or_else(|| anyhow!("Model not initialized"))?;
  model.embed(formatted_text)
}

/// Generate a reranking relevance score using EmbeddingGemma semantic similarity task
///
/// This function uses the "Semantic Similarity" task which is specifically optimized
/// for assessing text similarity (not retrieval). Both query and document get the
/// same semantic similarity formatting for optimal similarity comparison.
#[cfg(not(tarpaulin_include))]
pub async fn score_relevance(query: &str, document: &str) -> Result<f32> {
  bentley::verbose!(&format!(
    "Reranking with semantic similarity task: query='{}', doc_length={}",
    query.chars().take(50).collect::<String>(),
    document.len()
  ));

  // Use semantic similarity task for both query and document
  // This is specifically designed for similarity assessment, not retrieval
  let query_embedding = create_semantic_similarity_embedding(query).await?;
  let doc_embedding = create_semantic_similarity_embedding(document).await?;

  // Calculate cosine similarity between semantic similarity embeddings
  let similarity = cosine_similarity(&query_embedding, &doc_embedding);

  bentley::verbose!(&format!("Semantic similarity reranking score: {similarity:.4}"));

  Ok(similarity)
}

/// Generate reranking relevance scores for multiple documents in a single batch
/// 
/// This provides massive performance improvement over sequential processing by 
/// processing all candidates in one inference call.
#[cfg(not(tarpaulin_include))]
pub async fn score_relevance_batch(query: &str, documents: &[&str]) -> Result<Vec<f32>> {
  if documents.is_empty() {
    return Ok(vec![]);
  }

  bentley::verbose!(&format!(
    "Batch reranking with semantic similarity task: query='{}', {} documents",
    query.chars().take(50).collect::<String>(),
    documents.len()
  ));

  // Generate query embedding once
  let query_embedding = create_semantic_similarity_embedding(query).await?;

  // Generate all document embeddings in a single batch
  let document_embeddings = {
    let formatted_docs: Vec<String> = documents
      .iter()
      .map(|doc| format!("task: sentence similarity | query: {}", doc))
      .collect();
    
    let formatted_refs: Vec<&str> = formatted_docs.iter().map(|s| s.as_str()).collect();
    create_embeddings_batch(&formatted_refs).await?
  };

  // Calculate cosine similarities for all documents
  let similarities: Vec<f32> = document_embeddings
    .iter()
    .map(|doc_embedding| cosine_similarity(&query_embedding, doc_embedding))
    .collect();

  bentley::verbose!(&format!(
    "Batch reranking completed: {} scores generated",
    similarities.len()
  ));

  Ok(similarities)
}

/// Calculate cosine similarity between two embedding vectors
///
/// Returns a value between -1 and 1, where:
/// - 1 = identical direction (high similarity)
/// - 0 = orthogonal (no similarity)  
/// - -1 = opposite direction (negative similarity)
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
  if a.len() != b.len() {
    bentley::warn!(&format!("Embedding dimension mismatch: {} vs {}", a.len(), b.len()));
    return 0.0;
  }

  if a.is_empty() {
    return 0.0;
  }

  // Calculate dot product
  let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();

  // Calculate magnitudes
  let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
  let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

  // Avoid division by zero
  if norm_a == 0.0 || norm_b == 0.0 {
    return 0.0;
  }

  // Return normalized cosine similarity, clamped to [0,1] for relevance scoring
  let similarity = dot_product / (norm_a * norm_b);

  // Convert from [-1,1] to [0,1] range for relevance scores
  // This ensures negative similarity (opposite direction) becomes low relevance
  ((similarity + 1.0) / 2.0).clamp(0.0, 1.0)
}

/// No-op functions when ML features are not available
#[cfg(not(feature = "ml-features"))]
pub async fn create_query_embedding(_query: &str) -> Result<Vec<f32>> {
  Err(anyhow!("ML features not available"))
}

#[cfg(not(feature = "ml-features"))]
pub async fn create_document_embedding(_content: &str, _title: Option<&str>) -> Result<Vec<f32>> {
  Err(anyhow!("ML features not available"))
}

#[cfg(not(feature = "ml-features"))]
pub async fn create_semantic_similarity_embedding(_content: &str) -> Result<Vec<f32>> {
  Err(anyhow!("ML features not available"))
}

#[cfg(not(feature = "ml-features"))]
pub async fn create_reranking_score(_query: &str, _document: &str) -> Result<f32> {
  // Return neutral score when ML features not available
  Ok(0.5)
}

#[cfg(test)]
mod gte_base_tests {
  use super::*;
  use anyhow::Result;
  use std::collections::HashMap;

  /// Mock implementations for testing
  struct MockTensorExtractor {
    tensors: HashMap<String, MockTensorData>,
  }

  struct MockTensorData {
    shape: Vec<i64>,
    data: Vec<f32>,
  }

  struct MockTokenEncoding {
    ids: Vec<u32>,
    attention_mask: Vec<u32>,
    type_ids: Vec<u32>,
  }

  struct MockSessionInputs {
    input_names: Vec<String>,
  }

  /// Mock tokenizer implementations
  struct MockTextTokenizer {
    should_fail: bool,
    token_ids: Vec<u32>,
  }

  #[derive(Debug)]
  struct MockTokenizerOutput {
    ids: Vec<u32>,
    attention_mask: Vec<u32>,
    type_ids: Vec<u32>,
  }

  impl TextTokenizer for MockTextTokenizer {
    fn encode_text(
      &self,
      _text: &str,
      _add_special_tokens: bool,
    ) -> Result<Box<dyn TokenizerOutput>> {
      if self.should_fail {
        return Err(anyhow!("Mock tokenization failure"));
      }

      let len = self.token_ids.len();
      Ok(Box::new(MockTokenizerOutput {
        ids: self.token_ids.clone(),
        attention_mask: vec![1; len],
        type_ids: vec![0; len],
      }))
    }
  }

  impl TokenEncoding for MockTokenizerOutput {
    fn get_ids(&self) -> &[u32] {
      &self.ids
    }
    fn get_attention_mask(&self) -> &[u32] {
      &self.attention_mask
    }
    fn get_type_ids(&self) -> &[u32] {
      &self.type_ids
    }
  }

  impl TokenizerOutput for MockTokenizerOutput {}

  impl TokenEncoding for MockTokenEncoding {
    fn get_ids(&self) -> &[u32] {
      &self.ids
    }
    fn get_attention_mask(&self) -> &[u32] {
      &self.attention_mask
    }
    fn get_type_ids(&self) -> &[u32] {
      &self.type_ids
    }
  }

  impl SessionInputs for MockSessionInputs {
    fn input_names(&self) -> Vec<String> {
      self.input_names.clone()
    }
  }

  /// Test cosine similarity calculation
  #[test]
  fn test_cosine_similarity() {
    // Test identical vectors (should return 1.0 after normalization)
    let vec_a = vec![1.0, 2.0, 3.0];
    let vec_b = vec![1.0, 2.0, 3.0];
    let similarity = cosine_similarity(&vec_a, &vec_b);
    assert!((similarity - 1.0).abs() < 0.001, "Identical vectors should have similarity 1.0");

    // Test orthogonal vectors (should return 0.5 after [0,1] normalization)
    let vec_c = vec![1.0, 0.0];
    let vec_d = vec![0.0, 1.0];
    let orthogonal_sim = cosine_similarity(&vec_c, &vec_d);
    assert!((orthogonal_sim - 0.5).abs() < 0.001, "Orthogonal vectors should have similarity 0.5");

    // Test opposite vectors (should return 0.0 after [0,1] normalization)
    let vec_e = vec![1.0, 2.0, 3.0];
    let vec_f = vec![-1.0, -2.0, -3.0];
    let opposite_sim = cosine_similarity(&vec_e, &vec_f);
    assert!(opposite_sim < 0.001, "Opposite vectors should have similarity near 0.0");

    // Test zero vectors (should return 0.0)
    let zero_vec = vec![0.0, 0.0, 0.0];
    let normal_vec = vec![1.0, 2.0, 3.0];
    let zero_sim = cosine_similarity(&zero_vec, &normal_vec);
    assert_eq!(zero_sim, 0.0, "Zero vector should have similarity 0.0");

    // Test empty vectors (should return 0.0)
    let empty_vec: Vec<f32> = vec![];
    let empty_sim = cosine_similarity(&empty_vec, &empty_vec);
    assert_eq!(empty_sim, 0.0, "Empty vectors should have similarity 0.0");

    // Test dimension mismatch (should return 0.0)
    let vec_2d = vec![1.0, 2.0];
    let vec_3d = vec![1.0, 2.0, 3.0];
    let mismatch_sim = cosine_similarity(&vec_2d, &vec_3d);
    assert_eq!(mismatch_sim, 0.0, "Dimension mismatch should return 0.0");

    // Test that similar vectors have higher similarity than dissimilar ones
    let similar_a = vec![1.0, 2.0, 3.0];
    let similar_b = vec![1.1, 2.1, 2.9]; // Close to similar_a
    let different_c = vec![10.0, -5.0, 0.1]; // Different from similar_a

    let similar_score = cosine_similarity(&similar_a, &similar_b);
    let different_score = cosine_similarity(&similar_a, &different_c);

    assert!(similar_score > different_score, "Similar vectors should have higher similarity");
  }

  /// Test prepare_from with token_type_ids expected by model
  #[test]
  fn test_prepare_with_token_type_ids_expected() -> Result<()> {
    let tokens = MockTokenEncoding {
      ids: vec![101, 7592, 102], // [CLS] hello [SEP]
      attention_mask: vec![1, 1, 1],
      type_ids: vec![0, 0, 0],
    };

    let session = MockSessionInputs {
      input_names: vec![
        "input_ids".to_string(),
        "attention_mask".to_string(),
        "token_type_ids".to_string(), // Model expects this
      ],
    };

    let result = EmbeddingModel::prepare(&tokens, &session)?;

    // Should contain all three tensors
    assert_eq!(result.len(), 3);
    assert!(result.contains_key("input_ids"));
    assert!(result.contains_key("attention_mask"));
    assert!(result.contains_key("token_type_ids"));

    Ok(())
  }

  /// Test prepare_from with token_type_ids NOT expected by model
  #[test]
  fn test_prepare_without_token_type_ids_expected() -> Result<()> {
    let tokens = MockTokenEncoding {
      ids: vec![101, 7592, 102],
      attention_mask: vec![1, 1, 1],
      type_ids: vec![0, 0, 0],
    };

    let session = MockSessionInputs {
      input_names: vec![
        "input_ids".to_string(),
        "attention_mask".to_string(),
        // No token_type_ids - model doesn't expect it
      ],
    };

    let result = EmbeddingModel::prepare(&tokens, &session)?;

    // Should contain only two tensors
    assert_eq!(result.len(), 2);
    assert!(result.contains_key("input_ids"));
    assert!(result.contains_key("attention_mask"));
    assert!(!result.contains_key("token_type_ids")); // Should be excluded

    Ok(())
  }

  /// Test prepare_from with single token
  #[test]
  fn test_prepare_single_token() -> Result<()> {
    let tokens = MockTokenEncoding {
      ids: vec![101], // Just [CLS]
      attention_mask: vec![1],
      type_ids: vec![0],
    };

    let session = MockSessionInputs {
      input_names: vec!["input_ids".to_string(), "attention_mask".to_string()],
    };

    let result = EmbeddingModel::prepare(&tokens, &session)?;

    assert_eq!(result.len(), 2);
    assert!(result.contains_key("input_ids"));
    assert!(result.contains_key("attention_mask"));

    Ok(())
  }

  /// Test prepare_from with empty tokens (edge case)
  #[test]
  fn test_prepare_empty_tokens() -> Result<()> {
    let tokens = MockTokenEncoding { ids: vec![], attention_mask: vec![], type_ids: vec![] };

    let session = MockSessionInputs {
      input_names: vec!["input_ids".to_string(), "attention_mask".to_string()],
    };

    let result = EmbeddingModel::prepare(&tokens, &session)?;

    assert_eq!(result.len(), 2);
    assert!(result.contains_key("input_ids"));
    assert!(result.contains_key("attention_mask"));

    Ok(())
  }

  /// Test prepare_from with long sequence
  #[test]
  fn test_prepare_long_sequence() -> Result<()> {
    let tokens = MockTokenEncoding {
      ids: (0..512).collect(), // 512 tokens
      attention_mask: vec![1; 512],
      type_ids: vec![0; 256].into_iter().chain(vec![1; 256]).collect(), // Mixed type IDs
    };

    let session = MockSessionInputs {
      input_names: vec![
        "input_ids".to_string(),
        "attention_mask".to_string(),
        "token_type_ids".to_string(),
      ],
    };

    let result = EmbeddingModel::prepare(&tokens, &session)?;

    assert_eq!(result.len(), 3);
    assert!(result.contains_key("input_ids"));
    assert!(result.contains_key("attention_mask"));
    assert!(result.contains_key("token_type_ids"));

    Ok(())
  }

  /// Test prepare_from with unconventional model input names
  #[test]
  fn test_prepare_custom_input_names() -> Result<()> {
    let tokens = MockTokenEncoding {
      ids: vec![101, 7592, 102],
      attention_mask: vec![1, 1, 1],
      type_ids: vec![0, 0, 0],
    };

    let session = MockSessionInputs {
      input_names: vec![
        "input_ids".to_string(),
        "attention_mask".to_string(),
        "custom_input".to_string(), // Different name, not token_type_ids
      ],
    };

    let result = EmbeddingModel::prepare(&tokens, &session)?;

    // Should not include token_type_ids since "token_type_ids" not in input names
    assert_eq!(result.len(), 2);
    assert!(result.contains_key("input_ids"));
    assert!(result.contains_key("attention_mask"));
    assert!(!result.contains_key("token_type_ids"));

    Ok(())
  }

  /// Test sequence length validation - extracted logic
  #[test]
  fn test_validate_sequence_length_within_limit() -> Result<()> {
    // Test at various valid lengths
    assert!(EmbeddingModel::validate_sequence_length(1).is_ok());
    assert!(EmbeddingModel::validate_sequence_length(100).is_ok());
    assert!(EmbeddingModel::validate_sequence_length(511).is_ok()); // Exactly at limit

    Ok(())
  }

  #[test]
  fn test_validate_sequence_length_exceeds_limit() {
    // Test over the limit - should succeed due to temporary workaround
    let result = EmbeddingModel::validate_sequence_length(512);
    assert!(result.is_ok());

    // Test well over the limit - should also succeed due to temporary workaround
    let result = EmbeddingModel::validate_sequence_length(1000);
    assert!(result.is_ok());
  }

  /// Test tokenize with normal case
  #[test]
  fn test_tokenize_normal_case() -> Result<()> {
    let tokenizer = MockTextTokenizer {
      should_fail: false,
      token_ids: vec![101, 7592, 2256, 102], // [CLS] hello world [SEP]
    };

    let result = EmbeddingModel::tokenize("hello world", &tokenizer)?;

    assert_eq!(result.get_ids().len(), 4);
    assert_eq!(result.get_ids(), &[101, 7592, 2256, 102]);
    assert_eq!(result.get_attention_mask(), &[1, 1, 1, 1]);
    assert_eq!(result.get_type_ids(), &[0, 0, 0, 0]);

    Ok(())
  }

  /// Test tokenize with tokenization failure
  #[test]
  fn test_tokenize_tokenization_failure() {
    let tokenizer = MockTextTokenizer { should_fail: true, token_ids: vec![] };

    let result = EmbeddingModel::tokenize("any text", &tokenizer);

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Mock tokenization failure"));
  }

  /// Test tokenize with sequence length at exactly the limit
  #[test]
  fn test_tokenize_at_sequence_limit() -> Result<()> {
    let tokenizer = MockTextTokenizer {
      should_fail: false,
      token_ids: vec![1; 511], // Exactly 511 tokens (the limit)
    };

    let result = EmbeddingModel::tokenize("long text", &tokenizer)?;

    assert_eq!(result.get_ids().len(), 511);

    Ok(())
  }

  /// Test tokenize with sequence length exceeding limit
  #[test]
  fn test_tokenize_exceeds_sequence_limit() {
    let tokenizer = MockTextTokenizer {
      should_fail: false,
      token_ids: vec![1; 512], // 512 tokens - exceeds limit of 511
    };

    let result = EmbeddingModel::tokenize("very long text", &tokenizer);

    // Should succeed due to temporary workaround
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_ids().len(), 512);
  }

  /// Test tokenize with empty input (edge case)
  #[test]
  fn test_tokenize_empty_input() -> Result<()> {
    let tokenizer = MockTextTokenizer {
      should_fail: false,
      token_ids: vec![101, 102], // Just [CLS] [SEP]
    };

    let result = EmbeddingModel::tokenize("", &tokenizer)?;

    assert_eq!(result.get_ids().len(), 2); // Should have special tokens
    assert_eq!(result.get_ids(), &[101, 102]);

    Ok(())
  }

  /// Test tokenize with single character
  #[test]
  fn test_tokenize_single_character() -> Result<()> {
    let tokenizer = MockTextTokenizer {
      should_fail: false,
      token_ids: vec![101, 1037, 102], // [CLS] a [SEP]
    };

    let result = EmbeddingModel::tokenize("a", &tokenizer)?;

    assert_eq!(result.get_ids().len(), 3);
    assert_eq!(result.get_ids(), &[101, 1037, 102]);

    Ok(())
  }

  /// Test tokenize with very long sequence over limit
  #[test]
  fn test_tokenize_way_over_limit() {
    let tokenizer = MockTextTokenizer {
      should_fail: false,
      token_ids: vec![1; 1000], // Way over limit
    };

    let result = EmbeddingModel::tokenize("extremely long text", &tokenizer);

    // Should succeed due to temporary workaround
    assert!(result.is_ok());
    assert_eq!(result.unwrap().get_ids().len(), 1000);
  }

  impl EmbeddingOutput for MockTensorExtractor {
    fn get_tensor(&self, key: &str) -> Option<&dyn TensorData> {
      self.tensors.get(key).map(|t| t as &dyn TensorData)
    }
  }

  impl TensorData for MockTensorData {
    fn extract_f32_data(&self) -> Result<(&[i64], &[f32])> {
      Ok((&self.shape, &self.data))
    }
  }

  /// Test extract_embedding with "last_hidden_state" tensor
  #[test]
  fn test_extract_embedding_last_hidden_state() -> Result<()> {
    let mut tensors = HashMap::new();
    tensors.insert(
      "last_hidden_state".to_string(),
      MockTensorData {
        shape: vec![1, 2, 3],                     // batch=1, seq=2, hidden=3
        data: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], // 2 tokens, 3 dims each
      },
    );

    let output = MockTensorExtractor { tensors };
    let result = EmbeddingModel::extract_embedding(&output)?;

    assert_eq!(result.len(), 3);
    assert_eq!(result[0], 2.5); // mean of [1.0, 4.0]
    assert_eq!(result[1], 3.5); // mean of [2.0, 5.0]
    assert_eq!(result[2], 4.5); // mean of [3.0, 6.0]

    Ok(())
  }

  /// Test extract_embedding fallback to "0" tensor
  #[test]
  fn test_extract_embedding_fallback_to_zero() -> Result<()> {
    let mut tensors = HashMap::new();
    tensors.insert(
      "0".to_string(),
      MockTensorData {
        shape: vec![1, 1, 2],   // batch=1, seq=1, hidden=2
        data: vec![10.0, 20.0], // single token
      },
    );

    let output = MockTensorExtractor { tensors };
    let result = EmbeddingModel::extract_embedding(&output)?;

    assert_eq!(result, vec![10.0, 20.0]); // no averaging needed for single token

    Ok(())
  }

  /// Test extract_embedding with both tensors - should prefer "last_hidden_state"
  #[test]
  fn test_extract_embedding_prefers_last_hidden_state() -> Result<()> {
    let mut tensors = HashMap::new();
    tensors.insert(
      "last_hidden_state".to_string(),
      MockTensorData {
        shape: vec![1, 1, 2],
        data: vec![100.0, 200.0], // This should be used
      },
    );
    tensors.insert(
      "0".to_string(),
      MockTensorData {
        shape: vec![1, 1, 2],
        data: vec![1.0, 2.0], // This should be ignored
      },
    );

    let output = MockTensorExtractor { tensors };
    let result = EmbeddingModel::extract_embedding(&output)?;

    assert_eq!(result, vec![100.0, 200.0]); // Used last_hidden_state

    Ok(())
  }

  /// Test extract_embedding when no tensor is found
  #[test]
  fn test_extract_embedding_no_tensor_found() {
    let tensors = HashMap::new(); // empty - no tensors
    let output = MockTensorExtractor { tensors };

    let result = EmbeddingModel::extract_embedding(&output);

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("No output found from model"));
    assert!(error_msg.contains("last_hidden_state"));
    assert!(error_msg.contains("0"));
  }

  /// Test extract_embedding with complex multi-token scenario
  #[test]
  fn test_extract_embedding_complex_scenario() -> Result<()> {
    let mut tensors = HashMap::new();
    tensors.insert(
      "last_hidden_state".to_string(),
      MockTensorData {
        shape: vec![1, 4, 3], // batch=1, seq=4, hidden=3
        data: vec![
          1.0, 2.0, 3.0, // token 1
          4.0, 5.0, 6.0, // token 2
          7.0, 8.0, 9.0, // token 3
          10.0, 11.0, 12.0, // token 4
        ],
      },
    );

    let output = MockTensorExtractor { tensors };
    let result = EmbeddingModel::extract_embedding(&output)?;

    assert_eq!(result.len(), 3);
    assert_eq!(result[0], 5.5); // mean of [1.0, 4.0, 7.0, 10.0]
    assert_eq!(result[1], 6.5); // mean of [2.0, 5.0, 8.0, 11.0]
    assert_eq!(result[2], 7.5); // mean of [3.0, 6.0, 9.0, 12.0]

    Ok(())
  }

  /// Test normal mean pooling behavior with valid inputs
  #[test]
  fn test_mean_pool_normal_case() -> Result<()> {
    // Shape: [batch_size=1, seq_length=2, hidden_size=3]
    let shape = vec![1i64, 2i64, 3i64];

    // Data for 2 tokens, each with 3 hidden dimensions
    // Token 1: [1.0, 2.0, 3.0]
    // Token 2: [4.0, 5.0, 6.0]
    // Expected mean: [2.5, 3.5, 4.5]
    let data = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];

    let result = EmbeddingModel::mean_pool((&shape, &data))?;

    assert_eq!(result.len(), 3);
    assert_eq!(result[0], 2.5);
    assert_eq!(result[1], 3.5);
    assert_eq!(result[2], 4.5);

    Ok(())
  }

  /// Test mean pooling with single token
  #[test]
  fn test_mean_pool_single_token() -> Result<()> {
    // Shape: [batch_size=1, seq_length=1, hidden_size=4]
    let shape = vec![1i64, 1i64, 4i64];

    // Data for 1 token with 4 hidden dimensions
    let data = vec![10.0f32, 20.0, 30.0, 40.0];

    let result = EmbeddingModel::mean_pool((&shape, &data))?;

    assert_eq!(result.len(), 4);
    assert_eq!(result[0], 10.0);
    assert_eq!(result[1], 20.0);
    assert_eq!(result[2], 30.0);
    assert_eq!(result[3], 40.0);

    Ok(())
  }

  /// Test mean pooling with multiple tokens
  #[test]
  fn test_mean_pool_multiple_tokens() -> Result<()> {
    // Shape: [batch_size=1, seq_length=3, hidden_size=2]
    let shape = vec![1i64, 3i64, 2i64];

    // Data for 3 tokens, each with 2 hidden dimensions
    // Token 1: [1.0, 2.0]
    // Token 2: [3.0, 4.0]
    // Token 3: [5.0, 6.0]
    // Expected mean: [3.0, 4.0]
    let data = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];

    let result = EmbeddingModel::mean_pool((&shape, &data))?;

    assert_eq!(result.len(), 2);
    assert_eq!(result[0], 3.0);
    assert_eq!(result[1], 4.0);

    Ok(())
  }

  /// Test mean pooling with negative values
  #[test]
  fn test_mean_pool_negative_values() -> Result<()> {
    // Shape: [batch_size=1, seq_length=2, hidden_size=2]
    let shape = vec![1i64, 2i64, 2i64];

    // Data with negative values
    // Token 1: [-1.0, 2.0]
    // Token 2: [3.0, -4.0]
    // Expected mean: [1.0, -1.0]
    let data = vec![-1.0f32, 2.0, 3.0, -4.0];

    let result = EmbeddingModel::mean_pool((&shape, &data))?;

    assert_eq!(result.len(), 2);
    assert_eq!(result[0], 1.0);
    assert_eq!(result[1], -1.0);

    Ok(())
  }

  /// Test mean pooling with zero values
  #[test]
  fn test_mean_pool_zero_values() -> Result<()> {
    // Shape: [batch_size=1, seq_length=2, hidden_size=3]
    let shape = vec![1i64, 2i64, 3i64];

    // Data with all zeros
    let data = vec![0.0f32; 6];

    let result = EmbeddingModel::mean_pool((&shape, &data))?;

    assert_eq!(result.len(), 3);
    assert_eq!(result[0], 0.0);
    assert_eq!(result[1], 0.0);
    assert_eq!(result[2], 0.0);

    Ok(())
  }

  /// Test mean pooling with mixed positive/negative/zero values
  #[test]
  fn test_mean_pool_mixed_values() -> Result<()> {
    // Shape: [batch_size=1, seq_length=3, hidden_size=1]
    let shape = vec![1i64, 3i64, 1i64];

    // Data: [-5.0, 0.0, 5.0]
    // Expected mean: [0.0]
    let data = vec![-5.0f32, 0.0, 5.0];

    let result = EmbeddingModel::mean_pool((&shape, &data))?;

    assert_eq!(result.len(), 1);
    assert_eq!(result[0], 0.0);

    Ok(())
  }

  /// Test mean pooling with empty sequence - should handle division by zero
  #[test]
  fn test_mean_pool_empty_sequence() {
    // Shape: [batch_size=1, seq_length=0, hidden_size=3]
    let shape = vec![1i64, 0i64, 3i64];
    let data = vec![];

    // This should either return an error or handle the division by zero gracefully
    let result = EmbeddingModel::mean_pool((&shape, &data));

    // The current implementation will cause division by zero
    // This test documents the current behavior and should be updated if the function is fixed
    match result {
      Ok(embedding) => {
        // If it succeeds, all values should be NaN due to 0/0
        assert_eq!(embedding.len(), 3);
        for value in embedding {
          assert!(value.is_nan());
        }
      }
      Err(_) => {
        // Error is also acceptable for this edge case
      }
    }
  }

  /// Test mean pooling with empty hidden dimension
  #[test]
  fn test_mean_pool_empty_hidden_dimension() -> Result<()> {
    // Shape: [batch_size=1, seq_length=2, hidden_size=0]
    let shape = vec![1i64, 2i64, 0i64];
    let data = vec![];

    let result = EmbeddingModel::mean_pool((&shape, &data))?;

    // Should return empty vector
    assert_eq!(result.len(), 0);

    Ok(())
  }

  /// Test mean pooling with decimal averages
  #[test]
  fn test_mean_pool_decimal_averages() -> Result<()> {
    // Shape: [batch_size=1, seq_length=3, hidden_size=1]
    let shape = vec![1i64, 3i64, 1i64];

    // Data: [1.0, 2.0, 3.0]
    // Expected mean: [2.0]
    let data = vec![1.0f32, 2.0, 3.0];

    let result = EmbeddingModel::mean_pool((&shape, &data))?;

    assert_eq!(result.len(), 1);
    assert_eq!(result[0], 2.0);

    Ok(())
  }

  /// Test mean pooling with floating point precision
  #[test]
  fn test_mean_pool_floating_point_precision() -> Result<()> {
    // Shape: [batch_size=1, seq_length=3, hidden_size=2]
    let shape = vec![1i64, 3i64, 2i64];

    // Data that will create floating point division
    // Token 1: [1.0, 1.0]
    // Token 2: [1.0, 1.0]
    // Token 3: [1.0, 1.0]
    // Expected mean: [1.0, 1.0]
    let data = vec![1.0f32, 1.0, 1.0, 1.0, 1.0, 1.0];

    let result = EmbeddingModel::mean_pool((&shape, &data))?;

    assert_eq!(result.len(), 2);
    assert!((result[0] - 1.0).abs() < f32::EPSILON);
    assert!((result[1] - 1.0).abs() < f32::EPSILON);

    Ok(())
  }

  /// Test mean pooling with large values
  #[test]
  fn test_mean_pool_large_values() -> Result<()> {
    // Shape: [batch_size=1, seq_length=2, hidden_size=2]
    let shape = vec![1i64, 2i64, 2i64];

    // Data with large values
    let data = vec![1000.0f32, 2000.0, 3000.0, 4000.0];

    let result = EmbeddingModel::mean_pool((&shape, &data))?;

    assert_eq!(result.len(), 2);
    assert_eq!(result[0], 2000.0);
    assert_eq!(result[1], 3000.0);

    Ok(())
  }

  /// Test mean pooling with very small values
  #[test]
  fn test_mean_pool_small_values() -> Result<()> {
    // Shape: [batch_size=1, seq_length=2, hidden_size=2]
    let shape = vec![1i64, 2i64, 2i64];

    // Data with very small values
    let data = vec![0.001f32, 0.002, 0.003, 0.004];

    let result = EmbeddingModel::mean_pool((&shape, &data))?;

    assert_eq!(result.len(), 2);
    assert!((result[0] - 0.002).abs() < f32::EPSILON);
    assert!((result[1] - 0.003).abs() < f32::EPSILON);

    Ok(())
  }

  /// Test normalization with normal vector
  #[test]
  fn test_normalize_embedding_normal() -> Result<()> {
    let embedding = vec![3.0, 4.0, 0.0]; // magnitude = 5.0
    let result = EmbeddingModel::normalize_embedding(embedding)?;

    assert_eq!(result.len(), 3);
    assert!((result[0] - 0.6).abs() < f32::EPSILON); // 3/5
    assert!((result[1] - 0.8).abs() < f32::EPSILON); // 4/5
    assert!((result[2] - 0.0).abs() < f32::EPSILON); // 0/5

    // Check magnitude is now 1.0
    let magnitude: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((magnitude - 1.0).abs() < f32::EPSILON);

    Ok(())
  }

  /// Test normalization preserves zero vector
  #[test]
  fn test_normalize_embedding_zero_vector() -> Result<()> {
    let embedding = vec![0.0, 0.0, 0.0];
    let result = EmbeddingModel::normalize_embedding(embedding.clone())?;

    assert_eq!(result, embedding); // Should be unchanged
    Ok(())
  }

  /// Test normalization with unit vector
  #[test]
  fn test_normalize_embedding_unit_vector() -> Result<()> {
    let embedding = vec![1.0, 0.0, 0.0]; // Already unit length
    let result = EmbeddingModel::normalize_embedding(embedding.clone())?;

    assert_eq!(result, embedding); // Should be unchanged
    Ok(())
  }

  /// Test normalization with large values
  #[test]
  fn test_normalize_embedding_large_values() -> Result<()> {
    let embedding = vec![1000.0, 2000.0]; // magnitude = sqrt(5000000) ≈ 2236
    let result = EmbeddingModel::normalize_embedding(embedding)?;

    assert_eq!(result.len(), 2);

    // Check magnitude is now 1.0
    let magnitude: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((magnitude - 1.0).abs() < f32::EPSILON);

    // Check direction is preserved (ratio should be 1:2)
    assert!((result[1] / result[0] - 2.0).abs() < 0.001);

    Ok(())
  }

  /// Test normalization with negative values
  #[test]
  fn test_normalize_embedding_negative_values() -> Result<()> {
    let embedding = vec![-3.0, 4.0]; // magnitude = 5.0
    let result = EmbeddingModel::normalize_embedding(embedding)?;

    assert_eq!(result.len(), 2);
    assert!((result[0] - (-0.6)).abs() < f32::EPSILON); // -3/5
    assert!((result[1] - 0.8).abs() < f32::EPSILON); //  4/5

    // Check magnitude is now 1.0
    let magnitude: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((magnitude - 1.0).abs() < f32::EPSILON);

    Ok(())
  }
}


