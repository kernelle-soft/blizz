use anyhow::{anyhow, Result};
use std::path::Path;

#[cfg(feature = "neural")]
use ort::{session::{Session, builder::GraphOptimizationLevel}, value::TensorRef};
#[cfg(feature = "neural")]
use tokenizers::Tokenizer;

/// Trait for computing text embeddings - allows for testing with mocks
pub trait EmbeddingModel {
    fn compute_embeddings(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

/// Real ONNX-based embedding model implementation
#[cfg(feature = "neural")]
pub struct OnnxEmbeddingModel {
    session: Session,
    tokenizer: Tokenizer,
}

#[cfg(feature = "neural")]
impl OnnxEmbeddingModel {
    /// Initialize the ONNX model and tokenizer
    pub async fn new() -> Result<Self> {
        // Initialize ONNX Runtime
        ort::init()
            .with_name("blizz-model")
            .commit()
            .map_err(|e| anyhow!("Failed to initialize ONNX Runtime: {}", e))?;
        
        // Load model once at startup
        let session = Session::builder()
            .map_err(|e| anyhow!("Failed to create session builder: {}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level1)
            .map_err(|e| anyhow!("Failed to set optimization level: {}", e))?
            .with_intra_threads(1)
            .map_err(|e| anyhow!("Failed to set thread count: {}", e))?
            .commit_from_url("https://cdn.pyke.io/0/pyke:ort-rs/example-models@0.0.0/all-MiniLM-L6-v2.onnx")
            .map_err(|e| anyhow!("Failed to load model: {}", e))?;

        // Load tokenizer
        let tokenizer_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("tokenizer.json");
        
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;

        Ok(Self { session, tokenizer })
    }
}

#[cfg(feature = "neural")]
impl EmbeddingModel for OnnxEmbeddingModel {
    fn compute_embeddings(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        
        // Use encode_batch for efficient processing of multiple texts  
        let encodings = self.tokenizer.encode_batch(texts.to_vec(), false)
            .map_err(|e| anyhow!("Failed to encode texts: {}", e))?;
        
        // Get the padded length (all encodings are padded to same length)
        let padded_token_length = encodings[0].len();
        
        // Flatten all token IDs and attention masks for batched inference
        let ids: Vec<i64> = encodings.iter()
            .flat_map(|e| e.get_ids().iter().map(|&id| id as i64))
            .collect();
        let mask: Vec<i64> = encodings.iter()
            .flat_map(|e| e.get_attention_mask().iter().map(|&mask| mask as i64))
            .collect();
        
        // Create tensors with shape [batch_size, sequence_length]
        let ids_tensor = TensorRef::from_array_view(([texts.len(), padded_token_length], &*ids))?;
        let mask_tensor = TensorRef::from_array_view(([texts.len(), padded_token_length], &*mask))?;
        
        // Run batched inference
        let outputs = self.session.run(ort::inputs![ids_tensor, mask_tensor])?;
        
        // Extract embeddings from output (index 1 for sentence transformers contains pooled embeddings)
        let embedding_output = if outputs.len() > 1 { &outputs[1] } else { &outputs[0] };
        let embeddings = embedding_output
            .try_extract_array::<f32>()?
            .into_dimensionality::<ndarray::Ix2>()?;
        
        // Extract each embedding from the batch
        let mut result = Vec::new();
        for i in 0..texts.len() {
            let embedding_view = embeddings.index_axis(ndarray::Axis(0), i);
            let embedding_vec: Vec<f32> = embedding_view.iter().copied().collect();
            result.push(embedding_vec);
        }
        
        Ok(result)
    }
}

/// Mock embedding model for testing
pub struct MockEmbeddingModel {
    pub fail_on_texts: Vec<String>,
    pub response_embeddings: Vec<Vec<f32>>,
}

impl Default for MockEmbeddingModel {
    fn default() -> Self {
        Self::new()
    }
}

impl MockEmbeddingModel {
    pub fn new() -> Self {
        Self {
            fail_on_texts: vec![],
            response_embeddings: vec![vec![0.1, 0.2, 0.3]; 10], // Default mock embeddings
        }
    }
    
    pub fn with_failure_on(mut self, text: String) -> Self {
        self.fail_on_texts.push(text);
        self
    }
    
    pub fn with_embeddings(mut self, embeddings: Vec<Vec<f32>>) -> Self {
        self.response_embeddings = embeddings;
        self
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
        
        // Return mock embeddings (cycle through available ones)
        let mut result = Vec::new();
        for (i, _text) in texts.iter().enumerate() {
            let embedding_index = i % self.response_embeddings.len();
            result.push(self.response_embeddings[embedding_index].clone());
        }
        
        Ok(result)
    }
}

/// Factory function to create the appropriate model for the environment
#[cfg(feature = "neural")]
pub async fn create_production_model() -> Result<OnnxEmbeddingModel> {
    OnnxEmbeddingModel::new().await
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