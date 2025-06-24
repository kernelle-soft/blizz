mod mock_github;

use jerrod::platform::{
  github::{GitHubPlatform, GitHubPlatformOptions},
  GitPlatform, ReactionType,
};
use mock_github::MockGitHub;
use std::env;
use tempfile::TempDir;

fn setup_test_env() -> TempDir {
  let temp_dir = TempDir::new().unwrap();
  env::set_var("KERNELLE_DIR", temp_dir.path());
  temp_dir
}

#[tokio::test]
async fn test_github_platform_basic_functionality() {
  let _temp_dir = setup_test_env();
  let mock = MockGitHub::with_test_data();

  // Test get_repository
  let repo_result = mock.get_repository("test_owner", "test_repo").await;
  assert!(repo_result.is_ok());
  let repo = repo_result.unwrap();
  assert_eq!(repo.owner, "test_owner");
  assert_eq!(repo.name, "test_repo");
}

#[tokio::test]
async fn test_github_platform_merge_request_operations() {
  let _temp_dir = setup_test_env();
  let mock = MockGitHub::with_test_data();

  // Test get_merge_request
  let mr_result = mock.get_merge_request("test_owner", "test_repo", 123).await;
  assert!(mr_result.is_ok());
  let mr = mr_result.unwrap();
  assert_eq!(mr.number, 123);
  assert_eq!(mr.title, "Test Pull Request");
}

#[tokio::test]
async fn test_github_platform_discussions() {
  let _temp_dir = setup_test_env();
  let mock = MockGitHub::with_test_data();

  // Test get_discussions
  let discussions_result = mock.get_discussions("test_owner", "test_repo", 123).await;
  assert!(discussions_result.is_ok());
  let discussions = discussions_result.unwrap();
  assert_eq!(discussions.len(), 1);
  assert_eq!(discussions[0].id, "disc123");
}

#[tokio::test]
async fn test_github_platform_comments() {
  let _temp_dir = setup_test_env();
  let mock = MockGitHub::with_test_data();

  // Test add_comment
  let comment_result =
    mock.add_comment("test_owner", "test_repo", "thread_1", "Test comment").await;
  assert!(comment_result.is_ok());
  let note = comment_result.unwrap();
  assert_eq!(note.body, "Test comment");
}

#[tokio::test]
async fn test_github_platform_reactions() {
  let _temp_dir = setup_test_env();
  let mock = MockGitHub::with_test_data();

  // Test add_reaction
  let reaction_result =
    mock.add_reaction("test_owner", "test_repo", "thread_1", ReactionType::ThumbsUp).await;
  assert!(reaction_result.is_ok());
  assert!(reaction_result.unwrap());
}

#[tokio::test]
async fn test_github_platform_resolve_discussion() {
  let _temp_dir = setup_test_env();
  let mock = MockGitHub::with_test_data();

  // Test resolve_discussion
  let resolve_result = mock.resolve_discussion("test_owner", "test_repo", "thread_1").await;
  assert!(resolve_result.is_ok());
  assert!(resolve_result.unwrap());
}

#[tokio::test]
async fn test_github_platform_failure_modes() {
  let _temp_dir = setup_test_env();
  let mut mock = MockGitHub::new();
  mock.should_fail = true;

  // Test that failures are handled gracefully
  let repo_result = mock.get_repository("test_owner", "test_repo").await;
  assert!(repo_result.is_err());

  let mr_result = mock.get_merge_request("test_owner", "test_repo", 123).await;
  assert!(mr_result.is_err());

  let discussions_result = mock.get_discussions("test_owner", "test_repo", 123).await;
  assert!(discussions_result.is_err());
}

#[tokio::test]
async fn test_github_platform_empty_results() {
  let _temp_dir = setup_test_env();
  let mock = MockGitHub::new(); // Empty mock

  // Test with no data
  let repo_result = mock.get_repository("nonexistent", "repo").await;
  assert!(repo_result.is_err()); // Should error when not found

  let discussions_result = mock.get_discussions("test_owner", "test_repo", 999).await;
  assert!(discussions_result.is_ok());
  let discussions = discussions_result.unwrap();
  assert_eq!(discussions.len(), 0); // Should return empty vec
}

#[tokio::test]
async fn test_github_platform_diffs_and_pipelines() {
  let _temp_dir = setup_test_env();
  let mock = MockGitHub::with_test_data();

  // Test get_diffs (returns empty for mock)
  let diffs_result = mock.get_diffs("test_owner", "test_repo", 123).await;
  assert!(diffs_result.is_ok());
  let diffs = diffs_result.unwrap();
  assert_eq!(diffs.len(), 0);

  // Test get_pipelines (returns empty for mock) - sha parameter is string
  let pipelines_result = mock.get_pipelines("test_owner", "test_repo", "abc123").await;
  assert!(pipelines_result.is_ok());
  let pipelines = pipelines_result.unwrap();
  assert_eq!(pipelines.len(), 0);
}

// Skip these tests as they require real GitHub authentication and API calls
#[tokio::test]
#[ignore]
async fn test_real_github_platform_creation() {
  // This would test GitHubPlatform::new() which requires authentication
  // Skip for coverage testing to focus on achievable improvements
  let _result = GitHubPlatform::new(GitHubPlatformOptions::default()).await;
  // Would need real auth credentials to test properly
}

#[tokio::test]
#[ignore]
async fn test_real_github_api_integration() {
  // This would test actual GitHub API calls
  // Skip for coverage testing to focus on testable areas
}
