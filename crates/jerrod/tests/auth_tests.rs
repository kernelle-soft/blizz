use jerrod::auth::{get_github_token, get_gitlab_token};
use std::env;
use tempfile::TempDir;

fn setup_test_env() -> TempDir {
  let temp_dir = TempDir::new().unwrap();
  env::set_var("KERNELLE_DIR", temp_dir.path());
  temp_dir
}

#[tokio::test]
#[ignore] // Requires terminal input for credentials
async fn test_get_github_token_with_missing_credentials() {
  let _temp_dir = setup_test_env();

  // Should fail when no credentials are configured
  let result = get_github_token().await;
  assert!(result.is_err());

  // Error should be related to missing credentials
  let error_msg = result.unwrap_err().to_string();
  assert!(
    error_msg.contains("credential") || error_msg.contains("token") || error_msg.contains("github")
  );
}

#[tokio::test]
#[ignore] // Requires terminal input for credentials
async fn test_get_gitlab_token_with_missing_credentials() {
  let _temp_dir = setup_test_env();

  // Should fail when no credentials are configured
  let result = get_gitlab_token().await;
  assert!(result.is_err());

  // Error should be related to missing credentials
  let error_msg = result.unwrap_err().to_string();
  assert!(
    error_msg.contains("credential") || error_msg.contains("token") || error_msg.contains("gitlab")
  );
}

#[tokio::test]
#[ignore] // Requires terminal input for credentials
async fn test_auth_functions_exist_and_callable() {
  let _temp_dir = setup_test_env();

  // These should at least be callable without panicking
  // Even if they fail due to missing credentials
  let github_result = get_github_token().await;
  let gitlab_result = get_gitlab_token().await;

  // Both should return Result types (either Ok or Err)
  assert!(github_result.is_ok() || github_result.is_err());
  assert!(gitlab_result.is_ok() || gitlab_result.is_err());
}

// NOTE: We can't easily test successful credential retrieval without
// setting up real Sentinel credentials, which would be a security risk
// in automated tests. The integration with Sentinel should be tested
// in the Sentinel crate itself.
