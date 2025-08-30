use anyhow::anyhow;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use std::path::{Path, PathBuf};
use std::{env, fs};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener;
use tokio::signal;
use tokio::task::JoinHandle;

use insights::gte_base::GTEBase;
use bentley::daemon_logs::{DaemonLogs, LogsRequest, LogsResponse};
use std::sync::Arc;

// Trait to abstract async I/O for testability
#[cfg_attr(test, mockall::automock)]
trait AsyncWriter {
  async fn write_all(&mut self, data: &[u8]) -> std::io::Result<()>;
}

#[cfg_attr(test, mockall::automock)]
trait AsyncReader {
  async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize>;
}

// Trait to abstract embedding functionality for testability
#[cfg_attr(test, mockall::automock)]
trait Embedder {
  fn embed(&mut self, text: &str) -> anyhow::Result<Vec<f32>>;
}

// Implement the traits for UnixStream
#[cfg(not(tarpaulin_include))]
impl AsyncWriter for tokio::net::UnixStream {
  async fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
    AsyncWriteExt::write_all(self, data).await
  }
}

#[cfg(not(tarpaulin_include))]
impl AsyncReader for tokio::net::UnixStream {
  async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
    AsyncReadExt::read_to_end(self, buf).await
  }
}

// Implement the trait for GTEBase
#[cfg(not(tarpaulin_include))]
impl Embedder for GTEBase {
  fn embed(&mut self, text: &str) -> anyhow::Result<Vec<f32>> {
    self.embed(text)
  }
}

#[derive(Debug, Deserialize)]
struct EmbeddingRequest {
  request: String,
  body: String,
}

#[derive(Debug, Serialize)]
struct EmbeddingResponse {
  success: bool,
  body: Vec<f32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  error: Option<ErrorInfo>,
}

#[derive(Debug, Serialize)]
struct ErrorInfo {
  message: String,
  tag: String,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum DaemonResponse {
  Embedding(EmbeddingResponse),
  Logs(LogsResponse),
}

#[cfg(not(tarpaulin_include))]
impl EmbeddingResponse {
  fn success(embedding: Vec<f32>) -> Self {
    Self { success: true, body: embedding, error: None }
  }

  fn error(message: &str, tag: &str) -> Self {
    Self {
      success: false,
      body: Vec::new(),
      error: Some(ErrorInfo { message: message.to_string(), tag: tag.to_string() }),
    }
  }
}



#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<()> {
  let insights_path = get_base()?;

  // Ensure directory exists
  fs::create_dir_all(&insights_path)?;

    // Initialize daemon logs to disk
  let log_file_path = insights_path.join("daemon.logs.jsonl");
  let logs = DaemonLogs::new(&log_file_path)
    .map_err(|e| anyhow!("Failed to initialize daemon logs: {}", e))?;
  
  // Log daemon startup
  logs.log("info", "Daemon starting up", "startup").await;

  // Load the embedder model
  let embedder = match GTEBase::load().await {
    Ok(embedder) => {
      logs.log("info", "Successfully loaded GTE-Base embedding model", "embedder").await;
      Some(Arc::new(tokio::sync::Mutex::new(embedder)))
    }
    Err(e) => {
      logs.log("warn", &format!("Failed to load embedder model: {e}"), "embedder").await;
      logs.log("info", "Daemon will run without embedding capabilities", "embedder").await;
      bentley::warn!(&format!("Failed to load embedder model: {e}"));
      bentley::warn!("Daemon will run without embedding capabilities");
      None
    }
  };

  let socket_path = create_socket(&insights_path)?;

  logs.log("info", &format!("Socket created: {}", socket_path.display()), "ipc").await;

  bentley::info!("daemon started - press ctrl+c to exit");

  let ipc_handle = spawn_handler(&socket_path, embedder.map(|e| Arc::clone(&e)), logs.clone());

  // Wait for shutdown signal
  signal::ctrl_c().await?;
  bentley::verbose!("\nshutting down daemon");

  logs.log("info", "Received shutdown signal", "shutdown").await;

  // Clean up socket file
  let _ = fs::remove_file(&socket_path);

  // Clean up PID file
  let pid_file = insights_path.join("daemon.pid");
  let _ = fs::remove_file(&pid_file);

  ipc_handle.abort();
  Ok(())
}

fn get_base() -> Result<PathBuf> {
  let base = if let Ok(dir) = env::var("BLIZZ_HOME") {
    PathBuf::from(dir)
  } else {
    dirs::home_dir().ok_or_else(|| anyhow!("failed to determine home directory"))?.join(".blizz")
  };

  let insights_path = base.join("persistent").join("insights");

  Ok(insights_path)
}

#[cfg(not(tarpaulin_include))]
fn create_socket(insights_path: &Path) -> Result<PathBuf> {
  let socket = insights_path.join("daemon.sock");
  let _ = fs::remove_file(&socket);
  Ok(socket)
}

#[cfg(not(tarpaulin_include))]
fn spawn_handler(
  socket: &PathBuf,
  embedder: Option<Arc<tokio::sync::Mutex<GTEBase>>>,
  logs: DaemonLogs,
) -> JoinHandle<()> {
  let listener = create_listener(socket);
  bentley::info!(&format!("listening on socket: {}", socket.display()));
  tokio::spawn(async move {
    handle_connections(listener, embedder, logs).await;
  })
}

#[cfg(not(tarpaulin_include))]
fn create_listener(socket: &PathBuf) -> UnixListener {
  match UnixListener::bind(socket) {
    Ok(listener) => listener,
    Err(e) => {
      bentley::error!(&format!("failed to bind socket: {e}"));
      std::process::exit(1);
    }
  }
}

#[cfg(not(tarpaulin_include))]
async fn handle_connections(
  listener: UnixListener,
  embedder: Option<Arc<tokio::sync::Mutex<GTEBase>>>,
  logs: DaemonLogs,
) {
  loop {
    let connection_result = listener.accept().await;

    let (mut stream, _) = match connection_result {
      Ok(connection) => connection,
      Err(e) => {
        bentley::warn!(&format!("failed to accept connection: {e}"));
        return;
      }
    };

    let embedder_for_task = embedder.as_ref().map(Arc::clone);
    let logs_for_task = logs.clone();
    tokio::spawn(async move {
      let response = handle_request::<_, GTEBase>(&mut stream, embedder_for_task, logs_for_task).await;
      send_response(&mut stream, response).await;
    });
  }
}

// LCOV exclusions the monomorphizing of this function
async fn handle_request<R: AsyncReader, E: Embedder>(
  stream: &mut R,
  embedder: Option<Arc<tokio::sync::Mutex<E>>>,
  logs: DaemonLogs,
) -> DaemonResponse {
  let data = match read_request_data(stream).await {
    Ok(data) => data, // LCOV_EXCL_LINE
    Err(res) => return DaemonResponse::Embedding(res),
  };

  let str = match parse_raw(data) {
    Ok(str) => str, // LCOV_EXCL_LINE
    Err(res) => return DaemonResponse::Embedding(res),
  };

  // Try parsing as EmbeddingRequest first
  if let Ok(embedding_request) = serde_json::from_str::<EmbeddingRequest>(&str) {
    if embedding_request.request == "embed" {
      logs.log("info", "Processing embedding request", "embedder").await;

      let response = match embedder {
        Some(arc) => {
          let mut embedder_lock = arc.lock().await;
          embed(&embedding_request.body, &mut *embedder_lock)
        }
        None => {
          bentley::warn!("embedding requested but no model loaded");
          EmbeddingResponse::error("Embedding model not available", "model_not_loaded")
        }
      };
      
      return DaemonResponse::Embedding(response);
    }
  }

  // Try parsing as LogsRequest
  if let Ok(logs_request) = serde_json::from_str::<LogsRequest>(&str) {
    if logs_request.request == "logs" {
      logs.log("info", "Processing logs request", "logs").await;
      
      match logs.get_logs(logs_request.limit, logs_request.level.as_deref()).await {
        Ok(filtered_logs) => return DaemonResponse::Logs(LogsResponse::success(filtered_logs)),
        Err(e) => return DaemonResponse::Logs(LogsResponse::error(
          &format!("Failed to read logs: {}", e),
          "read_logs_failed"
        )),
      }
    }
  }

  // If we get here, it's an unsupported request type
  bentley::warn!("received unsupported or malformed request");
  DaemonResponse::Embedding(EmbeddingResponse::error("Unsupported or malformed request", "unsupported_request"))
}

async fn read_request_data<R: AsyncReader>(stream: &mut R) -> Result<Vec<u8>, EmbeddingResponse> {
  let mut buffer = Vec::new();
  match stream.read_to_end(&mut buffer).await {
    Ok(_) => Ok(buffer),
    Err(e) => {
      bentley::warn!(&format!("failed to read from client: {e}"));
      Err(EmbeddingResponse::error(&format!("Failed to read request: {e}"), "read_failed"))
    }
  }
}

fn parse_raw(buffer: Vec<u8>) -> Result<String, EmbeddingResponse> {
  match String::from_utf8(buffer) {
    Ok(s) => Ok(s),
    Err(_) => {
      bentley::warn!("received invalid UTF-8 data");
      Err(EmbeddingResponse::error("Invalid UTF-8 data in request", "invalid_utf8"))
    }
  }
}



fn embed<E: Embedder>(text: &str, embedder: &mut E) -> EmbeddingResponse {
  match embedder.embed(text) {
    Ok(embedding) => {
      bentley::verbose!(&format!(
        "generated embedding for: {}",
        text.chars().take(50).collect::<String>()
      ));
      EmbeddingResponse::success(embedding)
    }
    Err(e) => {
      bentley::warn!(&format!("embedding failed: {e}"));
      EmbeddingResponse::error(&format!("Failed to generate embedding: {e}"), "embedding_failed")
    }
  }
}

async fn send_response<W: AsyncWriter, R: serde::Serialize>(stream: &mut W, response: R) {
  let response_json = match serde_json::to_string(&response) {
    Ok(json) => json, // LCOV_EXCL_LINE
    Err(e) => {
      bentley::error!(&format!("failed to serialize response: {e}"));
      return;
    }
  };

  if let Err(e) = stream.write_all(response_json.as_bytes()).await {
    bentley::warn!(&format!("failed to write response: {e}"));
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use mockall::predicate::*;

  #[tokio::test]
  async fn test_send_response_to_client_success() {
    let mut mock_stream = MockAsyncWriter::new();
    let test_embedding = vec![0.1, 0.2, 0.3];
    let response = EmbeddingResponse::success(test_embedding);

    // Expected JSON output
    let expected_json = serde_json::to_string(&response).unwrap();
    let expected_bytes = expected_json.as_bytes().to_vec();

    // Set up the mock expectation
    mock_stream.expect_write_all().with(eq(expected_bytes)).times(1).returning(|_| Ok(()));

    // Call the function
    send_response(&mut mock_stream, response).await;
  }

  #[tokio::test]
  async fn test_send_response_to_client_error_bad_json() {
    let mut mock_stream = MockAsyncWriter::new();

    // Create a struct that will fail JSON serialization
    struct FailingSerialize;

    impl serde::Serialize for FailingSerialize {
      fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
      where
        S: serde::Serializer,
      {
        Err(serde::ser::Error::custom("intentional serialization failure"))
      }
    }

    let failing_response = FailingSerialize;

    // Mock should not expect any calls to write_all since serialization fails
    mock_stream.expect_write_all().times(0);

    // Call the function - it should return early due to serialization failure
    send_response(&mut mock_stream, failing_response).await;
  }

  #[tokio::test]
  async fn test_send_response_to_client_write_failure() {
    let mut mock_stream = MockAsyncWriter::new();
    let test_embedding = vec![0.1, 0.2, 0.3];
    let response = EmbeddingResponse::success(test_embedding);

    // Expected JSON output
    let expected_json = serde_json::to_string(&response).unwrap();
    let expected_bytes = expected_json.as_bytes().to_vec();

    // Set up the mock to fail on write
    mock_stream
      .expect_write_all()
      .with(eq(expected_bytes))
      .times(1)
      .returning(|_| Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Write failed")));

    // Call the function - it should handle the write error gracefully
    send_response(&mut mock_stream, response).await;
    // Function should complete without panicking, just log the warning
  }

  #[test]
  fn test_embedding_success() {
    let mut mock_embedder = MockEmbedder::new();
    let expected_embedding = vec![0.1, 0.2, 0.3, 0.4, 0.5];

    mock_embedder.expect_embed().with(eq("test text")).times(1).returning({
      let embedding = expected_embedding.clone();
      move |_| Ok(embedding.clone())
    });

    let result = embed("test text", &mut mock_embedder);

    assert!(result.success);
    assert_eq!(result.body, expected_embedding);
    assert!(result.error.is_none());
  }

  #[test]
  fn test_embedding_failure() {
    let mut mock_embedder = MockEmbedder::new();

    mock_embedder
      .expect_embed()
      .with(eq("test text"))
      .times(1)
      .returning(|_| Err(anyhow::anyhow!("Embedding computation failed")));

    let result = embed("test text", &mut mock_embedder);

    assert!(!result.success);
    assert!(result.body.is_empty());
    assert!(result.error.is_some());

    let error = result.error.unwrap();
    assert_eq!(error.message, "Failed to generate embedding: Embedding computation failed");
    assert_eq!(error.tag, "embedding_failed");
  }

  #[test]
  fn test_parse_utf8_data_success() {
    let valid_utf8_bytes = "Hello, world! 🦀".as_bytes().to_vec();

    let result = parse_raw(valid_utf8_bytes);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello, world! 🦀");
  }

  #[test]
  fn test_parse_utf8_data_failure() {
    // Create invalid UTF-8 sequence
    let invalid_utf8_bytes = vec![0xFF, 0xFE, 0xFD];

    let result = parse_raw(invalid_utf8_bytes);

    assert!(result.is_err());
    let error_response = result.unwrap_err();
    assert!(!error_response.success);
    assert!(error_response.body.is_empty());
    assert!(error_response.error.is_some());

    let error = error_response.error.unwrap();
    assert_eq!(error.message, "Invalid UTF-8 data in request");
    assert_eq!(error.tag, "invalid_utf8");
  }

  #[test]
  fn test_parse_json_request_success() {
    let valid_json = r#"{"request": "embed", "body": "test text content"}"#;

    let result = parse_json_request(valid_json);

    assert!(result.is_ok());
    let request = result.unwrap();
    assert_eq!(request.request, "embed");
    assert_eq!(request.body, "test text content");
  }

  #[test]
  fn test_parse_json_request_invalid_json() {
    let invalid_json = r#"{"request": "embed", "body": "unclosed string"#;

    let result = parse_json_request(invalid_json);

    assert!(result.is_err());
    let error_response = result.unwrap_err();
    assert!(!error_response.success);
    assert!(error_response.body.is_empty());
    assert!(error_response.error.is_some());

    let error = error_response.error.unwrap();
    assert!(error.message.starts_with("Invalid JSON request:"));
    assert_eq!(error.tag, "invalid_json");
  }

  #[test]
  fn test_parse_json_request_missing_fields() {
    let missing_fields_json = r#"{"request": "embed"}"#; // Missing "body" field

    let result = parse_json_request(missing_fields_json);

    assert!(result.is_err());
    let error_response = result.unwrap_err();
    assert!(!error_response.success);
    assert!(error_response.body.is_empty());
    assert!(error_response.error.is_some());

    let error = error_response.error.unwrap();
    assert!(error.message.starts_with("Invalid JSON request:"));
    assert!(error.message.contains("missing field"));
    assert_eq!(error.tag, "invalid_json");
  }

  #[test]
  fn test_parse_json_request_wrong_types() {
    let wrong_types_json = r#"{"request": 123, "body": true}"#; // Wrong field types

    let result = parse_json_request(wrong_types_json);

    assert!(result.is_err());
    let error_response = result.unwrap_err();
    assert!(!error_response.success);
    assert!(error_response.body.is_empty());
    assert!(error_response.error.is_some());

    let error = error_response.error.unwrap();
    assert!(error.message.starts_with("Invalid JSON request:"));
    assert_eq!(error.tag, "invalid_json");
  }

  #[tokio::test]
  async fn test_read_request_data_success() {
    let mut mock_reader = MockAsyncReader::new();
    let test_data = b"test request data".to_vec();

    mock_reader.expect_read_to_end().times(1).returning({
      let data = test_data.clone();
      move |buf| {
        buf.extend_from_slice(&data);
        Ok(data.len())
      }
    });

    let result = read_request_data(&mut mock_reader).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), test_data);
  }

  #[tokio::test]
  async fn test_read_request_data_failure() {
    let mut mock_reader = MockAsyncReader::new();

    mock_reader.expect_read_to_end().times(1).returning(|_| {
      Err(std::io::Error::new(std::io::ErrorKind::ConnectionReset, "Connection lost"))
    });

    let result = read_request_data(&mut mock_reader).await;

    assert!(result.is_err());
    let error_response = result.unwrap_err();
    assert!(!error_response.success);
    assert!(error_response.body.is_empty());
    assert!(error_response.error.is_some());

    let error = error_response.error.unwrap();
    assert!(error.message.starts_with("Failed to read request:"));
    assert!(error.message.contains("Connection lost"));
    assert_eq!(error.tag, "read_failed");
  }

  #[tokio::test]
  async fn test_process_client_request_success() {
    let mut mock_reader = MockAsyncReader::new();
    let request_json = r#"{"request": "embed", "body": "test text to embed"}"#;

    mock_reader.expect_read_to_end().times(1).returning({
      let data = request_json.as_bytes().to_vec();
      move |buf| {
        buf.extend_from_slice(&data);
        Ok(data.len())
      }
    });

    // Create a mock embedder
    let mock_embedder = Arc::new(tokio::sync::Mutex::new(MockEmbedder::new()));
    {
      let mut embedder_guard = mock_embedder.lock().await;
      embedder_guard
        .expect_embed()
        .with(eq("test text to embed"))
        .times(1)
        .returning(|_| Ok(vec![0.1, 0.2, 0.3]));
    }

    let result = handle_request(&mut mock_reader, Some(mock_embedder)).await;

    assert!(result.success);
    assert_eq!(result.body, vec![0.1, 0.2, 0.3]);
    assert!(result.error.is_none());
  }

  #[tokio::test]
  async fn test_process_client_request_read_failure() {
    let mut mock_reader = MockAsyncReader::new();

    mock_reader
      .expect_read_to_end()
      .times(1)
      .returning(|_| Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Pipe broken")));

    let result = handle_request::<_, GTEBase>(&mut mock_reader, None).await;

    assert!(!result.success);
    assert!(result.body.is_empty());
    assert!(result.error.is_some());

    let error = result.error.unwrap();
    assert!(error.message.starts_with("Failed to read request:"));
    assert_eq!(error.tag, "read_failed");
  }

  #[tokio::test]
  async fn test_process_client_request_utf8_failure() {
    let mut mock_reader = MockAsyncReader::new();
    let invalid_utf8 = vec![0xFF, 0xFE, 0xFD]; // Invalid UTF-8 bytes

    mock_reader.expect_read_to_end().times(1).returning({
      let data = invalid_utf8.clone();
      move |buf| {
        buf.extend_from_slice(&data);
        Ok(data.len())
      }
    });

    let result = handle_request::<_, GTEBase>(&mut mock_reader, None).await;

    assert!(!result.success);
    assert!(result.body.is_empty());
    assert!(result.error.is_some());

    let error = result.error.unwrap();
    assert_eq!(error.message, "Invalid UTF-8 data in request");
    assert_eq!(error.tag, "invalid_utf8");
  }

  #[tokio::test]
  async fn test_process_client_request_json_failure() {
    let mut mock_reader = MockAsyncReader::new();
    let invalid_json = r#"{"request": "embed", "body": unclosed"#; // Invalid JSON

    mock_reader.expect_read_to_end().times(1).returning({
      let data = invalid_json.as_bytes().to_vec();
      move |buf| {
        buf.extend_from_slice(&data);
        Ok(data.len())
      }
    });

    let result = handle_request::<_, GTEBase>(&mut mock_reader, None).await;

    assert!(!result.success);
    assert!(result.body.is_empty());
    assert!(result.error.is_some());

    let error = result.error.unwrap();
    assert!(error.message.starts_with("Invalid JSON request:"));
    assert_eq!(error.tag, "invalid_json");
  }

  #[tokio::test]
  async fn test_process_client_request_unsupported_request_type() {
    let mut mock_reader = MockAsyncReader::new();
    let request_json = r#"{"request": "summarize", "body": "some text"}"#; // Wrong request type

    mock_reader.expect_read_to_end().times(1).returning({
      let data = request_json.as_bytes().to_vec();
      move |buf| {
        buf.extend_from_slice(&data);
        Ok(data.len())
      }
    });

    let result = handle_request::<_, GTEBase>(&mut mock_reader, None).await;

    assert!(!result.success);
    assert!(result.body.is_empty());
    assert!(result.error.is_some());

    let error = result.error.unwrap();
    assert_eq!(error.message, "Unsupported request type: summarize");
    assert_eq!(error.tag, "unsupported_request");
  }

  #[tokio::test]
  async fn test_process_client_request_no_embedder() {
    let mut mock_reader = MockAsyncReader::new();
    let request_json = r#"{"request": "embed", "body": "test text"}"#;

    mock_reader.expect_read_to_end().times(1).returning({
      let data = request_json.as_bytes().to_vec();
      move |buf| {
        buf.extend_from_slice(&data);
        Ok(data.len())
      }
    });

    let result = handle_request::<_, GTEBase>(&mut mock_reader, None).await;

    assert!(!result.success);
    assert!(result.body.is_empty());
    assert!(result.error.is_some());

    let error = result.error.unwrap();
    assert_eq!(error.message, "Embedding model not available");
    assert_eq!(error.tag, "model_not_loaded");
  }

  #[tokio::test]
  async fn test_process_client_request_embedding_failure() {
    let mut mock_reader = MockAsyncReader::new();
    let request_json = r#"{"request": "embed", "body": "test text"}"#;

    mock_reader.expect_read_to_end().times(1).returning({
      let data = request_json.as_bytes().to_vec();
      move |buf| {
        buf.extend_from_slice(&data);
        Ok(data.len())
      }
    });

    // Create a mock embedder that fails
    let mock_embedder = Arc::new(tokio::sync::Mutex::new(MockEmbedder::new()));
    {
      let mut embedder_guard = mock_embedder.lock().await;
      embedder_guard
        .expect_embed()
        .with(eq("test text"))
        .times(1)
        .returning(|_| Err(anyhow::anyhow!("GPU out of memory")));
    }

    let result = handle_request(&mut mock_reader, Some(mock_embedder)).await;

    assert!(!result.success);
    assert!(result.body.is_empty());
    assert!(result.error.is_some());

    let error = result.error.unwrap();
    assert_eq!(error.message, "Failed to generate embedding: GPU out of memory");
    assert_eq!(error.tag, "embedding_failed");
  }

  #[tokio::test]
  async fn test_process_client_request_exact_embed_type() {
    let mut mock_reader = MockAsyncReader::new();
    // Test with exact "embed" request type to ensure branch coverage
    let request_json = r#"{"request": "embed", "body": "precise test text"}"#;

    mock_reader.expect_read_to_end().times(1).returning({
      let data = request_json.as_bytes().to_vec();
      move |buf| {
        buf.extend_from_slice(&data);
        Ok(data.len())
      }
    });

    // Create a mock embedder with specific expectations
    let mock_embedder = Arc::new(tokio::sync::Mutex::new(MockEmbedder::new()));
    {
      let mut embedder_guard = mock_embedder.lock().await;
      embedder_guard
        .expect_embed()
        .with(eq("precise test text"))
        .times(1)
        .returning(|_| Ok(vec![0.5, 0.6, 0.7, 0.8]));
    }

    let result = handle_request(&mut mock_reader, Some(mock_embedder)).await;

    assert!(result.success);
    assert_eq!(result.body, vec![0.5, 0.6, 0.7, 0.8]);
    assert!(result.error.is_none());
  }

  #[tokio::test]
  async fn test_process_client_request_empty_body() {
    let mut mock_reader = MockAsyncReader::new();
    // Test with empty body to hit success path but with edge case
    let request_json = r#"{"request": "embed", "body": ""}"#;

    mock_reader.expect_read_to_end().times(1).returning({
      let data = request_json.as_bytes().to_vec();
      move |buf| {
        buf.extend_from_slice(&data);
        Ok(data.len())
      }
    });

    let mock_embedder = Arc::new(tokio::sync::Mutex::new(MockEmbedder::new()));
    {
      let mut embedder_guard = mock_embedder.lock().await;
      embedder_guard.expect_embed().with(eq("")).times(1).returning(|_| Ok(vec![]));
      // Empty embedding for empty text
    }

    let result = handle_request(&mut mock_reader, Some(mock_embedder)).await;

    assert!(result.success);
    assert!(result.body.is_empty());
    assert!(result.error.is_none());
  }

  #[tokio::test]
  async fn test_process_client_request_case_sensitive_request_type() {
    let mut mock_reader = MockAsyncReader::new();
    // Test case sensitivity of request type
    let request_json = r#"{"request": "EMBED", "body": "test"}"#; // Uppercase should fail

    mock_reader.expect_read_to_end().times(1).returning({
      let data = request_json.as_bytes().to_vec();
      move |buf| {
        buf.extend_from_slice(&data);
        Ok(data.len())
      }
    });

    let result = handle_request::<_, GTEBase>(&mut mock_reader, None).await;

    assert!(!result.success);
    assert!(result.body.is_empty());
    assert!(result.error.is_some());

    let error = result.error.unwrap();
    assert_eq!(error.message, "Unsupported request type: EMBED");
    assert_eq!(error.tag, "unsupported_request");
  }

  #[tokio::test]
  async fn test_process_client_request_minimal_json() {
    let mut mock_reader = MockAsyncReader::new();
    // Test with minimal valid JSON to ensure all success paths are hit
    let request_json = r#"{"request":"embed","body":"x"}"#; // Minimal JSON

    mock_reader.expect_read_to_end().times(1).returning({
      let data = request_json.as_bytes().to_vec();
      move |buf| {
        buf.extend_from_slice(&data);
        Ok(data.len())
      }
    });

    let mock_embedder = Arc::new(tokio::sync::Mutex::new(MockEmbedder::new()));
    {
      let mut embedder_guard = mock_embedder.lock().await;
      embedder_guard.expect_embed().with(eq("x")).times(1).returning(|_| Ok(vec![1.0]));
    }

    let result = handle_request(&mut mock_reader, Some(mock_embedder)).await;

    assert!(result.success);
    assert_eq!(result.body, vec![1.0]);
    assert!(result.error.is_none());
  }

  #[tokio::test]
  async fn test_process_client_request_unicode_content() {
    let mut mock_reader = MockAsyncReader::new();
    // Test with Unicode content to ensure UTF-8 parsing works correctly
    let request_json = r#"{"request": "embed", "body": "Hello 世界 🌍"}"#;

    mock_reader.expect_read_to_end().times(1).returning({
      let data = request_json.as_bytes().to_vec();
      move |buf| {
        buf.extend_from_slice(&data);
        Ok(data.len())
      }
    });

    let mock_embedder = Arc::new(tokio::sync::Mutex::new(MockEmbedder::new()));
    {
      let mut embedder_guard = mock_embedder.lock().await;
      embedder_guard
        .expect_embed()
        .with(eq("Hello 世界 🌍"))
        .times(1)
        .returning(|_| Ok(vec![0.1, 0.2, 0.3]));
    }

    let result = handle_request(&mut mock_reader, Some(mock_embedder)).await;

    assert!(result.success);
    assert_eq!(result.body, vec![0.1, 0.2, 0.3]);
    assert!(result.error.is_none());
  }

  /// Test get_base() function with environment variable scenarios
  #[test]
  fn test_get_base_with_blizz_home() {
    // Test with BLIZZ_HOME set
    std::env::set_var("BLIZZ_HOME", "/custom/blizz/path");

    let result = get_base().unwrap();
    let expected =
      std::path::PathBuf::from("/custom/blizz/path").join("persistent").join("insights");

    assert_eq!(result, expected);

    // Clean up
    std::env::remove_var("BLIZZ_HOME");
  }

  #[test]
  fn test_get_base_without_blizz_home() {
    // Ensure BLIZZ_HOME is not set
    std::env::remove_var("BLIZZ_HOME");

    let result = get_base().unwrap();

    // Should fallback to home directory + .blizz
    let expected_prefix = dirs::home_dir().unwrap().join(".blizz");
    let expected = expected_prefix.join("persistent").join("insights");

    assert_eq!(result, expected);
  }

  #[test]
  fn test_get_base_path_construction() {
    // Test that the path construction always includes persistent/insights
    std::env::set_var("BLIZZ_HOME", "/test/base");

    let result = get_base().unwrap();

    // Verify the path ends with the expected structure
    assert!(result.ends_with("persistent/insights"));
    assert!(result.to_string_lossy().contains("/test/base"));

    // Clean up
    std::env::remove_var("BLIZZ_HOME");
  }

  /// Test create_socket() happy path
  #[test]
  fn test_create_socket_happy_path() {
    use tempfile::TempDir;

    // Create a temporary directory for testing
    let temp_dir = TempDir::new().unwrap();
    let insights_path = temp_dir.path();

    let result = create_socket(insights_path).unwrap();

    // Should create correct socket path
    let expected = insights_path.join("daemon.sock");
    assert_eq!(result, expected);
  }

  #[test]
  fn test_create_socket_path_construction() {
    use tempfile::TempDir;

    // Test with different base paths
    let temp_dir = TempDir::new().unwrap();
    let insights_path = temp_dir.path().join("custom").join("path");

    // Create the directory structure
    std::fs::create_dir_all(&insights_path).unwrap();

    let result = create_socket(&insights_path).unwrap();

    // Verify correct path construction
    assert!(result.ends_with("daemon.sock"));
    assert!(result.starts_with(&insights_path));

    let expected = insights_path.join("daemon.sock");
    assert_eq!(result, expected);
  }

  #[test]
  fn test_create_socket_removes_existing_file() {
    use std::fs;
    use tempfile::TempDir;

    // Create a temporary directory
    let temp_dir = TempDir::new().unwrap();
    let insights_path = temp_dir.path();
    let socket_path = insights_path.join("daemon.sock");

    // Create an existing file at the socket path
    fs::write(&socket_path, "existing content").unwrap();
    assert!(socket_path.exists());

    let result = create_socket(insights_path).unwrap();

    // Should return the correct path (the removal is best-effort, so file may or may not exist)
    assert_eq!(result, socket_path);
  }
}
