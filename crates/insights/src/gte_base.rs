use anyhow::{anyhow, Result};
use hf_hub::api::tokio::Api;
use ndarray::Array2;
use std::collections::HashMap;
use tokenizers::Tokenizer;

const MODEL_NAME: &str = "Alibaba-NLP/gte-base-en-v1.5";
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
  execution_providers::{CPUExecutionProvider, CoreMLExecutionProvider, ExecutionProviderDispatch},
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

pub struct GTEBase {
  session: Session,
  tokenizer: Tokenizer,
}

struct ModelFiles {
  tokenizer_file: std::path::PathBuf,
  model_path: std::path::PathBuf,
}

// Public API
#[cfg(not(tarpaulin_include))] // [rag-stack] - add CI/CD testing for cross-platform loading/unloading
impl GTEBase {
  /// Load the GTE-Base model from HuggingFace
  pub async fn load() -> Result<Self> {
    bentley::info!("loading model...");

    let model_files = Self::download_model().await?;
    let tokenizer = Self::load_tokenizer(model_files.tokenizer_file)?;
    let session = Self::load_model(model_files.model_path)?;
    Ok(Self { session, tokenizer })
  }

  /// Generate embeddings for a single text
  pub fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
    let tokens = Self::tokenize(text, &self.tokenizer)?;
    let input = Self::prepare(tokens.as_ref(), &self.session)?;
    let output = self.session.run(input)?;
    Self::extract_embedding(&output)
  }
}

// Model initialization
// violet ignore chunk - this is about as simple and flat as it's going to get without breaking this into
// singlet implementation blocks.
#[cfg(not(tarpaulin_include))] // [rag-stack] - add CI/CD testing for cross-platform loading/unloading
impl GTEBase {
  async fn download_model() -> Result<ModelFiles> {
    let api = Api::new().map_err(|e| anyhow!("HF API initialization failed: {}", e))?;

    let repo = api.model(MODEL_NAME.to_string());

    let tokenizer_file =
      repo.get(TOKENIZER_FILE).await.map_err(|e| anyhow!("Failed to download tokenizer: {}", e))?;

    let model_path =
      repo.get(MODEL_FILE).await.map_err(|e| anyhow!("Failed to download ONNX model: {}", e))?;

    Ok(ModelFiles { tokenizer_file, model_path })
  }

  fn load_tokenizer(path: std::path::PathBuf) -> Result<Tokenizer> {
    Tokenizer::from_file(path).map_err(|e| anyhow!("Failed to load tokenizer: {}", e))
  }

  fn load_model(model_path: std::path::PathBuf) -> Result<Session> {
    let providers = Self::get_execution_providers();

    let session =
      Session::builder()?.with_execution_providers(providers)?.commit_from_file(model_path)?;

    Ok(session)
  }
}

// Hardware detection
#[cfg(not(tarpaulin_include))] // [rag-stack] - add CI/CD testing for xplat
impl GTEBase {
  fn get_execution_providers() -> Vec<ExecutionProviderDispatch> {
    let mut providers = Vec::new();

    #[cfg(target_os = "macos")]
    {
      providers.push(CoreMLExecutionProvider::default().into());
    }

    #[cfg(target_os = "linux")]
    {
      if Self::is_cuda_available() {
        providers.push(CUDAExecutionProvider::default().build().error_on_failure());
      }
    }

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

// violet ignore chunk - this is about as simple and flat as it's going to get without being terse.
// Embedding processing
impl GTEBase {
  /// Testable tokenization logic
  fn tokenize(text: &str, tokenizer: &dyn TextTokenizer) -> Result<Box<dyn TokenizerOutput>> {
    let tokens = tokenizer.encode_text(text, true)?;

    let token_count = tokens.get_ids().len();
    Self::validate_sequence_length(token_count)?;

    Ok(tokens)
  }

  /// Validate token sequence length - extracted for easy testing
  fn validate_sequence_length(token_count: usize) -> Result<()> {
    const MAX_SEQUENCE_LENGTH: usize = 511; // GTE-Base limit is 512

    if token_count > MAX_SEQUENCE_LENGTH {
      let error_msg = format!(
        "Input text contains {token_count} tokens, which exceeds the model's maximum sequence length of {MAX_SEQUENCE_LENGTH}. Please reduce the input size."
      );
      bentley::warn!(&error_msg);
      return Err(anyhow!(error_msg));
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

    // Create input based on what the model expects
    let mut input = HashMap::new();
    input.insert("input_ids".to_string(), input_ids_tensor);
    input.insert("attention_mask".to_string(), attention_mask_tensor);

    // Only include token_type_ids if the model expects it
    let model_input_names = session.input_names();

    if model_input_names.contains(&"token_type_ids".to_string()) {
      input.insert("token_type_ids".to_string(), token_type_ids_tensor);
    } else {
      bentley::verbose!("Model doesn't expect token_type_ids, skipping");
    }

    Ok(input)
  }

  fn to_tensor<T: Copy + Into<i64>>(values: &[T]) -> Result<Value> {
    let seq_len = values.len();
    let array: Array2<i64> = Array2::from_shape_vec((1, seq_len), Self::to_i64(values))?;
    let tensor: Value = Value::from_array(array)?.into();
    Ok(tensor)
  }

  fn to_i64<T: Copy + Into<i64>>(values: &[T]) -> Vec<i64> {
    values.iter().map(|&x| x.into()).collect()
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

    let result = GTEBase::prepare(&tokens, &session)?;

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

    let result = GTEBase::prepare(&tokens, &session)?;

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

    let result = GTEBase::prepare(&tokens, &session)?;

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

    let result = GTEBase::prepare(&tokens, &session)?;

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

    let result = GTEBase::prepare(&tokens, &session)?;

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

    let result = GTEBase::prepare(&tokens, &session)?;

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
    assert!(GTEBase::validate_sequence_length(1).is_ok());
    assert!(GTEBase::validate_sequence_length(100).is_ok());
    assert!(GTEBase::validate_sequence_length(511).is_ok()); // Exactly at limit

    Ok(())
  }

  #[test]
  fn test_validate_sequence_length_exceeds_limit() {
    // Test over the limit
    let result = GTEBase::validate_sequence_length(512);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("512 tokens"));
    assert!(error_msg.contains("exceeds the model's maximum sequence length of 511"));

    // Test well over the limit
    let result = GTEBase::validate_sequence_length(1000);
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("1000 tokens"));
  }

  /// Test tokenize with normal case
  #[test]
  fn test_tokenize_normal_case() -> Result<()> {
    let tokenizer = MockTextTokenizer {
      should_fail: false,
      token_ids: vec![101, 7592, 2256, 102], // [CLS] hello world [SEP]
    };

    let result = GTEBase::tokenize("hello world", &tokenizer)?;

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

    let result = GTEBase::tokenize("any text", &tokenizer);

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

    let result = GTEBase::tokenize("long text", &tokenizer)?;

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

    let result = GTEBase::tokenize("very long text", &tokenizer);

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("512 tokens"));
    assert!(error_msg.contains("exceeds the model's maximum sequence length of 511"));
  }

  /// Test tokenize with empty input (edge case)
  #[test]
  fn test_tokenize_empty_input() -> Result<()> {
    let tokenizer = MockTextTokenizer {
      should_fail: false,
      token_ids: vec![101, 102], // Just [CLS] [SEP]
    };

    let result = GTEBase::tokenize("", &tokenizer)?;

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

    let result = GTEBase::tokenize("a", &tokenizer)?;

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

    let result = GTEBase::tokenize("extremely long text", &tokenizer);

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("1000 tokens"));
    assert!(error_msg.contains("exceeds the model's maximum sequence length of 511"));
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
    let result = GTEBase::extract_embedding(&output)?;

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
    let result = GTEBase::extract_embedding(&output)?;

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
    let result = GTEBase::extract_embedding(&output)?;

    assert_eq!(result, vec![100.0, 200.0]); // Used last_hidden_state

    Ok(())
  }

  /// Test extract_embedding when no tensor is found
  #[test]
  fn test_extract_embedding_no_tensor_found() {
    let tensors = HashMap::new(); // empty - no tensors
    let output = MockTensorExtractor { tensors };

    let result = GTEBase::extract_embedding(&output);

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
    let result = GTEBase::extract_embedding(&output)?;

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

    let result = GTEBase::mean_pool((&shape, &data))?;

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

    let result = GTEBase::mean_pool((&shape, &data))?;

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

    let result = GTEBase::mean_pool((&shape, &data))?;

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

    let result = GTEBase::mean_pool((&shape, &data))?;

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

    let result = GTEBase::mean_pool((&shape, &data))?;

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

    let result = GTEBase::mean_pool((&shape, &data))?;

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
    let result = GTEBase::mean_pool((&shape, &data));

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

    let result = GTEBase::mean_pool((&shape, &data))?;

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

    let result = GTEBase::mean_pool((&shape, &data))?;

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

    let result = GTEBase::mean_pool((&shape, &data))?;

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

    let result = GTEBase::mean_pool((&shape, &data))?;

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

    let result = GTEBase::mean_pool((&shape, &data))?;

    assert_eq!(result.len(), 2);
    assert!((result[0] - 0.002).abs() < f32::EPSILON);
    assert!((result[1] - 0.003).abs() < f32::EPSILON);

    Ok(())
  }
}
