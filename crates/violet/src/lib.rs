//! Violet - Simple Code Complexity Analysis
//!
//! A language-agnostic code complexity analysis tool that uses information theory
//! to measure cognitive load. No AST parsing, no language-specific rules -
//! just simple, effective complexity scoring.

pub mod simplicity;

pub use simplicity::{analyze_file, ChunkScore, FileAnalysis};
