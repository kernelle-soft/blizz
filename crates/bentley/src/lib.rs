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

use chrono::{DateTime, Local, Utc};
use colored::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "daemon-logs")]
use std::collections::VecDeque;

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
  use super::*;

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

  /// In-memory log storage for daemons
  pub struct DaemonLogs {
    entries: VecDeque<LogEntry>,
    max_entries: usize,
  }

  impl DaemonLogs {
    /// Create a new daemon log storage with the specified capacity
    pub fn new(max_entries: usize) -> Self {
      Self {
        entries: VecDeque::with_capacity(max_entries),
        max_entries,
      }
    }

    /// Add a log entry to the storage
    pub fn add_log(&mut self, level: &str, message: &str, component: &str) {
      if self.entries.len() >= self.max_entries {
        self.entries.pop_front(); // Remove oldest
      }

      self.entries.push_back(LogEntry {
        timestamp: Utc::now(),
        level: level.to_string(),
        message: message.to_string(),
        component: component.to_string(),
      });
    }

    /// Retrieve logs with optional filtering and limiting
    pub fn get_logs(&self, limit: Option<usize>, level_filter: Option<&str>) -> Vec<LogEntry> {
      let mut logs: Vec<LogEntry> = self
        .entries
        .iter()
        .filter(|entry| {
          level_filter.map_or(true, |filter| {
            filter == "all" || entry.level == filter
          })
        })
        .cloned()
        .collect();

      // Sort by timestamp (newest first)
      logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

      // Apply limit
      if let Some(limit) = limit {
        logs.truncate(limit);
      }

      logs
    }

    /// Get the current number of stored log entries
    pub fn len(&self) -> usize {
      self.entries.len()
    }

    /// Check if the log storage is empty
    pub fn is_empty(&self) -> bool {
      self.entries.is_empty()
    }

    /// Get the maximum number of entries this storage can hold
    pub fn max_capacity(&self) -> usize {
      self.max_entries
    }
  }
}
