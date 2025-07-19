//! Violet - Simple Code Complexity Analysis
//!
//! A language-agnostic code complexity analysis tool that uses information theory
//! to measure cognitive load. No AST parsing, no language-specific rules -
//! just simple, effective complexity scoring.

pub mod config;
pub mod simplicity;

pub use config::VioletConfig;
pub use simplicity::{analyze_file, ComplexityRegion, RegionType, FileAnalysis};
