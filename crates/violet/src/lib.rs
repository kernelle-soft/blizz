//! Language-agnostic code complexity analysis using information theory

pub mod chunking;
pub mod config;
pub mod directives;
pub mod scoring;
pub mod simplicity;

pub use config::VioletConfig;
pub use simplicity::{analyze_file, FileAnalysis};
