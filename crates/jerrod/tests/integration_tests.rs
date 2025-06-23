use jerrod::{ReviewSession, SessionManager, platform::*};
use std::fs;
use tempfile::TempDir;

fn create_test_session() -> ReviewSession {
    let repository = Repository {
        owner: "test_owner".to_string(),
        name: "test_repo".to_string(),
        full_name: "test_owner/test_repo".to_string(),
        url: "https://github.com/test_owner/test_repo".to_string(),
    };

    let author = User {
        id: "author123".to_string(),
        username: "testuser".to_string(),
        display_name: "Test User".to_string(),
        avatar_url: None,
    };

    let merge_request = MergeRequest {
        id: "mr123".to_string(),
        number: 123,
        title: "Test MR".to_string(),
        description: Some("Test description".to_string()),
        state: MergeRequestState::Open,
        author,
        assignee: None,
        source_branch: "feature/test".to_string(),
        target_branch: "main".to_string(),
        url: "https://github.com/test_owner/test_repo/pull/123".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let discussions = vec![
        Discussion {
            id: "thread_1".to_string(),
            resolved: false,
            resolvable: true,
            file_path: Some("src/main.rs".to_string()),
            line_number: Some(42),
            notes: vec![],
        },
        Discussion {
            id: "thread_2".to_string(),
            resolved: false,
            resolvable: true,
            file_path: None,
            line_number: None,
            notes: vec![],
        },
    ];

    ReviewSession::new(repository, merge_request, "github".to_string(), discussions, vec![])
}

#[test]
fn test_session_creation() {
    let session = create_test_session();
    
    assert_eq!(session.merge_request.number, 123);
    assert_eq!(session.threads_remaining(), 2);
    assert!(!session.has_unresolved_threads());
}

#[test]
fn test_thread_queue_operations() {
    let mut session = create_test_session();
    
    // Test peek (should not remove from queue)
    let peek_result = session.peek_next_thread();
    assert!(peek_result.is_some());
    assert_eq!(session.threads_remaining(), 2);
    
    // Test pop (should remove from queue)
    let pop_result = session.pop_thread(false);
    assert!(pop_result.is_some());
    assert_eq!(session.threads_remaining(), 1);
    assert!(!session.has_unresolved_threads());
    
    // Test pop with unresolved flag
    let pop_result = session.pop_thread(true);
    assert!(pop_result.is_some());
    assert_eq!(session.threads_remaining(), 0);
    assert!(session.has_unresolved_threads());
}

#[test]
fn test_session_manager() {
    let temp_dir = TempDir::new().unwrap();
    std::env::set_var("KERNELLE_DIR", temp_dir.path());
    
    let mut session_manager = SessionManager::new().unwrap();
    assert!(!session_manager.session_exists());
    
    // Set context and save a session
    session_manager.with_session_context("github", "test/repo", 123).unwrap();
    let session = create_test_session();
    session_manager.save_session(&session).unwrap();
    
    assert!(session_manager.session_exists());
    
    // Load the session back
    let loaded_session = session_manager.load_session().unwrap();
    assert!(loaded_session.is_some());
    let loaded = loaded_session.unwrap();
    assert_eq!(loaded.merge_request.number, 123);
    
    // Clear the session
    session_manager.clear_session().unwrap();
    assert!(!session_manager.session_exists());
}

#[test]
fn test_empty_queue_handling() {
    let session = ReviewSession::new(
        Repository {
            owner: "test".to_string(),
            name: "repo".to_string(),
            full_name: "test/repo".to_string(),
            url: "https://github.com/test/repo".to_string(),
        },
        MergeRequest {
            id: "mr456".to_string(),
            number: 456,
            title: "Empty MR".to_string(),
            description: None,
            state: MergeRequestState::Open,
            author: User {
                id: "user1".to_string(),
                username: "user1".to_string(),
                display_name: "User 1".to_string(),
                avatar_url: None,
            },
            assignee: None,
            source_branch: "feature".to_string(),
            target_branch: "main".to_string(),
            url: "https://github.com/test/repo/pull/456".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        },
        "github".to_string(),
        vec![], // Empty discussions
        vec![]
    );
    
    assert!(session.peek_next_thread().is_none());
    assert_eq!(session.threads_remaining(), 0);
}

#[test]
fn test_platform_types() {
    // Test enum serialization/deserialization
    let state = MergeRequestState::Open;
    let json = serde_json::to_string(&state).unwrap();
    let deserialized: MergeRequestState = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, MergeRequestState::Open));
    
    // Test reaction type methods
    let reaction = ReactionType::ThumbsUp;
    assert_eq!(reaction.emoji(), "👍");
    assert_eq!(reaction.github_name(), "+1");
} 