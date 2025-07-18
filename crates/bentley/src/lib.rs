// violet ignore chunk
//! Bentley - The Town Crier of Kernelle
//!
//! A theatrical logging library that brings the dramatic flair of a three-ring circus
//! to your terminal output. Bentley serves as the voice for all Kernelle tools,
//! providing structured logging with personality and contextual formatting.
//!
//! ## The Town Crier's Role
//!
//! Bentley doesn't perform on his own - he amplifies the voices of others:
//! - **Jerrod** uses Bentley to announce MR review progress
//! - **Violet** calls upon Bentley to report code quality findings  
//! - **Blizz** has Bentley spotlight important insights
//!
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
fn log(message: &str) {
  for line in message.lines() {
    eprintln!("{line}");
  }
}

/// Format a colored prefix for log messages
fn format_prefix(color: Color, prefix: &str) -> String {
  format!("[{}]", prefix.color(color).bold())
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
  let prefix = format_prefix(Color::Green, "success");
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
