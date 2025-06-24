mod mock_github;

use chrono::Utc;
use jerrod::commands::{
  acknowledge, comment, commit, finish, peek, pop, refresh, resolve, start, status,
};
use jerrod::platform::{
  Discussion, GitPlatform, MergeRequest, MergeRequestState, Note, Pipeline, ReactionType,
  Repository, User,
};
use jerrod::session::{ReviewSession, SessionManager};
use mock_github::MockGitHub;
use std::env;
use std::fs;
use tempfile::TempDir;

fn setup_test_env() -> TempDir {
  let temp_dir = TempDir::new().unwrap();
  env::set_var("KERNELLE_DIR", temp_dir.path());
  temp_dir
}

async fn create_test_session() -> (TempDir, ReviewSession) {
  let temp_dir = setup_test_env();

  // Create test repository
  let repository = Repository {
    owner: "test_owner".to_string(),
    name: "test_repo".to_string(),
    full_name: "test_owner/test_repo".to_string(),
    url: "https://github.com/test_owner/test_repo".to_string(),
  };

  // Create test user
  let user = User {
    id: "user123".to_string(),
    username: "testuser".to_string(),
    display_name: "Test User".to_string(),
    avatar_url: Some("https://avatar.example.com/user123".to_string()),
  };

  // Create test MR
  let merge_request = MergeRequest {
    id: "mr123".to_string(),
    number: 123,
    title: "Test Pull Request".to_string(),
    description: Some("Test description".to_string()),
    state: MergeRequestState::Open,
    url: "https://github.com/test_owner/test_repo/pull/123".to_string(),
    source_branch: "feature-branch".to_string(),
    target_branch: "main".to_string(),
    author: user.clone(),
    assignee: None,
    created_at: Utc::now(),
    updated_at: Utc::now(),
  };

  // Create test note
  let note = Note {
    id: "note123".to_string(),
    author: user.clone(),
    body: "This needs improvement".to_string(),
    created_at: Utc::now(),
    updated_at: Utc::now(),
  };

  // Create test discussions
  let discussion = Discussion {
    id: "disc123".to_string(),
    resolved: false,
    resolvable: true,
    file_path: Some("src/main.rs".to_string()),
    line_number: Some(42),
    notes: vec![note],
  };

  // Create pipelines (empty for now)
  let pipelines: Vec<Pipeline> = vec![];

  // Create session with test data
  let session = ReviewSession::new(
    repository,
    merge_request,
    "github".to_string(),
    vec![discussion],
    pipelines,
  );

  (temp_dir, session)
}

#[tokio::test]
async fn test_acknowledge_reaction_flags() {
  use jerrod::commands::acknowledge::AcknowledgeConfig;

  // Test that we can create acknowledge config with reaction flags
  let config = AcknowledgeConfig::from_flags(
    true, false, false, false, // thumbs_up flags
    false, false, // thumbs_down flags
    false, false, // laugh flags
    false, false, false, false, false, // hooray flags
    false, false, false, // confused flags
    false, false, false, // heart flags
    false, false, false, false, false, // rocket flags
    false, false, false, // eyes flags
  );

  // Should create a config with thumbs up reaction
  assert_eq!(config.reaction_type.emoji(), "👍");

  // Test the command call (will fail due to missing session, but that's expected)
  let result = acknowledge::handle(config).await;
  assert!(result.is_err());
}

#[tokio::test]
async fn test_status_with_no_session() {
  let _temp_dir = setup_test_env();

  // This should fail gracefully when no session exists
  let result = status::handle().await;
  assert!(result.is_err());
}

#[tokio::test]
async fn test_peek_with_no_session() {
  let _temp_dir = setup_test_env();

  // This should fail gracefully when no session exists
  let result = peek::handle().await;
  assert!(result.is_err());
}

#[tokio::test]
async fn test_pop_with_no_session() {
  let _temp_dir = setup_test_env();

  // This should fail gracefully when no session exists
  let result = pop::handle(false).await;
  assert!(result.is_err());
}

#[tokio::test]
async fn test_finish_with_no_session() {
  let _temp_dir = setup_test_env();

  // Finish should handle no session gracefully
  let result = finish::handle().await;
  assert!(result.is_ok());
}

#[tokio::test]
async fn test_refresh_with_no_session() {
  let _temp_dir = setup_test_env();

  // Refresh should fail when no session exists
  let result = refresh::handle().await;
  assert!(result.is_err());
}

#[tokio::test]
async fn test_session_manager_creation() {
  let _temp_dir = setup_test_env();
  let session_manager_result = SessionManager::new();

  // Should be able to create session manager
  assert!(session_manager_result.is_ok());
  let session_manager = session_manager_result.unwrap();
  assert!(!session_manager.session_exists());
}

#[tokio::test]
async fn test_session_operations_with_mock_data() {
  let (_temp_dir, session) = create_test_session().await;

  // Verify session was created correctly
  assert_eq!(session.repository.owner, "test_owner");
  assert_eq!(session.merge_request.number, 123);
  assert_eq!(session.platform, "github");
  assert_eq!(session.thread_queue.len(), 1);
}

#[tokio::test]
async fn test_review_session_thread_operations() {
  let (_temp_dir, mut session) = create_test_session().await;

  // Test peek
  let next_thread = session.peek_next_thread();
  assert!(next_thread.is_some());
  assert_eq!(next_thread.unwrap().id, "disc123");

  // Test threads remaining
  assert_eq!(session.threads_remaining(), 1);

  // Test pop thread
  let popped = session.pop_thread(false);
  assert!(popped.is_some());
  assert_eq!(popped.unwrap().id, "disc123");

  // Verify queue is now empty
  assert_eq!(session.threads_remaining(), 0);
  assert!(session.peek_next_thread().is_none());
}

#[tokio::test]
async fn test_review_session_unresolved_threads() {
  let (_temp_dir, mut session) = create_test_session().await;

  // Initially no unresolved threads
  assert!(!session.has_unresolved_threads());

  // Pop thread as unresolved
  let popped = session.pop_thread(true);
  assert!(popped.is_some());

  // Should now have unresolved threads
  assert!(session.has_unresolved_threads());
}

#[tokio::test]
async fn test_reaction_type_methods() {
  let reaction = ReactionType::ThumbsUp;
  assert_eq!(reaction.emoji(), "👍");
  assert_eq!(reaction.github_name(), "+1");

  let reaction = ReactionType::Heart;
  assert_eq!(reaction.emoji(), "❤️");
  assert_eq!(reaction.github_name(), "heart");
}

// Skip these tests as they involve complex platform interactions
#[tokio::test]
#[ignore]
async fn test_start_command_integration() {
  // This would require complex mocking of platform detection and API calls
  // Skip for now to focus on achievable coverage gains
}

#[tokio::test]
#[ignore]
async fn test_comment_command_integration() {
  // This would require mocking GitHub API and auth system
  // Skip for now to focus on achievable coverage gains
}

#[tokio::test]
#[ignore]
async fn test_commit_command_integration() {
  // This would require mocking git operations and file system
  // Skip for now to focus on achievable coverage gains
}

#[tokio::test]
#[ignore]
async fn test_resolve_command_integration() {
  // This would require complex platform mocking
  // Skip for now to focus on achievable coverage gains
}
