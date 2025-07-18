use anyhow::Result;
use jerrod::auth::{
  get_github_token, get_gitlab_token, register_provider_factory, reset_provider_factory,
};
use sentinel::CredentialProvider;
use serial_test::serial;

// Mock Sentinel implementation for testing
struct MockSentinel {
  credentials: std::collections::HashMap<String, String>,
}

impl MockSentinel {
  fn new() -> Self {
    let mut credentials = std::collections::HashMap::new();
    credentials.insert("github:token".to_string(), "mock_github_token".to_string());
    credentials.insert("gitlab:token".to_string(), "mock_gitlab_token".to_string());

    Self { credentials }
  }
}

impl CredentialProvider for MockSentinel {
  fn get_credential(&self, service: &str, key: &str) -> Result<String> {
    let lookup_key = format!("{service}:{key}");
    self
      .credentials
      .get(&lookup_key)
      .cloned()
      .ok_or_else(|| anyhow::anyhow!("Credential not found: {}", lookup_key))
  }

  fn store_credential(&self, _service: &str, _key: &str, _value: &str) -> Result<()> {
    // Mock store - do nothing
    Ok(())
  }
}

#[tokio::test]
#[serial]
async fn test_mock_github_token() {
  // Clean up first
  reset_provider_factory();

  // Register mock factory
  register_provider_factory(|| Box::new(MockSentinel::new()));

  let token = get_github_token().await.unwrap();
  assert_eq!(token, "mock_github_token");

  // Clean up
  reset_provider_factory();
}

#[tokio::test]
#[serial]
async fn test_mock_gitlab_token() {
  // Clean up first
  reset_provider_factory();

  // Register mock factory
  register_provider_factory(|| Box::new(MockSentinel::new()));

  let token = get_gitlab_token().await.unwrap();
  assert_eq!(token, "mock_gitlab_token");

  // Clean up
  reset_provider_factory();
}

#[tokio::test]
#[serial]
async fn test_mock_with_custom_credentials() {
  // Clean up first
  reset_provider_factory();

  // Create a custom mock with different credentials
  register_provider_factory(|| {
    let mut credentials = std::collections::HashMap::new();
    credentials.insert("github:token".to_string(), "custom_github_token".to_string());
    credentials.insert("gitlab:token".to_string(), "custom_gitlab_token".to_string());

    Box::new(MockSentinel { credentials })
  });

  let github_token = get_github_token().await.unwrap();
  let gitlab_token = get_gitlab_token().await.unwrap();

  assert_eq!(github_token, "custom_github_token");
  assert_eq!(gitlab_token, "custom_gitlab_token");

  // Clean up
  reset_provider_factory();
}

#[tokio::test]
#[serial]
async fn test_mock_error_handling() {
  // Clean up first
  reset_provider_factory();

  // Register a mock that always fails
  register_provider_factory(|| {
    Box::new(MockSentinel {
            credentials: std::collections::HashMap::new(), // Empty credentials
        })
  });

  let result = get_github_token().await;
  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("Credential not found"));

  // Clean up
  reset_provider_factory();
}

#[tokio::test]
#[serial]
async fn test_factory_persistence_across_calls() {
  // Clean up any previous state first
  reset_provider_factory();

  // Register mock factory
  register_provider_factory(|| Box::new(MockSentinel::new()));

  // Multiple calls should all use the same mock
  let token1 = get_github_token().await.unwrap();
  let token2 = get_github_token().await.unwrap();
  let token3 = get_gitlab_token().await.unwrap();

  assert_eq!(token1, "mock_github_token");
  assert_eq!(token2, "mock_github_token");
  assert_eq!(token3, "mock_gitlab_token");

  // Clean up
  reset_provider_factory();
}
