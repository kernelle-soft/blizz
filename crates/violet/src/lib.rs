//! Violet - Code Complexity Artisan
//! 
//! "Every line of code should be a masterpiece"
//! 
//! A local-only code complexity analysis and style enforcement tool
//! that promotes functional programming patterns and beautiful code.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

pub mod config;
pub mod parser;
pub mod metrics;
pub mod linter;

/// Core error type for Violet operations
#[derive(thiserror::Error, Debug)]
pub enum VioletError {
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Parser error: {0}")]
    Parser(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    JavaScript,
    TypeScript,
    Python,
    Rust,
    Bash,
    Go,
    Ruby,
}

impl Language {
    /// Get language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "js" | "mjs" | "cjs" => Some(Language::JavaScript),
            "ts" | "tsx" => Some(Language::TypeScript),
            "py" | "pyw" => Some(Language::Python),
            "rs" => Some(Language::Rust),
            "sh" | "bash" => Some(Language::Bash),
            "go" => Some(Language::Go),
            "rb" => Some(Language::Ruby),
            _ => None,
        }
    }
    
    /// Get file extensions for this language
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Language::JavaScript => &["js", "mjs", "cjs"],
            Language::TypeScript => &["ts", "tsx"],
            Language::Python => &["py", "pyw"],
            Language::Rust => &["rs"],
            Language::Bash => &["sh", "bash"],
            Language::Go => &["go"],
            Language::Ruby => &["rb"],
        }
    }
}

/// Code complexity metrics for a function or file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Number of parameters in function signature
    pub param_count: usize,
    /// Number of lines in function/file
    pub line_count: usize,
    /// Maximum nesting depth
    pub max_depth: usize,
    /// Cyclomatic complexity
    pub cyclomatic_complexity: usize,
    /// Start line number
    pub start_line: usize,
    /// End line number
    pub end_line: usize,
}

/// A violation of code quality rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// Type of rule violated
    pub rule: String,
    /// Severity level
    pub severity: Severity,
    /// Human-readable message
    pub message: String,
    /// File path
    pub file: PathBuf,
    /// Line number where violation occurs
    pub line: usize,
    /// Column number (optional)
    pub column: Option<usize>,
    /// Suggested fix (optional)
    pub suggestion: Option<String>,
}

/// Severity levels for violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Result type for Violet operations
pub type Result<T> = std::result::Result<T, VioletError>; 