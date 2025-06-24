//! Configuration management for Violet
//!
//! Handles loading, validating, and managing complexity thresholds
//! and rule configurations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
fn default_max_params() -> usize {
  3
}
fn default_max_function_lines() -> usize {
  50
}
fn default_max_function_depth() -> usize {
  3
}
fn default_max_complexity() -> usize {
  10
}
fn default_max_file_lines() -> usize {
  500
}
fn default_max_file_depth() -> usize {
  4
}

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
    let config_paths = [".violet.json", "violet.json", ".violet/config.json"];

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
    self.language_overrides.get(&language).cloned().unwrap_or_else(|| self.rules.clone())
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
  use std::fs;
  use tempfile::TempDir;

  fn create_test_config() -> Config {
    Config {
      rules: RuleThresholds {
        max_params: 3,
        max_function_lines: 50,
        max_function_depth: 3,
        max_complexity: 10,
        max_file_lines: 500,
        max_file_depth: 2,
      },
      language_overrides: std::collections::HashMap::new(),
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

  #[test]
  fn test_rules_default() {
    let rules = RuleThresholds::default();
    assert_eq!(rules.max_params, 3);
    assert_eq!(rules.max_function_lines, 50);
    assert_eq!(rules.max_function_depth, 3);
    assert_eq!(rules.max_complexity, 10);
    assert_eq!(rules.max_file_lines, 500);
    assert_eq!(rules.max_file_depth, 4);
  }

  #[test]
  fn test_config_default() {
    let config = Config::default();
    assert_eq!(config.rules.max_params, 3);
    assert_eq!(config.rules.max_function_lines, 50);
    assert!(config.language_overrides.is_empty());
    assert!(config.ignore.contains(&"node_modules/**".to_string()));
  }

  #[test]
  fn test_config_load_nonexistent_file() {
    let result = Config::load_from_file(Path::new("nonexistent.json"));
    assert!(result.is_err()); // Should error on nonexistent file
  }

  #[test]
  fn test_config_load_valid_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("violet.json");

    let config_content = r#"{
            "rules": {
                "max_params": 5,
                "max_function_lines": 100,
                "max_function_depth": 4,
                "max_complexity": 15,
                "max_file_lines": 1000,
                "max_file_depth": 3
            },
            "language_overrides": {
                "javascript": {
                    "max_params": 4,
                    "max_complexity": 12,
                    "max_function_lines": 60,
                    "max_function_depth": 2,
                    "max_file_lines": 800,
                    "max_file_depth": 3
                }
            }
        }"#;

    fs::write(&config_path, config_content).unwrap();

    let config = Config::load_from_file(&config_path).unwrap();
    assert_eq!(config.rules.max_params, 5);
    assert_eq!(config.rules.max_function_lines, 100);
    assert_eq!(config.rules.max_function_depth, 4);
    assert_eq!(config.rules.max_complexity, 15);
    assert_eq!(config.rules.max_file_lines, 1000);
    assert_eq!(config.rules.max_file_depth, 3);

    let js_override = config.language_overrides.get(&Language::JavaScript).unwrap();
    assert_eq!(js_override.max_params, 4);
    assert_eq!(js_override.max_complexity, 12);
    assert_eq!(js_override.max_function_lines, 60);
  }

  #[test]
  fn test_config_load_invalid_json() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid.json");

    fs::write(&config_path, "{ invalid json }").unwrap();

    let result = Config::load_from_file(&config_path);
    assert!(result.is_err());
  }

  #[test]
  fn test_config_load_partial_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("partial.json");

    let config_content = r#"{
            "rules": {
                "max_params": 7
            }
        }"#;

    fs::write(&config_path, config_content).unwrap();

    let config = Config::load_from_file(&config_path).unwrap();
    assert_eq!(config.rules.max_params, 7);
    // Other fields should have defaults
    assert_eq!(config.rules.max_function_lines, 50);
    assert_eq!(config.rules.max_complexity, 10);
  }

  #[test]
  fn test_config_get_rules_no_override() {
    let config = create_test_config();
    let rules = config.get_rules_for_language(Language::Rust);

    assert_eq!(rules.max_params, 3);
    assert_eq!(rules.max_function_lines, 50);
    assert_eq!(rules.max_function_depth, 3);
    assert_eq!(rules.max_complexity, 10);
    assert_eq!(rules.max_file_lines, 500);
    assert_eq!(rules.max_file_depth, 2);
  }

  #[test]
  fn test_config_get_rules_with_override() {
    let mut config = create_test_config();

    let override_rules = RuleThresholds {
      max_params: 5,
      max_function_lines: 75,
      max_function_depth: 4,
      max_complexity: 15,
      max_file_lines: 800,
      max_file_depth: 3,
    };

    config.language_overrides.insert(Language::JavaScript, override_rules);

    let rules = config.get_rules_for_language(Language::JavaScript);
    assert_eq!(rules.max_params, 5);
    assert_eq!(rules.max_function_lines, 75);
    assert_eq!(rules.max_function_depth, 4);
    assert_eq!(rules.max_complexity, 15);
    assert_eq!(rules.max_file_lines, 800);
    assert_eq!(rules.max_file_depth, 3);
  }

  #[test]
  fn test_config_get_rules_no_override_returns_default() {
    let config = create_test_config();

    let rules = config.get_rules_for_language(Language::Python);
    // Should return the base rules since no override exists
    assert_eq!(rules.max_params, config.rules.max_params);
    assert_eq!(rules.max_function_lines, config.rules.max_function_lines);
    assert_eq!(rules.max_complexity, config.rules.max_complexity);
  }

  #[test]
  fn test_rules_equality() {
    let rules1 = RuleThresholds::default();
    let rules2 = RuleThresholds::default();
    assert_eq!(rules1.max_params, rules2.max_params);
    assert_eq!(rules1.max_function_lines, rules2.max_function_lines);

    let rules3 = RuleThresholds { max_params: 5, ..RuleThresholds::default() };
    assert_ne!(rules1.max_params, rules3.max_params);
  }

  #[test]
  fn test_config_serialization() {
    let config = create_test_config();
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: Config = serde_json::from_str(&json).unwrap();
    assert_eq!(config.rules, deserialized.rules);
  }

  #[test]
  fn test_rule_thresholds_serialization() {
    let rules = RuleThresholds {
      max_params: 6,
      max_function_lines: 80,
      max_function_depth: 2,
      max_complexity: 20,
      max_file_lines: 1200,
      max_file_depth: 5,
    };

    let json = serde_json::to_string(&rules).unwrap();
    let deserialized: RuleThresholds = serde_json::from_str(&json).unwrap();

    assert_eq!(rules, deserialized);
  }

  #[test]
  fn test_config_with_multiple_language_overrides() {
    let mut config = Config::default();

    let js_override = RuleThresholds {
      max_params: 4,
      max_complexity: 12,
      max_function_lines: 60,
      max_function_depth: 2,
      max_file_lines: 800,
      max_file_depth: 3,
    };

    let py_override = RuleThresholds {
      max_params: 5,
      max_function_lines: 60,
      max_function_depth: 4,
      max_complexity: 15,
      max_file_lines: 800,
      max_file_depth: 3,
    };

    config.language_overrides.insert(Language::JavaScript, js_override.clone());
    config.language_overrides.insert(Language::Python, py_override.clone());

    // Test JavaScript overrides
    let js_rules = config.get_rules_for_language(Language::JavaScript);
    assert_eq!(js_rules, js_override);

    // Test Python overrides
    let py_rules = config.get_rules_for_language(Language::Python);
    assert_eq!(py_rules, py_override);

    // Test unoverridden language
    let rust_rules = config.get_rules_for_language(Language::Rust);
    assert_eq!(rust_rules, config.rules);
  }

  #[test]
  fn test_should_ignore_file() {
    let config = Config::default();

    // Test directory ignoring
    assert!(config.should_ignore_file(Path::new("node_modules/test.js")));
    assert!(config.should_ignore_file(Path::new("target/debug/main.rs")));
    assert!(config.should_ignore_file(Path::new("build/output.js")));

    // Test files that shouldn't be ignored
    assert!(!config.should_ignore_file(Path::new("src/main.rs")));
    assert!(!config.should_ignore_file(Path::new("lib/utils.js")));
    assert!(!config.should_ignore_file(Path::new("test.py")));
  }

  #[test]
  fn test_glob_matching() {
    // Test the internal glob_match function
    assert!(glob_match("*.js", "test.js"));
    assert!(glob_match("src/*.rs", "src/main.rs"));
    assert!(glob_match("node_modules/**", "node_modules/package/index.js"));
    assert!(!glob_match("*.js", "test.py"));
    assert!(!glob_match("src/*.rs", "lib/main.rs"));
  }

  #[test]
  fn test_config_load_and_save() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.json");

    let original_config = Config {
      rules: RuleThresholds {
        max_params: 7,
        max_function_lines: 120,
        max_function_depth: 5,
        max_complexity: 25,
        max_file_lines: 1500,
        max_file_depth: 6,
      },
      language_overrides: HashMap::new(),
      ignore: vec!["test/**".to_string()],
      ignore_dirs: vec!["test".to_string()],
    };

    // Save config
    original_config.save_to_file(&config_path).unwrap();

    // Load config
    let loaded_config = Config::load_from_file(&config_path).unwrap();

    assert_eq!(original_config.rules, loaded_config.rules);
    assert_eq!(original_config.ignore, loaded_config.ignore);
  }
}
