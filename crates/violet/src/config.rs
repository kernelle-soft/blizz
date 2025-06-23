//! Configuration management for Violet
//! 
//! Handles loading, validating, and managing complexity thresholds
//! and rule configurations.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::{Language, Result, VioletError};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Global rule thresholds
    pub rules: RuleThresholds,
    /// Language-specific overrides
    #[serde(default)]
    pub language_overrides: HashMap<Language, RuleThresholds>,
    /// Files to ignore (glob patterns)
    #[serde(default)]
    pub ignore: Vec<String>,
    /// Directories to ignore
    #[serde(default)]
    pub ignore_dirs: Vec<String>,
}

/// Rule thresholds for complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleThresholds {
    /// Maximum function parameters
    #[serde(default = "default_max_params")]
    pub max_params: usize,
    /// Maximum function length in lines
    #[serde(default = "default_max_function_lines")]
    pub max_function_lines: usize,
    /// Maximum nesting depth in functions
    #[serde(default = "default_max_function_depth")]
    pub max_function_depth: usize,
    /// Maximum cyclomatic complexity
    #[serde(default = "default_max_complexity")]
    pub max_complexity: usize,
    /// Maximum file length in lines
    #[serde(default = "default_max_file_lines")]
    pub max_file_lines: usize,
    /// Maximum nesting depth in files
    #[serde(default = "default_max_file_depth")]
    pub max_file_depth: usize,
}

// Default threshold functions
fn default_max_params() -> usize { 3 }
fn default_max_function_lines() -> usize { 50 }
fn default_max_function_depth() -> usize { 3 }
fn default_max_complexity() -> usize { 10 }
fn default_max_file_lines() -> usize { 500 }
fn default_max_file_depth() -> usize { 4 }

impl Default for RuleThresholds {
    fn default() -> Self {
        Self {
            max_params: default_max_params(),
            max_function_lines: default_max_function_lines(),
            max_function_depth: default_max_function_depth(),
            max_complexity: default_max_complexity(),
            max_file_lines: default_max_file_lines(),
            max_file_depth: default_max_file_depth(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rules: RuleThresholds::default(),
            language_overrides: HashMap::new(),
            ignore: vec![
                "node_modules/**".to_string(),
                "target/**".to_string(),
                "build/**".to_string(),
                "dist/**".to_string(),
                ".git/**".to_string(),
            ],
            ignore_dirs: vec![
                "node_modules".to_string(),
                "target".to_string(),
                "build".to_string(),
                "dist".to_string(),
                ".git".to_string(),
            ],
        }
    }
}

impl Config {
    /// Load configuration from a file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }
    
    /// Load configuration from current directory or defaults
    pub fn load() -> Result<Self> {
        let config_paths = [
            ".violet.json",
            "violet.json",
            ".violet/config.json",
        ];
        
        for path in &config_paths {
            if Path::new(path).exists() {
                return Self::load_from_file(path);
            }
        }
        
        // No config file found, use defaults
        Ok(Config::default())
    }
    
    /// Save configuration to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Get rule thresholds for a specific language
    pub fn get_rules_for_language(&self, language: Language) -> RuleThresholds {
        self.language_overrides
            .get(&language)
            .cloned()
            .unwrap_or_else(|| self.rules.clone())
    }
    
    /// Check if a file should be ignored
    pub fn should_ignore_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        
        // Check directory ignores first
        if let Some(parent) = path.parent() {
            let parent_str = parent.to_string_lossy();
            for ignore_dir in &self.ignore_dirs {
                if parent_str.contains(ignore_dir) {
                    return true;
                }
            }
        }
        
        // Check file pattern ignores
        for pattern in &self.ignore {
            if glob_match(pattern, &path_str) {
                return true;
            }
        }
        
        false
    }
}

/// Simple glob matching (basic implementation)
fn glob_match(pattern: &str, text: &str) -> bool {
    // Very basic glob matching - in production we'd use the globset crate
    if pattern.contains("**") {
        let prefix = pattern.split("**").next().unwrap_or("");
        text.contains(prefix)
    } else if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            text.starts_with(parts[0]) && text.ends_with(parts[1])
        } else {
            false
        }
    } else {
        text == pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.rules.max_params, 3);
        assert_eq!(config.rules.max_function_lines, 50);
        assert!(config.ignore.contains(&"node_modules/**".to_string()));
    }
    
    #[test]
    fn test_should_ignore_file() {
        let config = Config::default();
        assert!(config.should_ignore_file(Path::new("node_modules/test.js")));
        assert!(config.should_ignore_file(Path::new("target/debug/test")));
        assert!(!config.should_ignore_file(Path::new("src/main.rs")));
    }
} 