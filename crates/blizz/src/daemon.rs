use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::time::{timeout, Duration};

#[cfg(feature = "neural")]
use ort::{session::{Session, builder::GraphOptimizationLevel}, value::TensorRef};
#[cfg(feature = "neural")]
use tokenizers::Tokenizer;

/// Request to compute embeddings (supports batching!)
#[derive(Serialize, Deserialize)]
struct EmbeddingRequest {
    texts: Vec<String>,  // Changed to support multiple texts
    id: String,
}

/// Response with computed embeddings (supports batching!)
#[derive(Serialize, Deserialize)]
struct EmbeddingResponse {
    embeddings: Vec<Vec<f32>>,  // Changed to support multiple embeddings
    id: String,
    error: Option<String>,
}

/// Embedding service that keeps model loaded in memory
struct EmbeddingService {
    #[cfg(feature = "neural")]
    session: Session,
    #[cfg(feature = "neural")]
    tokenizer: Tokenizer,
}

impl EmbeddingService {
    /// Initialize the service by loading the model once
    #[cfg(feature = "neural")]
    async fn new() -> Result<Self> {
        // Initialize ONNX Runtime
        ort::init()
            .with_name("blizz-daemon")
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

        println!("✅ Blizz embedding daemon started - model loaded and ready");

        Ok(Self { session, tokenizer })
    }
    
    #[cfg(not(feature = "neural"))]
    async fn new() -> Result<Self> {
        Err(anyhow!("Neural features not enabled"))
    }

    /// Compute embeddings for given texts (supports batching!)
    #[cfg(feature = "neural")]
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
    
    #[cfg(not(feature = "neural"))]
    fn compute_embeddings(&mut self, _texts: &[String]) -> Result<Vec<Vec<f32>>> {
        Err(anyhow!("Neural features not enabled"))
    }

    /// Handle incoming embedding request
    async fn handle_request(&mut self, request: EmbeddingRequest) -> EmbeddingResponse {
        match self.compute_embeddings(&request.texts) {
            Ok(embeddings) => EmbeddingResponse {
                embeddings,
                id: request.id,
                error: None,
            },
            Err(e) => EmbeddingResponse {
                embeddings: vec![],
                id: request.id,
                error: Some(e.to_string()),
            },
        }
    }
}

/// Handle a client connection
async fn handle_client(mut stream: UnixStream, service: &mut EmbeddingService) -> Result<()> {
    let mut reader = BufReader::new(&mut stream);
    let mut line = String::new();
    
    // Read request line
    reader.read_line(&mut line).await?;
    
    // Parse request
    let request: EmbeddingRequest = serde_json::from_str(&line.trim())?;
    
    // Compute embedding
    let response = service.handle_request(request).await;
    
    // Send response
    let response_json = serde_json::to_string(&response)?;
    stream.write_all(response_json.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    
    Ok(())
}

/// Socket path for IPC
const SOCKET_PATH: &str = "/tmp/blizz-embeddings.sock";

#[tokio::main]
async fn main() -> Result<()> {
    // Clean up any existing socket
    let _ = std::fs::remove_file(SOCKET_PATH);
    
    // Create the embedding service (loads model)
    let mut service = EmbeddingService::new().await?;
    
    // Bind to Unix socket
    let listener = UnixListener::bind(SOCKET_PATH)?;
    println!("🚀 Blizz daemon listening on {}", SOCKET_PATH);
    
    // Auto-shutdown after 5 minutes of inactivity
    let inactivity_timeout = Duration::from_secs(300);
    
    loop {
        // Wait for connection with timeout
        match timeout(inactivity_timeout, listener.accept()).await {
            Ok(Ok((stream, _))) => {
                // Handle request
                if let Err(e) = handle_client(stream, &mut service).await {
                    eprintln!("Error handling client: {}", e);
                }
            }
            Ok(Err(e)) => {
                eprintln!("Error accepting connection: {}", e);
                break;
            }
            Err(_) => {
                // Timeout - shutdown due to inactivity
                println!("💤 Blizz daemon shutting down due to inactivity");
                break;
            }
        }
    }
    
    // Clean up socket
    let _ = std::fs::remove_file(SOCKET_PATH);
    
    Ok(())
} 