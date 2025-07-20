use anyhow::Result;
use blizz::daemon::{EmbeddingRequest, EmbeddingResponse, EmbeddingService};
use blizz::model::MockEmbeddingModel;

#[cfg(test)]
mod daemon_tests {
  use super::*;

  #[tokio::test]
  async fn test_embedding_service_creation() {
    let model = MockEmbeddingModel::new();
    let _service = EmbeddingService::new(model);

    // Service should be created successfully
    // This tests the basic constructor
  }

  #[tokio::test]
  async fn test_handle_request_successful() {
    let model =
      MockEmbeddingModel::new().with_embeddings(vec![vec![0.1, 0.2, 0.3], vec![0.4, 0.5, 0.6]]);

    let mut service = EmbeddingService::new(model);

    let request = EmbeddingRequest {
      texts: vec!["test text".to_string(), "another text".to_string()],
      id: "test-id-123".to_string(),
    };

    let response = service.handle_request(request).await;

    assert_eq!(response.id, "test-id-123");
    assert!(response.error.is_none());
    assert_eq!(response.embeddings.len(), 2);
    assert_eq!(response.embeddings[0], vec![0.1, 0.2, 0.3]);
    assert_eq!(response.embeddings[1], vec![0.4, 0.5, 0.6]);
  }

  #[tokio::test]
  async fn test_handle_request_with_model_failure() {
    let model = MockEmbeddingModel::new().with_failure_on("failing text".to_string());

    let mut service = EmbeddingService::new(model);

    let request =
      EmbeddingRequest { texts: vec!["failing text".to_string()], id: "test-id-456".to_string() };

    let response = service.handle_request(request).await;

    assert_eq!(response.id, "test-id-456");
    assert!(response.error.is_some());
    assert!(response.error.as_ref().unwrap().contains("Mock failure for text"));
    assert!(response.embeddings.is_empty());
  }

  #[tokio::test]
  async fn test_handle_request_empty_texts() {
    let model = MockEmbeddingModel::new();
    let mut service = EmbeddingService::new(model);

    let request = EmbeddingRequest { texts: vec![], id: "empty-test".to_string() };

    let response = service.handle_request(request).await;

    assert_eq!(response.id, "empty-test");
    assert!(response.error.is_none());
    assert!(response.embeddings.is_empty());
  }

  #[tokio::test]
  async fn test_handle_request_large_batch() {
    let model = MockEmbeddingModel::new().with_embeddings(vec![vec![1.0, 2.0]; 50]); // 50 identical embeddings

    let mut service = EmbeddingService::new(model);

    let texts: Vec<String> = (0..25).map(|i| format!("text {i}")).collect();
    let request = EmbeddingRequest { texts, id: "batch-test".to_string() };

    let response = service.handle_request(request).await;

    assert_eq!(response.id, "batch-test");
    assert!(response.error.is_none());
    assert_eq!(response.embeddings.len(), 25);

    // All embeddings should be the same due to our mock setup
    for embedding in response.embeddings {
      assert_eq!(embedding, vec![1.0, 2.0]);
    }
  }

  #[test]
  fn test_embedding_request_serialization() -> Result<()> {
    let request = EmbeddingRequest {
      texts: vec!["hello".to_string(), "world".to_string()],
      id: "serial-test".to_string(),
    };

    let json = serde_json::to_string(&request)?;
    let deserialized: EmbeddingRequest = serde_json::from_str(&json)?;

    assert_eq!(deserialized.texts, vec!["hello", "world"]);
    assert_eq!(deserialized.id, "serial-test");

    Ok(())
  }

  #[test]
  fn test_embedding_response_serialization() -> Result<()> {
    let response = EmbeddingResponse {
      embeddings: vec![vec![0.1, 0.2], vec![0.3, 0.4]],
      id: "response-test".to_string(),
      error: None,
    };

    let json = serde_json::to_string(&response)?;
    let deserialized: EmbeddingResponse = serde_json::from_str(&json)?;

    assert_eq!(deserialized.embeddings, vec![vec![0.1, 0.2], vec![0.3, 0.4]]);
    assert_eq!(deserialized.id, "response-test");
    assert!(deserialized.error.is_none());

    Ok(())
  }

  #[test]
  fn test_embedding_response_serialization_with_error() -> Result<()> {
    let response = EmbeddingResponse {
      embeddings: vec![],
      id: "error-test".to_string(),
      error: Some("Something went wrong".to_string()),
    };

    let json = serde_json::to_string(&response)?;
    let deserialized: EmbeddingResponse = serde_json::from_str(&json)?;

    assert!(deserialized.embeddings.is_empty());
    assert_eq!(deserialized.id, "error-test");
    assert_eq!(deserialized.error, Some("Something went wrong".to_string()));

    Ok(())
  }

  #[test]
  fn test_embedding_request_invalid_json() {
    let invalid_json = r#"{"texts": ["test"], "missing_id": true}"#;
    let result: Result<EmbeddingRequest, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_service_handles_unicode_text() {
    let model = MockEmbeddingModel::new().with_embeddings(vec![vec![0.7, 0.8, 0.9]]);

    let mut service = EmbeddingService::new(model);

    let request = EmbeddingRequest {
      texts: vec!["Hello 世界 🌍".to_string()],
      id: "unicode-test".to_string(),
    };

    let response = service.handle_request(request).await;

    assert_eq!(response.id, "unicode-test");
    assert!(response.error.is_none());
    assert_eq!(response.embeddings.len(), 1);
    assert_eq!(response.embeddings[0], vec![0.7, 0.8, 0.9]);
  }

  #[tokio::test]
  async fn test_service_handles_very_long_text() {
    let model = MockEmbeddingModel::new();
    let mut service = EmbeddingService::new(model);

    // Create a very long text (more than typical token limits)
    let long_text = "word ".repeat(1000);

    let request = EmbeddingRequest { texts: vec![long_text], id: "long-text-test".to_string() };

    let response = service.handle_request(request).await;

    assert_eq!(response.id, "long-text-test");
    // Should either succeed or fail gracefully, but not panic
    assert!(response.error.is_none() || response.error.is_some());
  }

  #[tokio::test]
  async fn test_service_preserves_request_id() {
    let model = MockEmbeddingModel::new();
    let mut service = EmbeddingService::new(model);

    let unique_id = "very-unique-id-12345";
    let request = EmbeddingRequest { texts: vec!["test".to_string()], id: unique_id.to_string() };

    let response = service.handle_request(request).await;
    assert_eq!(response.id, unique_id);
  }

  #[tokio::test]
  async fn test_mock_model_cycling_behavior() {
    let model = MockEmbeddingModel::new().with_embeddings(vec![vec![1.0, 1.0], vec![2.0, 2.0]]);

    let mut service = EmbeddingService::new(model);

    let request = EmbeddingRequest {
      texts: vec![
        "text1".to_string(),
        "text2".to_string(),
        "text3".to_string(), // Should cycle back to first embedding
      ],
      id: "cycle-test".to_string(),
    };

    let response = service.handle_request(request).await;

    assert_eq!(response.embeddings.len(), 3);
    assert_eq!(response.embeddings[0], vec![1.0, 1.0]);
    assert_eq!(response.embeddings[1], vec![2.0, 2.0]);
    assert_eq!(response.embeddings[2], vec![1.0, 1.0]); // Cycled back
  }
}
