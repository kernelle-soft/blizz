//! Daemon logging infrastructure for bentley
//!
//! This module provides persistent, structured logging for daemons with:
//! - JSONL disk storage with unlimited capacity
//! - Thread-safe async operations with internal locking
//! - Optional console output (silent mode support)
//! - Full bentley macro integration for unified logging

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "schemars")]
use schemars::JsonSchema;

// Types and Data Structures
// =========================

/// Request context information for logs
#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct LogContext {
  /// Request ID for correlation
  #[serde(skip_serializing_if = "Option::is_none")]
  pub request_id: Option<String>,

  /// HTTP method
  #[serde(skip_serializing_if = "Option::is_none")]
  pub method: Option<String>,

  /// Request path
  #[serde(skip_serializing_if = "Option::is_none")]
  pub path: Option<String>,

  /// User agent
  #[serde(skip_serializing_if = "Option::is_none")]
  pub user_agent: Option<String>,

  /// Request duration in milliseconds
  #[serde(skip_serializing_if = "Option::is_none")]
  pub duration_ms: Option<f64>,

  /// HTTP status code
  #[serde(skip_serializing_if = "Option::is_none")]
  pub status_code: Option<u16>,
}

/// A structured log entry for daemon operations
#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub struct LogEntry {
  pub timestamp: DateTime<Utc>,
  pub level: String,
  pub message: String,
  pub component: String,

  /// Optional request context
  #[serde(skip_serializing_if = "Option::is_none")]
  pub context: Option<LogContext>,
}

/// Request structure for querying daemon logs
#[derive(Debug, Serialize, Deserialize)]
pub struct LogsRequest {
  pub request: String,
  pub limit: Option<usize>,
  pub level: Option<String>,
}

/// Response structure for daemon log queries
#[derive(Debug, Serialize, Deserialize)]
pub struct LogsResponse {
  pub success: bool,
  pub logs: Vec<LogEntry>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub error: Option<ErrorInfo>,
}

/// Error information for daemon responses
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorInfo {
  pub message: String,
  pub tag: String,
}

/// Internal log storage implementation
struct DaemonLogsInner {
  log_file_path: std::path::PathBuf,
  silent: bool,
}

/// Thread-safe disk-based log storage for daemons using JSONL format
#[derive(Clone)]
pub struct DaemonLogs {
  inner: std::sync::Arc<tokio::sync::Mutex<DaemonLogsInner>>,
}

// Constructor Functions
// =====================

#[cfg(not(tarpaulin_include))]
impl LogsResponse {
  pub fn success(logs: Vec<LogEntry>) -> Self {
    Self { success: true, logs, error: None }
  }

  pub fn error(message: &str, tag: &str) -> Self {
    Self {
      success: false,
      logs: Vec::new(),
      error: Some(ErrorInfo { message: message.to_string(), tag: tag.to_string() }),
    }
  }
}

impl DaemonLogsInner {
  /// Create a new daemon log storage that writes to the specified file path
  fn new<P: AsRef<std::path::Path>>(log_file_path: P, silent: bool) -> std::io::Result<Self> {
    let log_file_path = log_file_path.as_ref().to_path_buf();

    // Ensure parent directory exists
    if let Some(parent) = log_file_path.parent() {
      std::fs::create_dir_all(parent)?;
    }

    // Create file if it doesn't exist (but don't truncate if it does)
    if !log_file_path.exists() {
      std::fs::File::create(&log_file_path)?;
    }

    Ok(Self { log_file_path, silent })
  }

  /// Add a log entry to storage (appends to JSONL file)
  fn add_log(&mut self, level: &str, message: &str, component: &str) -> std::io::Result<()> {
    self.add_log_with_context(level, message, component, None)
  }

  /// Add a log entry with context to storage (appends to JSONL file)
  fn add_log_with_context(
    &mut self,
    level: &str,
    message: &str,
    component: &str,
    context: Option<LogContext>,
  ) -> std::io::Result<()> {
    let entry = LogEntry {
      timestamp: Utc::now(),
      level: level.to_string(),
      message: message.to_string(),
      component: component.to_string(),
      context,
    };

    // Serialize to JSON and append to file
    let json_line = serde_json::to_string(&entry)
      .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    use std::fs::OpenOptions;
    use std::io::Write;

    let mut file = OpenOptions::new().create(true).append(true).open(&self.log_file_path)?;

    writeln!(file, "{json_line}")?;
    file.flush()?;

    Ok(())
  }
}

// Log Operations
// ==============

impl DaemonLogsInner {
  /// Retrieve logs with optional filtering and limiting (reads from JSONL file)
  #[cfg(not(tarpaulin_include))]
  fn get_logs(
    &self,
    limit: Option<usize>,
    level_filter: Option<&str>,
  ) -> std::io::Result<Vec<LogEntry>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    if !self.log_file_path.exists() {
      return Ok(Vec::new());
    }

    let file = File::open(&self.log_file_path)?;
    let reader = BufReader::new(file);

    let mut logs = Vec::new();

    for line_result in reader.lines() {
      let line = line_result?;
      if line.trim().is_empty() {
        continue;
      }

      match serde_json::from_str::<LogEntry>(&line) {
        Ok(entry) => {
          // Apply level filter
          let matches_level =
            level_filter.is_none_or(|filter| filter == "all" || entry.level == filter);

          if matches_level {
            logs.push(entry);
          }
        }
        Err(_) => {
          // Skip malformed lines
          continue;
        }
      }
    }

    // Sort by timestamp (newest first) to get most recent entries first
    logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Apply limit to get the most recent N entries
    if let Some(limit) = limit {
      logs.truncate(limit);
    }

    // Reverse to show oldest first, newest last (for terminal-friendly display)
    logs.reverse();

    Ok(logs)
  }

  /// Get the path to the log file
  #[cfg(not(tarpaulin_include))]
  fn log_file_path(&self) -> &std::path::Path {
    &self.log_file_path
  }

  /// Check if the log file exists and has content
  #[cfg(not(tarpaulin_include))]
  fn has_logs(&self) -> bool {
    self.log_file_path.exists()
      && std::fs::metadata(&self.log_file_path).map(|m| m.len() > 0).unwrap_or(false)
  }

  /// Get the size of the log file in bytes
  #[cfg(not(tarpaulin_include))]
  fn file_size(&self) -> std::io::Result<u64> {
    let metadata = std::fs::metadata(&self.log_file_path)?;
    Ok(metadata.len())
  }
}

// Core API
// ========

#[cfg(not(tarpaulin_include))]
impl DaemonLogs {
  /// Create a new thread-safe daemon log storage
  pub fn new<P: AsRef<std::path::Path>>(log_file_path: P) -> std::io::Result<Self> {
    Self::new_with_silent(log_file_path, false)
  }

  /// Create a new thread-safe daemon log storage with silent option
  pub fn new_with_silent<P: AsRef<std::path::Path>>(
    log_file_path: P,
    silent: bool,
  ) -> std::io::Result<Self> {
    let inner = DaemonLogsInner::new(log_file_path, silent)?;
    Ok(Self { inner: std::sync::Arc::new(tokio::sync::Mutex::new(inner)) })
  }

  /// Add a log entry (handles locking internally)
  pub async fn add_log(&self, level: &str, message: &str, component: &str) -> std::io::Result<()> {
    let mut guard = self.inner.lock().await;
    guard.add_log(level, message, component)
  }

  /// Add a log entry with context (handles locking internally)
  pub async fn add_log_with_context(
    &self,
    level: &str,
    message: &str,
    component: &str,
    context: Option<LogContext>,
  ) -> std::io::Result<()> {
    let mut guard = self.inner.lock().await;
    guard.add_log_with_context(level, message, component, context)
  }

  /// Add a log entry (fire-and-forget, ignores errors)
  pub async fn log(&self, level: &str, message: &str, component: &str) {
    let _ = self.add_log(level, message, component).await;
  }

  /// Add a log entry with context (fire-and-forget, ignores errors)
  pub async fn log_with_context(
    &self,
    level: &str,
    message: &str,
    component: &str,
    context: LogContext,
  ) {
    let _ = self.add_log_with_context(level, message, component, Some(context)).await;
  }

  /// Retrieve logs with optional filtering and limiting (handles locking internally)
  pub async fn get_logs(
    &self,
    limit: Option<usize>,
    level_filter: Option<&str>,
  ) -> std::io::Result<Vec<LogEntry>> {
    let guard = self.inner.lock().await;
    guard.get_logs(limit, level_filter)
  }

  /// Get the path to the log file
  pub async fn log_file_path(&self) -> std::path::PathBuf {
    let guard = self.inner.lock().await;
    guard.log_file_path().to_path_buf()
  }

  /// Check if the log file exists and has content
  pub async fn has_logs(&self) -> bool {
    let guard = self.inner.lock().await;
    guard.has_logs()
  }

  /// Get the size of the log file in bytes
  pub async fn file_size(&self) -> std::io::Result<u64> {
    let guard = self.inner.lock().await;
    guard.file_size()
  }
}

// Standard Logging Wrappers
// =========================

#[cfg(not(tarpaulin_include))]
impl DaemonLogs {
  /// Log an info message (to disk + console unless silent)
  pub async fn info(&self, message: &str, component: &str) {
    self.log("info", message, component).await;

    let guard = self.inner.lock().await;
    if !guard.silent {
      crate::info!(message);
    }
  }

  /// Log an info message with context (to disk + console unless silent)
  pub async fn info_with_context(&self, message: &str, component: &str, context: LogContext) {
    self.log_with_context("info", message, component, context).await;

    let guard = self.inner.lock().await;
    if !guard.silent {
      crate::info!(message);
    }
  }

  /// Log a warning message (to disk + console unless silent)
  pub async fn warn(&self, message: &str, component: &str) {
    self.log("warn", message, component).await;

    let guard = self.inner.lock().await;
    if !guard.silent {
      crate::warn!(message);
    }
  }

  /// Log a warning message with context (to disk + console unless silent)
  pub async fn warn_with_context(&self, message: &str, component: &str, context: LogContext) {
    self.log_with_context("warn", message, component, context).await;

    let guard = self.inner.lock().await;
    if !guard.silent {
      crate::warn!(message);
    }
  }

  /// Log a verbose message (to disk + console unless silent)
  pub async fn verbose(&self, message: &str, component: &str) {
    self.log("verbose", message, component).await;

    let guard = self.inner.lock().await;
    if !guard.silent {
      crate::verbose!(message);
    }
  }

  /// Log an error message (to disk + console unless silent)
  pub async fn error(&self, message: &str, component: &str) {
    self.log("error", message, component).await;

    let guard = self.inner.lock().await;
    if !guard.silent {
      crate::error!(message);
    }
  }

  /// Log an error message with context (to disk + console unless silent)
  pub async fn error_with_context(&self, message: &str, component: &str, context: LogContext) {
    self.log_with_context("error", message, component, context).await;

    let guard = self.inner.lock().await;
    if !guard.silent {
      crate::error!(message);
    }
  }

  /// Log a debug message (to disk + console unless silent)
  pub async fn debug(&self, message: &str, component: &str) {
    self.log("debug", message, component).await;

    let guard = self.inner.lock().await;
    if !guard.silent {
      crate::debug!(message);
    }
  }

  /// Log a success message (to disk + console unless silent)
  pub async fn success(&self, message: &str, component: &str) {
    self.log("success", message, component).await;

    let guard = self.inner.lock().await;
    if !guard.silent {
      crate::success!(message);
    }
  }

  /// Log a success message with context (to disk + console unless silent)
  pub async fn success_with_context(&self, message: &str, component: &str, context: LogContext) {
    self.log_with_context("success", message, component, context).await;

    let guard = self.inner.lock().await;
    if !guard.silent {
      crate::success!(message);
    }
  }
}

// Theatrical Logging Wrappers
// ============================

#[cfg(not(tarpaulin_include))]
impl DaemonLogs {
  /// Log an announcement message (to disk + console unless silent)
  pub async fn announce(&self, message: &str, component: &str) {
    self.log("announce", message, component).await;

    let guard = self.inner.lock().await;
    if !guard.silent {
      crate::announce!(message);
    }
  }

  /// Log a spotlight message (to disk + console unless silent)
  pub async fn spotlight(&self, message: &str, component: &str) {
    self.log("spotlight", message, component).await;

    let guard = self.inner.lock().await;
    if !guard.silent {
      crate::spotlight!(message);
    }
  }

  /// Log a flourish message (to disk + console unless silent)
  pub async fn flourish(&self, message: &str, component: &str) {
    self.log("flourish", message, component).await;

    let guard = self.inner.lock().await;
    if !guard.silent {
      crate::flourish!(message);
    }
  }

  /// Log a showstopper message (to disk + console unless silent)
  pub async fn showstopper(&self, message: &str, component: &str) {
    self.log("showstopper", message, component).await;

    let guard = self.inner.lock().await;
    if !guard.silent {
      crate::showstopper!(message);
    }
  }
}

// Tests
// =====

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;
  use tempfile::TempDir;

  /// Helper function to create a temporary log file path
  fn temp_log_path() -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let log_path = temp_dir.path().join("test.log");
    (temp_dir, log_path)
  }

  // Constructor Tests
  // =================

  #[tokio::test]
  async fn test_daemon_logs_new_creates_file() {
    let (_temp_dir, log_path) = temp_log_path();

    let logs = DaemonLogs::new(&log_path).unwrap();

    // File should exist after creation
    assert!(log_path.exists());

    // Should not be silent by default
    let path_result = logs.log_file_path().await;
    assert_eq!(path_result, log_path);
  }

  #[tokio::test]
  async fn test_daemon_logs_new_with_silent() {
    let (_temp_dir, log_path) = temp_log_path();

    let logs = DaemonLogs::new_with_silent(&log_path, true).unwrap();

    // File should exist
    assert!(log_path.exists());

    // Should be accessible
    let path_result = logs.log_file_path().await;
    assert_eq!(path_result, log_path);
  }

  #[tokio::test]
  async fn test_daemon_logs_creates_parent_directories() {
    let temp_dir = TempDir::new().unwrap();
    let nested_path = temp_dir.path().join("nested").join("deep").join("test.log");

    let _logs = DaemonLogs::new(&nested_path).unwrap();

    // Parent directories should be created
    assert!(nested_path.parent().unwrap().exists());
    assert!(nested_path.exists());
  }

  // Basic Logging Tests
  // ===================

  #[tokio::test]
  async fn test_add_log_writes_to_file() {
    let (_temp_dir, log_path) = temp_log_path();
    let logs = DaemonLogs::new_with_silent(&log_path, true).unwrap();

    // Add a log entry
    logs.add_log("info", "Test message", "test_component").await.unwrap();

    // File should contain the log entry
    let content = fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("Test message"));
    assert!(content.contains("info"));
    assert!(content.contains("test_component"));

    // Should be valid JSON
    let lines: Vec<&str> = content.trim().split('\n').collect();
    assert_eq!(lines.len(), 1);

    let entry: LogEntry = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(entry.message, "Test message");
    assert_eq!(entry.level, "info");
    assert_eq!(entry.component, "test_component");
  }

  #[tokio::test]
  async fn test_log_fire_and_forget() {
    let (_temp_dir, log_path) = temp_log_path();
    let logs = DaemonLogs::new_with_silent(&log_path, true).unwrap();

    // Should not panic even if there were hypothetical errors
    logs.log("debug", "Fire and forget message", "test").await;

    // Should still write to file
    let content = fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("Fire and forget message"));
  }

  #[tokio::test]
  async fn test_multiple_log_entries_append() {
    let (_temp_dir, log_path) = temp_log_path();
    let logs = DaemonLogs::new_with_silent(&log_path, true).unwrap();

    // Add multiple entries
    logs.add_log("info", "First message", "comp1").await.unwrap();
    logs.add_log("warn", "Second message", "comp2").await.unwrap();
    logs.add_log("error", "Third message", "comp3").await.unwrap();

    // File should contain all entries
    let content = fs::read_to_string(&log_path).unwrap();
    let lines: Vec<&str> = content.trim().split('\n').collect();
    assert_eq!(lines.len(), 3);

    // Parse and verify each entry
    let entry1: LogEntry = serde_json::from_str(lines[0]).unwrap();
    let entry2: LogEntry = serde_json::from_str(lines[1]).unwrap();
    let entry3: LogEntry = serde_json::from_str(lines[2]).unwrap();

    assert_eq!(entry1.message, "First message");
    assert_eq!(entry2.message, "Second message");
    assert_eq!(entry3.message, "Third message");
  }

  // Log Retrieval Tests
  // ===================

  #[tokio::test]
  async fn test_get_logs_empty_file() {
    let (_temp_dir, log_path) = temp_log_path();
    let logs = DaemonLogs::new_with_silent(&log_path, true).unwrap();

    // Should return empty vec for empty file
    let result = logs.get_logs(None, None).await.unwrap();
    assert_eq!(result.len(), 0);
  }

  #[tokio::test]
  async fn test_get_logs_no_filter() {
    let (_temp_dir, log_path) = temp_log_path();
    let logs = DaemonLogs::new_with_silent(&log_path, true).unwrap();

    // Add some logs
    logs.add_log("info", "Message 1", "comp1").await.unwrap();
    logs.add_log("warn", "Message 2", "comp2").await.unwrap();
    logs.add_log("error", "Message 3", "comp3").await.unwrap();

    // Get all logs
    let result = logs.get_logs(None, None).await.unwrap();
    assert_eq!(result.len(), 3);

    // Should be sorted by timestamp (oldest first, newest last) - since we added them quickly,
    // let's just check they're all there
    let messages: Vec<_> = result.iter().map(|e| e.message.as_str()).collect();
    assert!(messages.contains(&"Message 1"));
    assert!(messages.contains(&"Message 2"));
    assert!(messages.contains(&"Message 3"));
  }

  #[tokio::test]
  async fn test_get_logs_with_level_filter() {
    let (_temp_dir, log_path) = temp_log_path();
    let logs = DaemonLogs::new_with_silent(&log_path, true).unwrap();

    // Add logs with different levels
    logs.add_log("info", "Info message", "comp1").await.unwrap();
    logs.add_log("warn", "Warn message", "comp2").await.unwrap();
    logs.add_log("error", "Error message", "comp3").await.unwrap();
    logs.add_log("info", "Another info", "comp4").await.unwrap();

    // Filter by "info" level
    let info_logs = logs.get_logs(None, Some("info")).await.unwrap();
    assert_eq!(info_logs.len(), 2);
    for entry in &info_logs {
      assert_eq!(entry.level, "info");
    }

    // Filter by "warn" level
    let warn_logs = logs.get_logs(None, Some("warn")).await.unwrap();
    assert_eq!(warn_logs.len(), 1);
    assert_eq!(warn_logs[0].level, "warn");
    assert_eq!(warn_logs[0].message, "Warn message");

    // Filter by "all" should return everything
    let all_logs = logs.get_logs(None, Some("all")).await.unwrap();
    assert_eq!(all_logs.len(), 4);
  }

  #[tokio::test]
  async fn test_get_logs_with_limit() {
    let (_temp_dir, log_path) = temp_log_path();
    let logs = DaemonLogs::new_with_silent(&log_path, true).unwrap();

    // Add 5 logs
    for i in 1..=5 {
      logs.add_log("info", &format!("Message {i}"), "comp").await.unwrap();
    }

    // Test limit of 3
    let limited = logs.get_logs(Some(3), None).await.unwrap();
    assert_eq!(limited.len(), 3);

    // Test limit of 0
    let empty = logs.get_logs(Some(0), None).await.unwrap();
    assert_eq!(empty.len(), 0);

    // Test limit higher than available
    let all = logs.get_logs(Some(10), None).await.unwrap();
    assert_eq!(all.len(), 5);
  }

  #[tokio::test]
  async fn test_get_logs_with_level_filter_and_limit() {
    let (_temp_dir, log_path) = temp_log_path();
    let logs = DaemonLogs::new_with_silent(&log_path, true).unwrap();

    // Add mixed logs
    logs.add_log("info", "Info 1", "comp").await.unwrap();
    logs.add_log("error", "Error 1", "comp").await.unwrap();
    logs.add_log("info", "Info 2", "comp").await.unwrap();
    logs.add_log("error", "Error 2", "comp").await.unwrap();
    logs.add_log("info", "Info 3", "comp").await.unwrap();

    // Get only 2 info messages
    let result = logs.get_logs(Some(2), Some("info")).await.unwrap();
    assert_eq!(result.len(), 2);
    for entry in &result {
      assert_eq!(entry.level, "info");
    }
  }

  #[tokio::test]
  async fn test_get_logs_handles_malformed_json() {
    let (_temp_dir, log_path) = temp_log_path();

    // Write some valid and invalid JSON lines
    fs::write(
      &log_path,
      r#"{"timestamp":"2024-01-01T12:00:00Z","level":"info","message":"Valid","component":"test"}
invalid json line
{"timestamp":"2024-01-01T12:01:00Z","level":"warn","message":"Also valid","component":"test"}
another bad line
"#,
    )
    .unwrap();

    let logs = DaemonLogs::new_with_silent(&log_path, true).unwrap();

    // Should only return the valid entries
    let result = logs.get_logs(None, None).await.unwrap();
    assert_eq!(result.len(), 2);

    let messages: Vec<_> = result.iter().map(|e| e.message.as_str()).collect();
    assert!(messages.contains(&"Valid"));
    assert!(messages.contains(&"Also valid"));
  }

  // File Operations Tests
  // =====================

  #[tokio::test]
  async fn test_has_logs() {
    let (_temp_dir, log_path) = temp_log_path();
    let logs = DaemonLogs::new_with_silent(&log_path, true).unwrap();

    // Should be false for empty file
    assert!(!logs.has_logs().await);

    // Add a log entry
    logs.add_log("info", "Test", "comp").await.unwrap();

    // Should be true after adding content
    assert!(logs.has_logs().await);
  }

  #[tokio::test]
  async fn test_file_size() {
    let (_temp_dir, log_path) = temp_log_path();
    let logs = DaemonLogs::new_with_silent(&log_path, true).unwrap();

    // Should be 0 for empty file
    let initial_size = logs.file_size().await.unwrap();
    assert_eq!(initial_size, 0);

    // Add a log entry
    logs.add_log("info", "Test message", "comp").await.unwrap();

    // Size should increase
    let new_size = logs.file_size().await.unwrap();
    assert!(new_size > initial_size);
  }

  #[tokio::test]
  async fn test_log_file_path() {
    let (_temp_dir, log_path) = temp_log_path();
    let logs = DaemonLogs::new_with_silent(&log_path, true).unwrap();

    let returned_path = logs.log_file_path().await;
    assert_eq!(returned_path, log_path);
  }

  // Concurrency Tests
  // =================

  #[tokio::test]
  async fn test_concurrent_writes() {
    let (_temp_dir, log_path) = temp_log_path();
    let logs = DaemonLogs::new_with_silent(&log_path, true).unwrap();

    // Spawn multiple concurrent write tasks
    let mut handles = vec![];
    for i in 0..10 {
      let logs_clone = logs.clone();
      let handle = tokio::spawn(async move {
        logs_clone.add_log("info", &format!("Message {i}"), "concurrent").await.unwrap();
      });
      handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
      handle.await.unwrap();
    }

    // Should have all 10 entries
    let result = logs.get_logs(None, None).await.unwrap();
    assert_eq!(result.len(), 10);

    // All should be from concurrent component
    for entry in &result {
      assert_eq!(entry.component, "concurrent");
    }
  }

  #[tokio::test]
  async fn test_clone_shares_same_log_file() {
    let (_temp_dir, log_path) = temp_log_path();
    let logs1 = DaemonLogs::new_with_silent(&log_path, true).unwrap();
    let logs2 = logs1.clone();

    // Write to both instances
    logs1.add_log("info", "From logs1", "comp1").await.unwrap();
    logs2.add_log("warn", "From logs2", "comp2").await.unwrap();

    // Both should see both entries
    let result1 = logs1.get_logs(None, None).await.unwrap();
    let result2 = logs2.get_logs(None, None).await.unwrap();

    assert_eq!(result1.len(), 2);
    assert_eq!(result2.len(), 2);

    // Both should have the same content (since they share the same file)
    let messages1: std::collections::HashSet<_> = result1.iter().map(|e| &e.message).collect();
    let messages2: std::collections::HashSet<_> = result2.iter().map(|e| &e.message).collect();
    assert_eq!(messages1, messages2);
  }

  // Wrapper Method Tests
  // ====================

  // Note: Testing the actual console output is difficult in unit tests,
  // but we can at least verify that the methods work and log to disk

  #[tokio::test]
  async fn test_wrapper_methods_log_to_disk() {
    let (_temp_dir, log_path) = temp_log_path();
    let logs = DaemonLogs::new_with_silent(&log_path, true).unwrap(); // Silent mode

    // Test each wrapper method
    logs.info("Info test", "comp").await;
    logs.warn("Warn test", "comp").await;
    logs.error("Error test", "comp").await;
    logs.debug("Debug test", "comp").await;
    logs.success("Success test", "comp").await;
    logs.announce("Announce test", "comp").await;
    logs.spotlight("Spotlight test", "comp").await;
    logs.flourish("Flourish test", "comp").await;
    logs.showstopper("Showstopper test", "comp").await;

    // All should be logged to disk
    let result = logs.get_logs(None, None).await.unwrap();
    assert_eq!(result.len(), 9);

    // Check that we have all the expected levels
    let levels: std::collections::HashSet<_> = result.iter().map(|e| e.level.as_str()).collect();
    let expected_levels = vec![
      "info",
      "warn",
      "error",
      "debug",
      "success",
      "announce",
      "spotlight",
      "flourish",
      "showstopper",
    ];
    for expected in expected_levels {
      assert!(levels.contains(expected), "Missing level: {expected}");
    }
  }

  // Error Handling Tests
  // ====================

  #[tokio::test]
  async fn test_get_logs_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("does_not_exist.log");

    // Create logs instance but don't create the file
    let logs = DaemonLogs::new_with_silent(&nonexistent, true).unwrap();

    // Delete the file that was auto-created
    fs::remove_file(&nonexistent).unwrap();

    // Should return empty vec, not error
    let result = logs.get_logs(None, None).await.unwrap();
    assert_eq!(result.len(), 0);
  }

  // Response Constructor Tests
  // ==========================

  #[test]
  fn test_logs_response_success() {
    let logs = vec![LogEntry {
      timestamp: Utc::now(),
      level: "info".to_string(),
      message: "Test".to_string(),
      component: "test".to_string(),
      context: None,
    }];

    let response = LogsResponse::success(logs.clone());
    assert!(response.success);
    assert_eq!(response.logs.len(), 1);
    assert!(response.error.is_none());
  }

  #[test]
  fn test_logs_response_error() {
    let response = LogsResponse::error("Test error", "test_tag");
    assert!(!response.success);
    assert_eq!(response.logs.len(), 0);
    assert!(response.error.is_some());

    let error = response.error.unwrap();
    assert_eq!(error.message, "Test error");
    assert_eq!(error.tag, "test_tag");
  }
}
