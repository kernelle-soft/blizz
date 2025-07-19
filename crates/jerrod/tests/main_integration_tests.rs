use anyhow::Result;
use assert_cmd::Command;
use jerrod::auth::{register_provider_factory, reset_provider_factory};
use sentinel::CredentialProvider;
use serial_test::serial;
use std::env;
use tempfile::TempDir;

// Mock Sentinel for integration tests
struct MockSentinel;

impl CredentialProvider for MockSentinel {
  fn get_credential(&self, service: &str, key: &str) -> Result<String> {
    // Return fake credentials for tests
    match (service, key) {
      ("github", "token") => Ok("fake_github_token_for_integration_tests".to_string()),
      ("gitlab", "token") => Ok("fake_gitlab_token_for_integration_tests".to_string()),
      _ => Err(anyhow::anyhow!("Credential not found: {}:{}", service, key)),
    }
  }

  fn store_credential(&self, _service: &str, _key: &str, _value: &str) -> Result<()> {
    // Mock store - do nothing
    Ok(())
  }
}

// Helper to get a clean temporary directory for test sessions
fn get_temp_dir() -> TempDir {
  TempDir::new().expect("Failed to create temp directory")
}

// Helper to set up test environment
fn setup_test_env(temp_dir: &TempDir) {
  env::set_var("JERROD_SESSION_DIR", temp_dir.path());
  setup_mock_sentinel();
}

// Helper to set up mock Sentinel for tests
fn setup_mock_sentinel() {
  register_provider_factory(|| Box::new(MockSentinel));
}

// Helper to clean up mock Sentinel after tests
fn cleanup_mock_sentinel() {
  reset_provider_factory();
}

// Helper to add timeouts and test setup to commands
fn setup_test_command() -> Command {
  setup_mock_sentinel();
  let mut cmd = Command::cargo_bin("jerrod").unwrap();
  cmd.timeout(std::time::Duration::from_secs(10));
  cmd
}

#[test]
#[serial]
fn test_cli_help_command() {
  let mut cmd = Command::cargo_bin("jerrod").unwrap();
  let output = cmd.arg("--help").assert().success();

  // Should contain basic command information
  let stdout = String::from_utf8_lossy(&output.get_output().stdout);
  assert!(stdout.contains("jerrod"));
  assert!(stdout.contains("start"));
  assert!(stdout.contains("status"));
}

#[test]
#[serial]
fn test_cli_help_flag() {
  let mut cmd = setup_test_command();
  let output = cmd.arg("--help").assert().success();

  // Should contain basic CLI information
  let stdout = String::from_utf8_lossy(&output.get_output().stdout);
  assert!(stdout.contains("jerrod"));
  assert!(stdout.contains("Usage:"));
}

#[test]
#[serial]
fn test_cli_start_command_parsing() {
  let temp_dir = get_temp_dir();
  setup_test_env(&temp_dir);

  let mut cmd = Command::cargo_bin("jerrod").unwrap();
  cmd.timeout(std::time::Duration::from_secs(10)); // Prevent hanging

  // This should fail because we don't have valid API access
  // but it should parse the command successfully
  let output = cmd.args(["start", "test-project", "1"]).assert().failure();

  // Should show that it attempted to start (not a parsing error)
  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  // The error should be about platform access, not parsing
  assert!(!stderr.contains("error: unrecognized subcommand"));

  cleanup_mock_sentinel();
}

#[test]
#[serial]
fn test_cli_start_command_with_platform() {
  let temp_dir = get_temp_dir();
  setup_test_env(&temp_dir);

  let mut cmd = Command::cargo_bin("jerrod").unwrap();
  cmd.timeout(std::time::Duration::from_secs(10));

  let output = cmd.args(["start", "test-project", "1", "--platform", "github"]).assert().failure();

  // Should show that it attempted to start (not a parsing error)
  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  assert!(!stderr.contains("error: unrecognized subcommand"));

  cleanup_mock_sentinel();
}

#[test]
#[serial]
fn test_cli_status_command() {
  let temp_dir = get_temp_dir();
  setup_test_env(&temp_dir);

  let mut cmd = setup_test_command();
  // Status should fail when there's no session
  let output = cmd.arg("status").assert().failure();

  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  assert!(stderr.contains("No active review session found"));
}

#[test]
#[serial]
fn test_cli_peek_command() {
  let temp_dir = get_temp_dir();
  setup_test_env(&temp_dir);

  let mut cmd = setup_test_command();
  // Peek should fail when there's no session
  let output = cmd.arg("peek").assert().failure();

  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  assert!(stderr.contains("No active review session") || stderr.contains("session"));
}

#[test]
#[serial]
fn test_cli_pop_command() {
  let temp_dir = get_temp_dir();
  setup_test_env(&temp_dir);

  let mut cmd = setup_test_command();
  // Pop should fail when there's no session
  let output = cmd.arg("pop").assert().failure();

  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  assert!(stderr.contains("No active review session") || stderr.contains("session"));
}

#[test]
#[serial]
fn test_cli_pop_command_with_unresolved_flag() {
  let temp_dir = get_temp_dir();
  setup_test_env(&temp_dir);

  let mut cmd = setup_test_command();
  let output = cmd.args(["pop", "--unresolved"]).assert().failure();

  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  assert!(stderr.contains("No active review session") || stderr.contains("session"));
}

#[test]
#[serial]
fn test_cli_acknowledge_command() {
  let temp_dir = get_temp_dir();
  setup_test_env(&temp_dir);

  let mut cmd = setup_test_command();
  // Should fail without session but parse correctly
  let output = cmd.arg("acknowledge").assert().failure();

  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  assert!(!stderr.contains("error: unrecognized subcommand"));
}

#[test]
#[serial]
fn test_cli_acknowledge_command_with_reaction() {
  let temp_dir = get_temp_dir();
  setup_test_env(&temp_dir);

  let mut cmd = setup_test_command();
  // Should fail without session but parse correctly
  let output = cmd.args(["acknowledge", "--reaction", "heart"]).assert().failure();

  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  assert!(!stderr.contains("error: unrecognized subcommand"));
}

#[test]
#[serial]
fn test_cli_comment_command() {
  let temp_dir = get_temp_dir();
  setup_test_env(&temp_dir);

  let mut cmd = setup_test_command();
  // Should fail without session but parse correctly
  let output = cmd.args(["comment", "disc1", "Test comment"]).assert().failure();

  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  assert!(!stderr.contains("error: unrecognized subcommand"));
}

#[test]
#[serial]
fn test_cli_comment_command_new() {
  let temp_dir = get_temp_dir();
  setup_test_env(&temp_dir);

  let mut cmd = setup_test_command();
  // Should fail without session but parse correctly
  let output = cmd.args(["comment", "--new", "Test MR comment"]).assert().failure();

  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  assert!(!stderr.contains("error: unrecognized subcommand"));
}

#[test]
#[serial]
fn test_cli_resolve_command() {
  let temp_dir = get_temp_dir();
  setup_test_env(&temp_dir);

  let mut cmd = setup_test_command();
  // Should fail without session but parse correctly
  let output = cmd.arg("resolve").assert().failure();

  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  assert!(!stderr.contains("error: unrecognized subcommand"));
}



#[test]
#[serial]
fn test_cli_refresh_command() {
  let temp_dir = get_temp_dir();
  setup_test_env(&temp_dir);

  let mut cmd = setup_test_command();
  // Should fail without session but parse correctly
  let output = cmd.arg("refresh").assert().failure();

  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  assert!(!stderr.contains("error: unrecognized subcommand"));
}

#[test]
#[serial]
fn test_cli_finish_command() {
  let temp_dir = get_temp_dir();
  setup_test_env(&temp_dir);

  let mut cmd = setup_test_command();
  // Finish should work gracefully even without a session
  cmd.arg("finish").assert().success();
}

#[test]
#[serial]
fn test_cli_invalid_command() {
  let mut cmd = Command::cargo_bin("jerrod").unwrap();
  let output = cmd.arg("invalid-command").assert().failure();

  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  assert!(stderr.contains("error: unrecognized subcommand"));
}

#[test]
#[serial]
fn test_cli_start_invalid_mr_number() {
  let temp_dir = get_temp_dir();
  setup_test_env(&temp_dir);

  let mut cmd = Command::cargo_bin("jerrod").unwrap();
  let output = cmd.args(["start", "test-project", "not-a-number"]).assert().failure();

  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  // Should be a parsing error for invalid number
  assert!(stderr.contains("error:") || stderr.contains("invalid"));
}

#[test]
#[serial]
fn test_cli_start_missing_arguments() {
  let mut cmd = Command::cargo_bin("jerrod").unwrap();
  let output = cmd.arg("start").assert().failure();

  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  // Should show missing arguments error
  assert!(stderr.contains("error:") || stderr.contains("required"));
}

#[test]
#[serial]
fn test_cli_environment_variables() {
  let temp_dir = get_temp_dir();

  // Test that setting JERROD_SESSION_DIR environment variable works
  let mut cmd = Command::cargo_bin("jerrod").unwrap();
  cmd.env("JERROD_SESSION_DIR", temp_dir.path());
  cmd.env("JERROD_TEST_MODE", "true");
  cmd.timeout(std::time::Duration::from_secs(10));

  // Status should fail when there's no session, even with custom session dir
  let output = cmd.arg("status").assert().failure();

  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  assert!(stderr.contains("No active review session"));
}

#[test]
#[serial]
fn test_cli_start_with_url_format() {
  let temp_dir = get_temp_dir();
  setup_test_env(&temp_dir);

  let mut cmd = Command::cargo_bin("jerrod").unwrap();
  cmd.env("JERROD_TEST_MODE", "true");
  cmd.timeout(std::time::Duration::from_secs(10));

  // This should fail due to authentication but should parse URL correctly
  let output = cmd.args(["start", "owner/repo", "123"]).assert().failure();

  let stderr = String::from_utf8_lossy(&output.get_output().stderr);
  // Should not be a parsing error
  assert!(!stderr.contains("error: unrecognized subcommand"));
}
