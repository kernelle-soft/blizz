// violet ignore chunk
//! ## Features
//!
//! - Standard logging levels (info, warn, error, debug, success)
//! - Multi-line message support with consistent formatting
//! - Timestamp functions for event logging
//! - Theatrical enhancements (announce, spotlight, flourish, showstopper)
//! - Banner displays for important messages
//! - All output to stderr (compatible with bash logging.sh)
//!
//! ## Usage
//!
//! Standard logging functions: `info()`, `warn()`, `error()`, `debug()`, `success()`
//!
//! Theatrical functions: `announce()`, `spotlight()`, `flourish()`, `showstopper()`
//!
//! Event logging: `event_info()`, `event_warn()`, `event_error()`, `event_debug()`, `event_success()`

use chrono::Local;
use colored::*;

/// Initialize Bentley - sets up any necessary state
pub fn init() {
  // For now, this is a no-op, but provides a hook for future initialization
}

/// Core logging function that handles the actual output
pub fn log(message: &str) {
  for line in message.lines() {
    eprintln!("{line}");
  }
}

/// Format a colored prefix for log messages
fn format_prefix(color: Color, prefix: &str) -> String {
  format!("[{}]{:<width$}", prefix.color(color).bold(), "", width = 7 - prefix.len() - 2)
}

/// Create a banner line of the specified length and character
pub fn banner_line(length: usize, char: char) -> String {
  char.to_string().repeat(length)
}

/// Display a message with a banner around it
pub fn as_banner<F>(log_fn: F, message: &str, width: Option<usize>, border_char: Option<char>)
where
  F: Fn(&str),
{
  let width = width.unwrap_or(50);
  let border_char = border_char.unwrap_or('=');

  let banner = banner_line(width, border_char);

  log_fn(&banner);
  log_fn(message);
  log_fn(&banner);
}

pub fn verbose(message: &str) {
  let prefix = format_prefix(Color::Cyan, "verb");
  for line in message.lines() {
    log(&format!("{prefix} {line}"));
  }
}

/// Info level logging - general information
pub fn info(message: &str) {
  let prefix = format_prefix(Color::Blue, "info");
  for line in message.lines() {
    log(&format!("{prefix} {line}"));
  }
}

/// Warning level logging - something needs attention
pub fn warn(message: &str) {
  let prefix = format_prefix(Color::Yellow, "warn");
  for line in message.lines() {
    log(&format!("{prefix} {line}"));
  }
}

/// Error level logging - something went wrong
pub fn error(message: &str) {
  let prefix = format_prefix(Color::Red, "error");
  for line in message.lines() {
    log(&format!("{prefix} {line}"));
  }
}

pub fn fail(message: &str) {
  let prefix = format_prefix(Color::BrightRed, "fail");
  for line in message.lines() {
    log(&format!("{prefix} {line}"));
  }
}

/// Debug level logging - detailed diagnostic information
pub fn debug(message: &str) {
  let prefix = format_prefix(Color::Magenta, "debug");
  for line in message.lines() {
    log(&format!("{prefix} {line}"));
  }
}

/// Success level logging - something completed successfully
pub fn success(message: &str) {
  let prefix = format_prefix(Color::Green, "sccs");
  for line in message.lines() {
    log(&format!("{prefix} {line}"));
  }
}

/// Timestamped info event
pub fn event_info(message: &str) {
  let timestamp = Local::now().format("%H:%M:%S").to_string();
  let prefix = format!("[{}] [{}]", "event".blue().bold(), timestamp.cyan());
  for line in message.lines() {
    log(&format!("{prefix} {line}"));
  }
}

/// Timestamped warning event
pub fn event_warn(message: &str) {
  let timestamp = Local::now().format("%H:%M:%S").to_string();
  let prefix = format!("[{}] [{}]", "event".yellow().bold(), timestamp.cyan());
  for line in message.lines() {
    log(&format!("{prefix} {line}"));
  }
}

/// Timestamped error event
pub fn event_error(message: &str) {
  let timestamp = Local::now().format("%H:%M:%S").to_string();
  let prefix = format!("[{}] [{}]", "event".red().bold(), timestamp.cyan());
  for line in message.lines() {
    log(&format!("{prefix} {line}"));
  }
}

/// Timestamped debug event
pub fn event_debug(message: &str) {
  let timestamp = Local::now().format("%H:%M:%S").to_string();
  let prefix = format!("[{}] [{}]", "event".magenta().bold(), timestamp.cyan());
  for line in message.lines() {
    log(&format!("{prefix} {line}"));
  }
}

/// Timestamped success event
pub fn event_success(message: &str) {
  let timestamp = Local::now().format("%H:%M:%S").to_string();
  let prefix = format!("[{}] [{}]", "event".green().bold(), timestamp.cyan());
  for line in message.lines() {
    log(&format!("{prefix} {line}"));
  }
}

/// Theatrical announcement - for important but not critical messages
pub fn announce(message: &str) {
  as_banner(|msg| log(&msg.blue().bold().to_string()), message, Some(50), Some('-'));
}

/// Spotlight - highlight important information
pub fn spotlight(message: &str) {
  as_banner(|msg| log(&msg.yellow().bold().to_string()), message, Some(40), Some('*'));
}

/// Flourish - celebrate successful completion
pub fn flourish(message: &str) {
  as_banner(|msg| log(&msg.green().bold().to_string()), message, Some(45), Some('~'));
}

/// Show stopper - for critical announcements
pub fn showstopper(message: &str) {
  as_banner(|msg| log(&msg.bright_red().bold().to_string()), message, Some(60), Some('*'));
}

/// Macros for coverage-excluded logging - these expand with LCOV_EXCL_LINE at call sites
#[macro_export]
macro_rules! info {
  ($msg:expr) => {
    $crate::info($msg); // LCOV_EXCL_LINE
  };
}

#[macro_export]
macro_rules! warn {
  ($msg:expr) => {
    $crate::warn($msg); // LCOV_EXCL_LINE
  };
}

#[macro_export]
macro_rules! error {
  ($msg:expr) => {
    $crate::error($msg); // LCOV_EXCL_LINE
  };
}

#[macro_export]
macro_rules! verbose {
  ($msg:expr) => {
    $crate::verbose($msg); // LCOV_EXCL_LINE
  };
}

#[macro_export]
macro_rules! debug {
  ($msg:expr) => {
    $crate::debug($msg); // LCOV_EXCL_LINE
  };
}

#[macro_export]
macro_rules! success {
  ($msg:expr) => {
    $crate::success($msg); // LCOV_EXCL_LINE
  };
}

#[macro_export]
macro_rules! announce {
  ($msg:expr) => {
    $crate::announce($msg); // LCOV_EXCL_LINE
  };
}

#[macro_export]
macro_rules! event_info {
  ($msg:expr) => {
    $crate::event_info($msg); // LCOV_EXCL_LINE
  };
}

#[macro_export]
macro_rules! event_warn {
  ($msg:expr) => {
    $crate::event_warn($msg); // LCOV_EXCL_LINE
  };
}

#[macro_export]
macro_rules! event_error {
  ($msg:expr) => {
    $crate::event_error($msg); // LCOV_EXCL_LINE
  };
}

#[macro_export]
macro_rules! event_debug {
  ($msg:expr) => {
    $crate::event_debug($msg); // LCOV_EXCL_LINE
  };
}

#[macro_export]
macro_rules! event_success {
  ($msg:expr) => {
    $crate::event_success($msg); // LCOV_EXCL_LINE
  };
}

#[macro_export]
macro_rules! spotlight {
  ($msg:expr) => {
    $crate::spotlight($msg); // LCOV_EXCL_LINE
  };
}

#[macro_export]
macro_rules! flourish {
  ($msg:expr) => {
    $crate::flourish($msg); // LCOV_EXCL_LINE
  };
}

#[macro_export]
macro_rules! showstopper {
  ($msg:expr) => {
    $crate::showstopper($msg); // LCOV_EXCL_LINE
  };
}

/// Daemon logging infrastructure - available with "daemon-logs" feature
#[cfg(feature = "daemon-logs")]
pub mod daemon_logs {
  use serde::{Deserialize, Serialize};
  use chrono::{DateTime, Utc};

  /// A structured log entry for daemon operations
  #[derive(Debug, Serialize, Deserialize, Clone)]
  pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub component: String,
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

  impl LogsResponse {
    pub fn success(logs: Vec<LogEntry>) -> Self {
      Self { success: true, logs, error: None }
    }

    pub fn error(message: &str, tag: &str) -> Self {
      Self {
        success: false,
        logs: Vec::new(),
        error: Some(ErrorInfo {
          message: message.to_string(),
          tag: tag.to_string(),
        }),
      }
    }
  }

  /// Internal log storage implementation
  struct DaemonLogsInner {
    log_file_path: std::path::PathBuf,
  }

  /// Thread-safe disk-based log storage for daemons using JSONL format
  pub struct DaemonLogs {
    inner: std::sync::Arc<tokio::sync::Mutex<DaemonLogsInner>>,
  }

  impl DaemonLogsInner {
    /// Create a new daemon log storage that writes to the specified file path
    fn new<P: AsRef<std::path::Path>>(log_file_path: P) -> std::io::Result<Self> {
      let log_file_path = log_file_path.as_ref().to_path_buf();
      
      // Ensure parent directory exists
      if let Some(parent) = log_file_path.parent() {
        std::fs::create_dir_all(parent)?;
      }
      
      // Create file if it doesn't exist (but don't truncate if it does)
      if !log_file_path.exists() {
        std::fs::File::create(&log_file_path)?;
      }

      Ok(Self { log_file_path })
    }

    /// Add a log entry to storage (appends to JSONL file)
    fn add_log(&mut self, level: &str, message: &str, component: &str) -> std::io::Result<()> {
      let entry = LogEntry {
        timestamp: Utc::now(),
        level: level.to_string(),
        message: message.to_string(),
        component: component.to_string(),
      };

      // Serialize to JSON and append to file
      let json_line = serde_json::to_string(&entry)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
      
      use std::fs::OpenOptions;
      use std::io::Write;
      
      let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&self.log_file_path)?;
      
      writeln!(file, "{}", json_line)?;
      file.flush()?;
      
      Ok(())
    }

    /// Retrieve logs with optional filtering and limiting (reads from JSONL file)
    fn get_logs(&self, limit: Option<usize>, level_filter: Option<&str>) -> std::io::Result<Vec<LogEntry>> {
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
            let matches_level = level_filter.map_or(true, |filter| {
              filter == "all" || entry.level == filter
            });
            
            if matches_level {
              logs.push(entry);
            }
          },
          Err(_) => {
            // Skip malformed lines
            continue;
          }
        }
      }

      // Sort by timestamp (newest first)
      logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

      // Apply limit
      if let Some(limit) = limit {
        logs.truncate(limit);
      }

      Ok(logs)
    }

    /// Get the path to the log file
    fn log_file_path(&self) -> &std::path::Path {
      &self.log_file_path
    }

    /// Check if the log file exists and has content
    fn has_logs(&self) -> bool {
      self.log_file_path.exists() && 
        std::fs::metadata(&self.log_file_path)
          .map(|m| m.len() > 0)
          .unwrap_or(false)
    }

    /// Get the size of the log file in bytes
    fn file_size(&self) -> std::io::Result<u64> {
      let metadata = std::fs::metadata(&self.log_file_path)?;
      Ok(metadata.len())
    }
  }

  impl DaemonLogs {
    /// Create a new thread-safe daemon log storage
    pub fn new<P: AsRef<std::path::Path>>(log_file_path: P) -> std::io::Result<Self> {
      let inner = DaemonLogsInner::new(log_file_path)?;
      Ok(Self {
        inner: std::sync::Arc::new(tokio::sync::Mutex::new(inner)),
      })
    }

    /// Add a log entry (handles locking internally)
    pub async fn add_log(&self, level: &str, message: &str, component: &str) -> std::io::Result<()> {
      let mut guard = self.inner.lock().await;
      guard.add_log(level, message, component)
    }

    /// Add a log entry (fire-and-forget, ignores errors)
    pub async fn log(&self, level: &str, message: &str, component: &str) {
      let _ = self.add_log(level, message, component).await;
    }

    /// Retrieve logs with optional filtering and limiting (handles locking internally)
    pub async fn get_logs(&self, limit: Option<usize>, level_filter: Option<&str>) -> std::io::Result<Vec<LogEntry>> {
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

    /// Clone the DaemonLogs handle (cheap Arc clone)
    pub fn clone(&self) -> Self {
      Self {
        inner: std::sync::Arc::clone(&self.inner),
      }
    }
  }
}
