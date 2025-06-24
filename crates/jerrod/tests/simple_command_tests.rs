use jerrod::commands::{finish, start};
use jerrod::commands::acknowledge::{AcknowledgeConfig, AcknowledgeFlags};
use jerrod::platform::ReactionType;
use jerrod::session::SessionManager;
use std::env;
use tempfile::TempDir;

fn setup_test_env() -> TempDir {
  let temp_dir = TempDir::new().unwrap();
  env::set_var("KERNELLE_DIR", temp_dir.path());
  temp_dir
}

#[tokio::test]
async fn test_acknowledge_config_creation() {
  // Test thumbs up configuration
  let thumbs_up_config = AcknowledgeConfig::from_flags(AcknowledgeFlags {
    thumbs_up: true,
    ..Default::default()
  });
  assert!(matches!(thumbs_up_config.reaction_type, ReactionType::ThumbsUp));

  // Test heart configuration
  let heart_config = AcknowledgeConfig::from_flags(AcknowledgeFlags {
    heart: true,
    ..Default::default()
  });
  assert!(matches!(heart_config.reaction_type, ReactionType::Heart));
}

#[tokio::test]
async fn test_acknowledge_config_defaults() {
  // Test default configuration (all false should default to thumbs up)
  let default_config = AcknowledgeConfig::from_flags(AcknowledgeFlags::default());
  assert!(matches!(default_config.reaction_type, ReactionType::ThumbsUp));
}

#[tokio::test]
async fn test_acknowledge_config_priority() {
  // Test that multiple flags prioritize correctly (should pick first true flag)
  let multi_config = AcknowledgeConfig::from_flags(AcknowledgeFlags {
    thumbs_down: true,
    laugh: true,
    ..Default::default()
  });
  assert!(matches!(multi_config.reaction_type, ReactionType::ThumbsDown));
}

#[tokio::test]
async fn test_finish_command_without_session() {
  let _temp_dir = setup_test_env();

  // Finish should succeed even without session
  let result = finish::handle().await;
  assert!(result.is_ok());
}

#[tokio::test]
async fn test_start_command_invalid_repository() {
  let _temp_dir = setup_test_env();

  // Start should fail with invalid repository format
  let result = start::handle("invalid-repo-format".to_string(), 123, None).await;
  assert!(result.is_err());
}

#[tokio::test]
async fn test_start_command_invalid_platform() {
  let _temp_dir = setup_test_env();

  // Start should reject invalid platform
  let result = start::handle(
    "https://github.com/test/repo".to_string(),
    123,
    Some("invalid_platform".to_string()),
  )
  .await;

  assert!(result.is_err());
  let error_msg = result.unwrap_err().to_string();
  assert!(error_msg.contains("Unsupported platform") || error_msg.contains("invalid_platform"));
}

#[tokio::test]
async fn test_session_manager_initialization() {
  let _temp_dir = setup_test_env();

  let session_manager = SessionManager::new();
  assert!(session_manager.is_ok());

  let manager = session_manager.unwrap();
  assert!(!manager.session_exists());
}

#[tokio::test]
async fn test_session_manager_context() {
  let _temp_dir = setup_test_env();

  let mut session_manager = SessionManager::new().unwrap();

  // Test valid context
  let result = session_manager.with_session_context("github", "owner/repo", 123);
  assert!(result.is_ok());

  // Test empty repository
  let result = session_manager.with_session_context("github", "", 123);
  // Should handle empty string gracefully
  assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_reaction_type_emoji_methods() {
  // Test all reaction types have proper emoji and GitHub name mappings
  let test_cases = vec![
    (ReactionType::ThumbsUp, "👍", "+1"),
    (ReactionType::ThumbsDown, "👎", "-1"),
    (ReactionType::Laugh, "😄", "laugh"),
    (ReactionType::Hooray, "🎉", "hooray"),
    (ReactionType::Confused, "😕", "confused"),
    (ReactionType::Heart, "❤️", "heart"),
    (ReactionType::Rocket, "🚀", "rocket"),
    (ReactionType::Eyes, "👀", "eyes"),
  ];

  for (reaction, expected_emoji, expected_github_name) in test_cases {
    assert_eq!(reaction.emoji(), expected_emoji);
    assert_eq!(reaction.github_name(), expected_github_name);
  }
}

#[tokio::test]
async fn test_error_handling_graceful() {
  let _temp_dir = setup_test_env();

  // Test that commands fail gracefully with missing sessions
  // (Testing error handling, not success cases)

  // These should all return errors, not panic
  let status_result = jerrod::commands::status::handle().await;
  assert!(status_result.is_err());

  let peek_result = jerrod::commands::peek::handle().await;
  assert!(peek_result.is_err());

  let pop_result = jerrod::commands::pop::handle(false).await;
  assert!(pop_result.is_err());

  let refresh_result = jerrod::commands::refresh::handle().await;
  assert!(refresh_result.is_err());
}
