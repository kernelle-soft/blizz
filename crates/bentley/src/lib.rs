//! Bentley - The Town Crier of Kernelle
//! 
//! A theatrical logging and output formatting library that serves as the voice
//! for all tools in the Kernelle workspace. Bentley provides structured, colorful,
//! and contextual output formatting with the dramatic flair of a three-ring circus!
//! 
//! ## The Town Crier's Role
//! 
//! Bentley doesn't perform on his own - he amplifies the voices of others:
//! - **Jerrod** uses Bentley to announce MR review progress
//! - **Violet** calls upon Bentley to report code quality findings  
//! - **Blizz** has Bentley spotlight important insights
//! - **Adam** uses Bentley to flourish when knowledge is curated
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
//! ```rust
//! use bentley::*;
//! 
//! // Standard logging
//! info("Starting the operation...");
//! success("Task completed successfully!");
//! 
//! // Theatrical announcements
//! announce("Major milestone achieved!");
//! spotlight("Critical information highlighted!");
//! flourish("Celebrating completion!");
//! ```

use colored::*;
use chrono::Local;

/// The padding width for log level prefixes
const PADDING: usize = 10;

/// Initialize the Bentley logging system
pub fn init() {
    println!("🎪 {} 🎪", "Bentley logging system initialized".bright_green().bold());
}

/// Core logging function that writes to stderr (like the bash version)
pub fn log(message: &str) {
    eprintln!("{}", message);
}

/// Log multi-line text with a consistent prefix
pub fn log_multiline(prefix: &str, message: &str) {
    for line in message.lines() {
        log(&format!("{} {}", prefix, line));
    }
}

/// Format a prefix for a log level with theatrical flair
fn format_prefix(color: Color, label: &str) -> String {
    let formatted = format!("[{}]:", label).color(color).bold().to_string();
    format!("{:<width$}", formatted, width = PADDING)
}

/// Info level logging - for general information
pub fn info(message: &str) {
    let prefix = format_prefix(Color::Green, "info");
    
    if message.contains('\n') {
        log_multiline(&prefix, message);
    } else {
        log(&format!("{} {}", prefix, message));
    }
}

/// Warning level logging - for potential issues
pub fn warn(message: &str) {
    let prefix = format_prefix(Color::Yellow, "warn");
    
    if message.contains('\n') {
        log_multiline(&prefix, message);
    } else {
        log(&format!("{} {}", prefix, message));
    }
}

/// Error level logging - for serious problems
pub fn error(message: &str) {
    let prefix = format_prefix(Color::Red, "error");
    
    if message.contains('\n') {
        log_multiline(&prefix, message);
    } else {
        log(&format!("{} {}", prefix, message));
    }
}

/// Debug level logging - for development information
pub fn debug(message: &str) {
    let prefix = format_prefix(Color::Blue, "debug");
    
    if message.contains('\n') {
        log_multiline(&prefix, message);
    } else {
        log(&format!("{} {}", prefix, message));
    }
}

/// Success level logging - for positive outcomes with theatrical flair
pub fn success(message: &str) {
    let prefix = format_prefix(Color::Green, "success");
    
    if message.contains('\n') {
        log_multiline(&format!("{} ✓", prefix), message);
    } else {
        log(&format!("{} ✓ {}", prefix, message));
    }
}

/// Add timestamp to any logging function
pub fn with_timestamp<F>(log_fn: F, message: &str) 
where 
    F: Fn(&str)
{
    let now = Local::now();
    let timestamp = now.format("%Y-%m-%d %H:%M:%S");
    log_fn(&format!("{} {}", timestamp, message));
}

/// Event-level logging functions with timestamps

pub fn event_info(message: &str) {
    with_timestamp(info, message);
}

pub fn event_warn(message: &str) {
    with_timestamp(warn, message);
}

pub fn event_error(message: &str) {
    with_timestamp(error, message);
}

pub fn event_debug(message: &str) {
    with_timestamp(debug, message);
}

pub fn event_success(message: &str) {
    with_timestamp(success, message);
}

/// Create a banner line with specified width and character
pub fn banner_line(width: usize, ch: char) -> String {
    ch.to_string().repeat(width)
}

/// Display a message in a theatrical banner
pub fn as_banner<F>(log_fn: F, message: &str, width: Option<usize>, ch: Option<char>)
where
    F: Fn(&str) + Copy
{
    let banner_width = width.unwrap_or(80);
    let banner_char = ch.unwrap_or('-');
    
    let line = banner_line(banner_width, banner_char);
    
    log_fn(&line);
    
    // Wrap message content to banner width
    for chunk in message.chars().collect::<Vec<char>>().chunks(banner_width) {
        let text_line: String = chunk.iter().collect();
        log_fn(&text_line);
    }
    
    log_fn(&line);
}

/// Theatrical logging variants with extra flair

/// Ringmaster announcement - for major events
pub fn announce(message: &str) {
    let prefix = "🎪".bright_magenta().bold();
    log(&format!("{} {}", prefix, message.bright_white().bold()));
}

/// Spotlight moment - for highlighting important information  
pub fn spotlight(message: &str) {
    let prefix = "✨".bright_yellow().bold();
    log(&format!("{} {}", prefix, message.bright_cyan().bold()));
}

/// Dramatic flourish - for completing major tasks
pub fn flourish(message: &str) {
    let prefix = "🎭".bright_blue().bold();
    log(&format!("{} {}", prefix, message.bright_green().bold()));
}

/// Show stopper - for critical announcements
pub fn showstopper(message: &str) {
    as_banner(|msg| log(&msg.bright_red().bold().to_string()), message, Some(60), Some('*'));
} 