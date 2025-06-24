use assert_cmd::Command;
use serial_test::serial;
use std::env;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// Helper to get a clean temporary directory for test sessions
fn get_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

// Helper to set up test environment
fn setup_test_env(temp_dir: &TempDir) {
    env::set_var("JERROD_SESSION_DIR", temp_dir.path());
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
fn test_cli_version_command() {
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
    cmd.arg("--version").assert().success();
}

#[test]
#[serial]
fn test_cli_start_command_parsing() {
    let temp_dir = get_temp_dir();
    setup_test_env(&temp_dir);
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
    // This should fail because we don't have valid credentials/platform
    // but it should parse the command successfully
    let output = cmd.args(&["start", "test-project", "1"]).assert().failure();
    
    // Should show that it attempted to start (not a parsing error)
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    // The error should be about authentication or platform access, not parsing
    assert!(!stderr.contains("error: unrecognized subcommand"));
}

#[test]
#[serial]
fn test_cli_start_command_with_platform() {
    let temp_dir = get_temp_dir();
    setup_test_env(&temp_dir);
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
    let output = cmd.args(&["start", "test-project", "1", "--platform", "github"]).assert().failure();
    
    // Should show that it attempted to start (not a parsing error)
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    assert!(!stderr.contains("error: unrecognized subcommand"));
}

#[test]
#[serial]
fn test_cli_status_command() {
    let temp_dir = get_temp_dir();
    setup_test_env(&temp_dir);
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
    // Status should work even without a session
    cmd.arg("status").assert().success();
}

#[test]
#[serial]
fn test_cli_peek_command() {
    let temp_dir = get_temp_dir();
    setup_test_env(&temp_dir);
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
    // Peek should work even without a session
    cmd.arg("peek").assert().success();
}

#[test]
#[serial]
fn test_cli_pop_command() {
    let temp_dir = get_temp_dir();
    setup_test_env(&temp_dir);
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
    // Pop should work even without a session
    cmd.arg("pop").assert().success();
}

#[test]
#[serial]
fn test_cli_pop_command_with_unresolved_flag() {
    let temp_dir = get_temp_dir();
    setup_test_env(&temp_dir);
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
    cmd.args(&["pop", "--unresolved"]).assert().success();
}

#[test]
#[serial]
fn test_cli_acknowledge_command() {
    let temp_dir = get_temp_dir();
    setup_test_env(&temp_dir);
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
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
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
    // Should fail without session but parse correctly
    let output = cmd.args(&["acknowledge", "--reaction", "heart"]).assert().failure();
    
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    assert!(!stderr.contains("error: unrecognized subcommand"));
}

#[test]
#[serial]
fn test_cli_comment_command() {
    let temp_dir = get_temp_dir();
    setup_test_env(&temp_dir);
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
    // Should fail without session but parse correctly
    let output = cmd.args(&["comment", "disc1", "Test comment"]).assert().failure();
    
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    assert!(!stderr.contains("error: unrecognized subcommand"));
}

#[test]
#[serial]
fn test_cli_comment_command_new() {
    let temp_dir = get_temp_dir();
    setup_test_env(&temp_dir);
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
    // Should fail without session but parse correctly
    let output = cmd.args(&["comment", "--new", "Test MR comment"]).assert().failure();
    
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    assert!(!stderr.contains("error: unrecognized subcommand"));
}

#[test]
#[serial]
fn test_cli_resolve_command() {
    let temp_dir = get_temp_dir();
    setup_test_env(&temp_dir);
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
    // Should fail without session but parse correctly
    let output = cmd.arg("resolve").assert().failure();
    
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    assert!(!stderr.contains("error: unrecognized subcommand"));
}

#[test]
#[serial]
fn test_cli_commit_command() {
    let temp_dir = get_temp_dir();
    setup_test_env(&temp_dir);
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
    // Should fail without session but parse correctly
    let output = cmd.args(&["commit", "Test commit message"]).assert().failure();
    
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    assert!(!stderr.contains("error: unrecognized subcommand"));
}

#[test]
#[serial]
fn test_cli_commit_command_with_details() {
    let temp_dir = get_temp_dir();
    setup_test_env(&temp_dir);
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
    // Should fail without session but parse correctly
    let output = cmd.args(&["commit", "Test commit", "--details", "More details"]).assert().failure();
    
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    assert!(!stderr.contains("error: unrecognized subcommand"));
}

#[test]
#[serial]
fn test_cli_refresh_command() {
    let temp_dir = get_temp_dir();
    setup_test_env(&temp_dir);
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
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
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
    // Finish should work even without a session
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
    let output = cmd.args(&["start", "test-project", "not-a-number"]).assert().failure();
    
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
    cmd.arg("status").assert().success();
}

#[test]
#[serial]
fn test_cli_start_with_url_format() {
    let temp_dir = get_temp_dir();
    setup_test_env(&temp_dir);
    
    let mut cmd = Command::cargo_bin("jerrod").unwrap();
    // This should fail due to authentication but should parse URL correctly
    let output = cmd.args(&["start", "owner/repo", "123"]).assert().failure();
    
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    // Should not be a parsing error
    assert!(!stderr.contains("error: unrecognized subcommand"));
} 