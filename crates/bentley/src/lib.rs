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

// Constants
// ========

/// Default banner width for theatrical functions
const DEFAULT_BANNER_WIDTH: usize = 50;

/// Default prefix width for standard logging
const PREFIX_WIDTH: usize = 7;

// Core Functions
// ==============

/// Core logging function that handles the actual output
pub fn log(message: &str) {
  for line in message.lines() {
    eprintln!("{line}");
  }
}

// Utility Functions
// =================

/// Format a colored prefix for log messages
fn format_prefix(color: Color, prefix: &str) -> String {
  format!("[{}]{:<width$}", prefix.color(color).bold(), "", width = PREFIX_WIDTH - prefix.len() - 2)
}

/// Create a banner line of the specified length and character
#[cfg(not(tarpaulin_include))]
pub fn banner_line(length: usize, char: char) -> String {
  char.to_string().repeat(length)
}

/// Display a message with a banner around it
#[cfg(not(tarpaulin_include))]
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

// Logging Functions
// =================

/// Info level logging - general information
#[cfg(not(tarpaulin_include))]
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
#[cfg(not(tarpaulin_include))]
pub fn debug(message: &str) {
  let prefix = format_prefix(Color::Magenta, "debug");
  for line in message.lines() {
    log(&format!("{prefix} {line}"));
  }
}

/// Success level logging - something completed successfully
#[cfg(not(tarpaulin_include))]
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
#[cfg(not(tarpaulin_include))]
pub fn fail(message: &str) {
  let prefix = format_prefix(Color::BrightRed, "fail");
  for line in message.lines() {
    log(&format!("{prefix} {line}"));
  }
}

/// Theatrical announcement - for important but not critical messages
#[cfg(not(tarpaulin_include))]
pub fn announce(message: &str) {
  as_banner(|msg| log(&msg.blue().bold().to_string()), message, Some(50), Some('-'));
}

/// Spotlight - highlight important information
#[cfg(not(tarpaulin_include))]
pub fn spotlight(message: &str) {
  as_banner(|msg| log(&msg.yellow().bold().to_string()), message, Some(40), Some('*'));
}

/// Flourish - celebrate successful completion
#[cfg(not(tarpaulin_include))]
pub fn flourish(message: &str) {
  as_banner(|msg| log(&msg.green().bold().to_string()), message, Some(45), Some('~'));
}

/// Show stopper - for critical announcements
#[cfg(not(tarpaulin_include))]
pub fn showstopper(message: &str) {
  as_banner(|msg| log(&msg.bright_red().bold().to_string()), message, Some(60), Some('*'));
}

// Exported Macros
// ===============

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

// Daemon Logging
// ==============

/// Daemon logging infrastructure - available with "daemon-logs" feature
#[cfg(feature = "daemon-logs")]
pub mod daemon_logs;

// Re-export daemon_logs module contents for convenience
#[cfg(feature = "daemon-logs")]
pub use daemon_logs::{DaemonLogs, ErrorInfo, LogEntry, LogsRequest, LogsResponse};

// Tests
// =====

#[cfg(test)]
mod tests {
  use super::*;
  use colored::Color;

  // Utility Function Tests
  // ======================

  #[test]
  fn test_format_prefix_basic() {
    let result = format_prefix(Color::Blue, "info");

    // Should contain the prefix text
    assert!(result.contains("info"));

    // Should start with opening bracket
    assert!(result.starts_with('['));

    // Should be longer than just the prefix text due to brackets and formatting
    assert!(result.len() > "info".len());

    // Should produce a reasonable minimum size (at least 6 chars for "[info]")
    assert!(result.len() >= 6);
  }

  #[test]
  fn test_format_prefix_different_colors() {
    // Test different colors don't break the formatting
    let info_prefix = format_prefix(Color::Blue, "info");
    let warn_prefix = format_prefix(Color::Yellow, "warn");
    let error_prefix = format_prefix(Color::Red, "error");

    // All should be longer than base text due to color codes
    assert!(info_prefix.len() > "info".len());
    assert!(warn_prefix.len() > "warn".len());
    assert!(error_prefix.len() > "error".len());

    // Each should contain their respective text
    assert!(info_prefix.contains("info"));
    assert!(warn_prefix.contains("warn"));
    assert!(error_prefix.contains("error"));

    // All should start with brackets
    assert!(info_prefix.starts_with('['));
    assert!(warn_prefix.starts_with('['));
    assert!(error_prefix.starts_with('['));
  }

  #[test]
  fn test_format_prefix_different_lengths() {
    // Test prefixes of different lengths
    let short_prefix = format_prefix(Color::Green, "ok");
    let long_prefix = format_prefix(Color::Red, "error");

    // Both should be longer than their base text due to color codes and formatting
    assert!(short_prefix.len() > "ok".len());
    assert!(long_prefix.len() > "error".len());

    // Should contain the text
    assert!(short_prefix.contains("ok"));
    assert!(long_prefix.contains("error"));

    // Both should have consistent bracket formatting
    assert!(short_prefix.starts_with('['));
    assert!(long_prefix.starts_with('['));
  }

  #[test]
  fn test_banner_line_basic() {
    // Test basic functionality
    assert_eq!(banner_line(5, '='), "=====");
    assert_eq!(banner_line(3, '-'), "---");
    assert_eq!(banner_line(1, '*'), "*");
  }

  #[test]
  fn test_banner_line_edge_cases() {
    // Test edge cases
    assert_eq!(banner_line(0, '='), "");
    assert_eq!(banner_line(10, '~'), "~~~~~~~~~~");

    // Different characters
    assert_eq!(banner_line(4, '#'), "####");
    assert_eq!(banner_line(7, '.'), ".......");
  }

  #[test]
  fn test_banner_line_unicode_chars() {
    // Test with unicode characters
    assert_eq!(banner_line(3, '★'), "★★★");
    assert_eq!(banner_line(4, '▲'), "▲▲▲▲");
  }

  // Banner Formatting Tests
  // =======================

  #[test]
  fn test_as_banner_calls_function() {
    // Test that as_banner calls the provided function correctly
    use std::sync::{Arc, Mutex};

    let messages = Arc::new(Mutex::new(Vec::new()));
    let messages_clone = Arc::clone(&messages);

    let capture_fn = |msg: &str| {
      messages_clone.lock().unwrap().push(msg.to_string());
    };

    as_banner(capture_fn, "Test Message", Some(10), Some('*'));

    let captured = messages.lock().unwrap();
    assert_eq!(captured.len(), 3); // border + message + border
    assert_eq!(captured[0], "**********"); // top border
    assert_eq!(captured[1], "Test Message"); // message
    assert_eq!(captured[2], "**********"); // bottom border
  }

  #[test]
  fn test_as_banner_default_values() {
    use std::sync::{Arc, Mutex};

    let messages = Arc::new(Mutex::new(Vec::new()));
    let messages_clone = Arc::clone(&messages);

    let capture_fn = |msg: &str| {
      messages_clone.lock().unwrap().push(msg.to_string());
    };

    as_banner(capture_fn, "Test", None, None); // Use defaults

    let captured = messages.lock().unwrap();
    assert_eq!(captured.len(), 3);
    assert_eq!(captured[0].len(), DEFAULT_BANNER_WIDTH); // Should use default width (50)
    assert!(captured[0].starts_with("=")); // Should use default char ('=')
  }

  #[test]
  fn test_as_banner_custom_width_and_char() {
    use std::sync::{Arc, Mutex};

    let messages = Arc::new(Mutex::new(Vec::new()));
    let messages_clone = Arc::clone(&messages);

    let capture_fn = |msg: &str| {
      messages_clone.lock().unwrap().push(msg.to_string());
    };

    as_banner(capture_fn, "Custom", Some(15), Some('@'));

    let captured = messages.lock().unwrap();
    assert_eq!(captured[0], "@@@@@@@@@@@@@@@"); // 15 '@' characters
    assert_eq!(captured[1], "Custom");
    assert_eq!(captured[2], "@@@@@@@@@@@@@@@"); // 15 '@' characters
  }

  // Constants Tests
  // ===============

  #[test]
  fn test_constants_are_reasonable() {
    // Verify our constants have reasonable values
    // Note: These are compile-time constants, but we test them for documentation
    const _: () = assert!(DEFAULT_BANNER_WIDTH > 0);
    const _: () = assert!(PREFIX_WIDTH > 0);

    // PREFIX_WIDTH should be reasonable for our prefixes
    const _: () = assert!(PREFIX_WIDTH >= 7); // "[info] " = 7 chars

    // DEFAULT_BANNER_WIDTH should be reasonable for banners
    const _: () = assert!(DEFAULT_BANNER_WIDTH >= 20);

    // Runtime verification that constants are accessible
    assert_eq!(DEFAULT_BANNER_WIDTH, 50);
    assert_eq!(PREFIX_WIDTH, 7);
  }
}
