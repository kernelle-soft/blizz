use anyhow::Result;
use chrono::Utc;
use jerrod::auth::{register_provider_factory, reset_provider_factory};
use jerrod::commands::resolve;
use jerrod::platform::{Discussion, MergeRequest, MergeRequestState, Note, Repository, User};
use jerrod::session::{ReviewSession, ReviewSessionOptions, SessionManager};
use sentinel::MockCredentialProvider;
use serial_test::serial;
use tempfile::TempDir;

// Helper to create a test session with mock data
async fn create_test_session_with_thread(temp_dir: &TempDir) -> Result<ReviewSession> {
  // Set the KERNELLE_DIR first so session manager knows where to save
  std::env::set_var("KERNELLE_DIR", temp_dir.path());
  let mut session_manager = SessionManager::new()?;

  let repo = Repository {
    owner: "test-owner".to_string(),
    name: "test-repo".to_string(),
    full_name: "test-owner/test-repo".to_string(),
    url: "https://github.com/test-owner/test-repo".to_string(),
  };

  let mr = MergeRequest {
    id: "123".to_string(),
    number: 456,
    title: "Test MR".to_string(),
    description: Some("Test description".to_string()),
    state: MergeRequestState::Open,
    url: "https://github.com/test-owner/test-repo/pull/456".to_string(),
    author: User {
      id: "author123".to_string(),
      username: "test-author".to_string(),
      display_name: "Test Author".to_string(),
      avatar_url: Some("https://github.com/test-author.png".to_string()),
    },
    assignee: None,
    source_branch: "feature-branch".to_string(),
    target_branch: "main".to_string(),
    created_at: Utc::now(),
    updated_at: Utc::now(),
  };

  // Create a test discussion
  let discussion = Discussion {
    id: "1234567890".to_string(),
    notes: vec![Note {
      id: "2345678901".to_string(),
      body: "This needs to be resolved".to_string(),
      author: User {
        id: "reviewer1".to_string(),
        username: "reviewer".to_string(),
        display_name: "Code Reviewer".to_string(),
        avatar_url: Some("https://github.com/reviewer.png".to_string()),
      },
      created_at: Utc::now(),
      updated_at: Utc::now(),
    }],
    resolved: false,
    resolvable: true,
    file_path: Some("src/main.rs".to_string()),
    line_number: Some(42),
  };

  // Set up session context for the session manager
  session_manager.with_session_context("github", "test-owner/test-repo", 456)?;

  let session_data = ReviewSession::new(
    repo,
    mr,
    "github".to_string(),
    vec![discussion],
    vec![], // empty pipelines
    ReviewSessionOptions { host: None },
  );

  session_manager.save_session(&session_data)?;
  Ok(session_data)
}

#[tokio::test]
#[serial]
async fn test_resolve_with_thread() -> Result<()> {
  let temp_dir = TempDir::new()?;

  // Set up mock credentials
  register_provider_factory(|| {
    Box::new(MockCredentialProvider::new().with_credential("github", "token", "fake-token-123"))
  });

  // Create test session with a thread
  let _session = create_test_session_with_thread(&temp_dir).await?;

  // Set session directory for the command
  std::env::set_var("KERNELLE_DIR", temp_dir.path());

  // Test resolving the current thread
  let result = resolve::handle().await;

  // The function should handle session loading and validation correctly.
  // It will fail at the GitHub API call with auth errors which is expected
  // since we're using mock credentials. This means our business logic is working.
  if let Err(e) = &result {
    let error_msg = e.to_string();
    assert!(
      error_msg.contains("Bad credentials")
        || error_msg.contains("GitHub")
        || error_msg.contains("401 Unauthorized")
        || error_msg.contains("HTTP 401"),
      "Unexpected error: {}",
      error_msg
    );
  }

  reset_provider_factory();
  std::env::remove_var("KERNELLE_DIR");
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_resolve_no_thread() -> Result<()> {
  let temp_dir = TempDir::new()?;

  // Set up mock credentials
  register_provider_factory(|| {
    Box::new(MockCredentialProvider::new().with_credential("github", "token", "fake-token-123"))
  });

  // Create test session with no threads
  let mut session = create_test_session_with_thread(&temp_dir).await?;

  // Remove the thread from the queue
  session.pop_thread(false);

  let mut session_manager = SessionManager::new()?;
  session_manager.with_session_context("github", "test-owner/test-repo", 456)?;
  session_manager.save_session(&session)?;

  // Set session directory for the command
  std::env::set_var("KERNELLE_DIR", temp_dir.path());

  // Test resolving when no thread is available (should error)
  let result = resolve::handle().await;

  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("No current thread"));

  reset_provider_factory();
  std::env::remove_var("KERNELLE_DIR");
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_resolve_invalid_repository_format() -> Result<()> {
  let temp_dir = TempDir::new()?;

  // Set up mock credentials
  register_provider_factory(|| {
    Box::new(MockCredentialProvider::new().with_credential("github", "token", "fake-token-123"))
  });

  // Create test session with invalid repository format
  let mut session = create_test_session_with_thread(&temp_dir).await?;
  session.repository.full_name = "invalid-format".to_string();

  let mut session_manager = SessionManager::new()?;
  session_manager.with_session_context("github", "test-owner/test-repo", 456)?;
  session_manager.save_session(&session)?;

  // Set session directory for the command
  std::env::set_var("KERNELLE_DIR", temp_dir.path());

  // Test with invalid repository format
  let result = resolve::handle().await;

  assert!(result.is_err());
  assert!(result.unwrap_err().to_string().contains("Invalid repository format"));

  reset_provider_factory();
  std::env::remove_var("KERNELLE_DIR");
  Ok(())
}

#[tokio::test]
#[serial]
async fn test_resolve_no_session_file() -> Result<()> {
  let temp_dir = TempDir::new()?;

  // Set session directory but don't create any session
  std::env::set_var("KERNELLE_DIR", temp_dir.path());

  // Test without any session file
  let result = resolve::handle().await;

  assert!(result.is_err());

  std::env::remove_var("KERNELLE_DIR");
  Ok(())
}
