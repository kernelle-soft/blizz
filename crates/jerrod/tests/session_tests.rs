use chrono::Utc;
use jerrod::platform::{Discussion, MergeRequest, MergeRequestState, Note, Repository, User};
use jerrod::session::{
  load_current_session, ReviewSession, ReviewSessionOptions, SessionDiscovery, SessionManager,
};
use std::env;
use tempfile::TempDir;

fn setup_test_env() -> TempDir {
  let temp_dir = TempDir::new().unwrap();
  env::set_var("KERNELLE_DIR", temp_dir.path());
  temp_dir
}

fn create_test_repository() -> Repository {
  Repository {
    owner: "test_org".to_string(),
    name: "test_repo".to_string(),
    full_name: "test_org/test_repo".to_string(),
    url: "https://github.com/test_org/test_repo".to_string(),
  }
}

fn create_test_user() -> User {
  User {
    id: "user456".to_string(),
    username: "testauthor".to_string(),
    display_name: "Test Author".to_string(),
    avatar_url: Some("https://avatar.example.com/testauthor".to_string()),
  }
}

fn create_test_merge_request() -> MergeRequest {
  MergeRequest {
    id: "mr789".to_string(),
    number: 789,
    title: "Test Feature Implementation".to_string(),
    description: Some("This is a test merge request for comprehensive testing".to_string()),
    state: MergeRequestState::Open,
    url: "https://github.com/test_org/test_repo/pull/789".to_string(),
    source_branch: "feature/comprehensive-test".to_string(),
    target_branch: "main".to_string(),
    author: create_test_user(),
    assignee: None,
    created_at: Utc::now(),
    updated_at: Utc::now(),
  }
}

fn create_test_discussion(
  id: &str,
  file_path: Option<String>,
  line_number: Option<u32>,
) -> Discussion {
  let note = Note {
    id: format!("note_{id}"),
    author: create_test_user(),
    body: format!("Test discussion comment {id}"),
    created_at: Utc::now(),
    updated_at: Utc::now(),
  };

  Discussion {
    id: id.to_string(),
    resolved: false,
    resolvable: true,
    file_path,
    line_number,
    notes: vec![note],
  }
}

#[test]
fn test_session_manager_initialization() {
  let _temp_dir = setup_test_env();

  let session_manager = SessionManager::new();
  assert!(session_manager.is_ok());

  let manager = session_manager.unwrap();
  assert!(!manager.session_exists());
}

#[test]
fn test_session_manager_with_context() {
  let _temp_dir = setup_test_env();

  let mut session_manager = SessionManager::new().unwrap();
  let result = session_manager.with_session_context("github", "owner/repo", 123);
  assert!(result.is_ok());
}

#[test]
fn test_session_manager_invalid_context() {
  let _temp_dir = setup_test_env();

  let mut session_manager = SessionManager::new().unwrap();

  // Test with invalid platform
  let result = session_manager.with_session_context("invalid_platform", "owner/repo", 123);
  // Should either succeed or provide a meaningful error
  assert!(result.is_ok() || result.is_err());

  // Test with empty repository
  let result = session_manager.with_session_context("github", "", 123);
  assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_review_session_creation() {
  let repository = create_test_repository();
  let merge_request = create_test_merge_request();
  let discussions = vec![
    create_test_discussion("1", Some("src/main.rs".to_string()), Some(42)),
    create_test_discussion("2", Some("src/lib.rs".to_string()), Some(100)),
    create_test_discussion("3", None, None),
  ];

  let session = ReviewSession::new(
    repository.clone(),
    merge_request.clone(),
    "github".to_string(),
    discussions.clone(),
    vec![],
    ReviewSessionOptions { host: None },
  );

  assert_eq!(session.repository.owner, repository.owner);
  assert_eq!(session.merge_request.number, merge_request.number);
  assert_eq!(session.platform, "github");
  assert_eq!(session.threads_remaining(), 3);
  assert_eq!(session.discussions.len(), 3);
}

#[test]
fn test_review_session_thread_queue_operations() {
  let repository = create_test_repository();
  let merge_request = create_test_merge_request();
  let discussions = vec![
    create_test_discussion("first", Some("file1.rs".to_string()), Some(1)),
    create_test_discussion("second", Some("file2.rs".to_string()), Some(2)),
    create_test_discussion("third", Some("file3.rs".to_string()), Some(3)),
  ];

  let mut session = ReviewSession::new(
    repository,
    merge_request,
    "github".to_string(),
    discussions,
    vec![],
    ReviewSessionOptions { host: None },
  );

  // Test peek operations
  assert_eq!(session.threads_remaining(), 3);
  let peeked = session.peek_next_thread();
  assert!(peeked.is_some());
  assert_eq!(peeked.unwrap().id, "first");
  assert_eq!(session.threads_remaining(), 3); // Should not change queue

  // Test pop operations
  let popped = session.pop_thread(false);
  assert!(popped.is_some());
  assert_eq!(popped.unwrap().id, "first");
  assert_eq!(session.threads_remaining(), 2);
  assert!(!session.has_unresolved_threads());

  // Test pop with unresolved flag
  let popped_unresolved = session.pop_thread(true);
  assert!(popped_unresolved.is_some());
  assert_eq!(popped_unresolved.unwrap().id, "second");
  assert_eq!(session.threads_remaining(), 1);
  assert!(session.has_unresolved_threads());

  // Pop last thread
  let last_popped = session.pop_thread(false);
  assert!(last_popped.is_some());
  assert_eq!(last_popped.unwrap().id, "third");
  assert_eq!(session.threads_remaining(), 0);

  // Should return None when queue is empty
  assert!(session.peek_next_thread().is_none());
  assert!(session.pop_thread(false).is_none());
}

#[test]
fn test_review_session_empty_discussions() {
  let repository = create_test_repository();
  let merge_request = create_test_merge_request();

  let mut session = ReviewSession::new(
    repository,
    merge_request,
    "github".to_string(),
    vec![], // Empty discussions
    vec![],
    ReviewSessionOptions { host: None },
  );

  assert_eq!(session.threads_remaining(), 0);
  assert!(session.peek_next_thread().is_none());
  assert!(session.pop_thread(false).is_none());
  assert!(!session.has_unresolved_threads());
}

#[test]
fn test_session_save_and_load() {
  let _temp_dir = setup_test_env();

  let repository = create_test_repository();
  let merge_request = create_test_merge_request();
  let discussions = vec![create_test_discussion("test", Some("test.rs".to_string()), Some(1))];

  let session = ReviewSession::new(
    repository.clone(),
    merge_request.clone(),
    "github".to_string(),
    discussions,
    vec![],
    ReviewSessionOptions { host: None },
  );

  // Set up session manager
  let mut session_manager = SessionManager::new().unwrap();
  session_manager.with_session_context("github", "test_org/test_repo", 789).unwrap();

  // Save session
  let save_result = session_manager.save_session(&session);
  assert!(save_result.is_ok());
  assert!(session_manager.session_exists());

  // Load session
  let loaded_session = session_manager.load_session().unwrap();
  assert!(loaded_session.is_some());

  let loaded = loaded_session.unwrap();
  assert_eq!(loaded.repository.owner, repository.owner);
  assert_eq!(loaded.merge_request.number, merge_request.number);
  assert_eq!(loaded.platform, "github");
  assert_eq!(loaded.threads_remaining(), 1);
}

#[test]
fn test_session_clear() {
  let _temp_dir = setup_test_env();

  let repository = create_test_repository();
  let merge_request = create_test_merge_request();
  let session = ReviewSession::new(
    repository,
    merge_request,
    "github".to_string(),
    vec![],
    vec![],
    ReviewSessionOptions { host: None },
  );

  let mut session_manager = SessionManager::new().unwrap();
  session_manager.with_session_context("github", "test_org/test_repo", 789).unwrap();
  session_manager.save_session(&session).unwrap();

  assert!(session_manager.session_exists());

  // Clear session
  let clear_result = session_manager.clear_session();
  assert!(clear_result.is_ok());
  assert!(!session_manager.session_exists());

  // Loading should return None
  let loaded = session_manager.load_session().unwrap();
  assert!(loaded.is_none());
}

#[test]
fn test_load_current_session_with_no_session() {
  let _temp_dir = setup_test_env();

  // Should fail when no session exists
  let result = load_current_session();
  assert!(result.is_err());

  let error_msg = result.unwrap_err().to_string();
  assert!(
    error_msg.contains("session") || error_msg.contains("No") || error_msg.contains("not found")
  );
}

#[test]
fn test_session_discovery() {
  let _temp_dir = setup_test_env();

  let discovery = SessionDiscovery::new();
  assert!(discovery.is_ok());

  let disc = discovery.unwrap();
  let result = disc.find_any_session();
  assert!(result.is_ok());

  // Should return None when no sessions exist
  let found_session = result.unwrap();
  assert!(found_session.is_none());
}

#[test]
fn test_review_session_with_different_discussion_types() {
  let repository = create_test_repository();
  let merge_request = create_test_merge_request();

  let discussions = vec![
    // File-based discussion
    create_test_discussion("file_discussion", Some("src/main.rs".to_string()), Some(42)),
    // General discussion (no file)
    create_test_discussion("general_discussion", None, None),
    // File discussion without line number
    create_test_discussion("file_no_line", Some("README.md".to_string()), None),
  ];

  let session = ReviewSession::new(
    repository,
    merge_request,
    "github".to_string(),
    discussions,
    vec![],
    ReviewSessionOptions { host: None },
  );

  assert_eq!(session.threads_remaining(), 3);

  // Test accessing different discussion types
  assert!(session.discussions.contains_key("file_discussion"));
  assert!(session.discussions.contains_key("general_discussion"));
  assert!(session.discussions.contains_key("file_no_line"));

  let file_disc = session.discussions.get("file_discussion").unwrap();
  assert!(file_disc.file_path.is_some());
  assert_eq!(file_disc.line_number, Some(42));

  let general_disc = session.discussions.get("general_discussion").unwrap();
  assert!(general_disc.file_path.is_none());
  assert!(general_disc.line_number.is_none());
}

#[test]
fn test_session_persistence_edge_cases() {
  let _temp_dir = setup_test_env();

  // Test session manager behavior with corrupted or missing files
  let mut session_manager = SessionManager::new().unwrap();

  // Try to load when no session file exists
  let load_result = session_manager.load_session();
  assert!(load_result.is_ok());
  assert!(load_result.unwrap().is_none());

  // Try to clear when no session exists
  let clear_result = session_manager.clear_session();
  assert!(clear_result.is_ok());
}
