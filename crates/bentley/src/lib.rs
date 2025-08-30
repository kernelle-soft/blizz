// violet ignore chunk
//! Bentley - A logging and output formatting library
//!
//! ## Features
//!
//! - Standard logging levels (info, warn, error, debug, success)
//! - Multi-line message support with consistent formatting
//! - Theatrical enhancements (announce, spotlight, flourish, showstopper)
//! - Banner displays for important messages
//! - Daemon logging infrastructure (with "daemon-logs" feature)
//! - All output to stderr (compatible with bash logging.sh)
//!
//! ## Usage
//!
//! Standard logging functions: `info()`, `warn()`, `error()`, `debug()`, `success()`
//! Theatrical functions: `announce()`, `spotlight()`, `flourish()`, `showstopper()`

use colored::*;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Default banner width for theatrical functions
const DEFAULT_BANNER_WIDTH: usize = 50;

/// Default prefix width for standard logging
const PREFIX_WIDTH: usize = 7;

// ============================================================================
// CORE FUNCTIONS
// ============================================================================

/// Core logging function that handles the actual output
pub fn log(message: &str) {
  for line in message.lines() {
    eprintln!("{line}");
  }
}

// ============================================================================
// UTILITY FUNCTIONS AND HELPERS
// ============================================================================

/// Format a colored prefix for log messages
fn format_prefix(color: Color, prefix: &str) -> String {
  format!("[{}]{:<width$}", prefix.color(color).bold(), "", width = PREFIX_WIDTH - prefix.len() - 2)
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
  let width = width.unwrap_or(DEFAULT_BANNER_WIDTH);
  let border_char = border_char.unwrap_or('=');

  let banner = banner_line(width, border_char);

  log_fn(&banner);
  log_fn(message);
  log_fn(&banner);
}

// ============================================================================
// STANDARD LOGGING FUNCTIONS
// ============================================================================

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

/// Verbose level logging - detailed trace information
pub fn verbose(message: &str) {
  let prefix = format_prefix(Color::Cyan, "verb");
  for line in message.lines() {
    log(&format!("{prefix} {line}"));
  }
}

/// Fail level logging - critical failures
pub fn fail(message: &str) {
  let prefix = format_prefix(Color::BrightRed, "fail");
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

// ============================================================================
// EXPORTED MACROS
// ============================================================================

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
macro_rules! verbose {
  ($msg:expr) => {
    $crate::verbose($msg); // LCOV_EXCL_LINE
  };
}

#[macro_export]
macro_rules! announce {
  ($msg:expr) => {
    $crate::announce($msg); // LCOV_EXCL_LINE
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

// ============================================================================
// DAEMON LOGGING INFRASTRUCTURE 
// ============================================================================

/// Daemon logging infrastructure - available with "daemon-logs" feature
#[cfg(feature = "daemon-logs")]
pub mod daemon_logs;

// Re-export daemon_logs module contents for convenience
#[cfg(feature = "daemon-logs")]
pub use daemon_logs::{LogEntry, LogsRequest, LogsResponse, ErrorInfo, DaemonLogs};
