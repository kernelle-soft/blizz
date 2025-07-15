//! Configuration loading and merging for Violet
//!
//! Handles loading global defaults from the crate and project-specific overrides
//! from the current working directory, with proper threshold and ignore list merging.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Complete violet configuration after merging global and project configs
#[derive(Debug, Clone)]
pub struct VioletConfig {
  pub thresholds: HashMap<String, f64>,
  pub ignore_patterns: Vec<String>,
  pub default_threshold: f64,
}

/// Raw configuration file format - matches .violet.json5 structure
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ConfigFile {
  #[serde(default)]
  pub complexity: ComplexityConfig,
  #[serde(default)]
  pub ignore: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ComplexityConfig {
  #[serde(default)]
  pub thresholds: ThresholdConfig,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ThresholdConfig {
  #[serde(default = "default_threshold")]
  pub default: f64,

  /// File extension specific thresholds (e.g., ".rs": 7.0, ".md": 5.0)
  #[serde(flatten)]
  pub extensions: HashMap<String, f64>,
}

fn default_threshold() -> f64 {
  7.0
}

impl VioletConfig {
  /// Load configuration by merging global defaults with project overrides
  pub fn load() -> Result<Self> {
    let global_config = Self::load_global_config()?;
    let project_config = Self::load_project_config()?;

    Ok(Self::merge_configs(global_config, project_config))
  }

  /// Get the appropriate threshold for a given file path
  pub fn threshold_for_file<P: AsRef<Path>>(&self, file_path: P) -> f64 {
    let path = file_path.as_ref();

    // Get file extension
    if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
      let ext_key = format!(".{}", extension);
      if let Some(&threshold) = self.thresholds.get(&ext_key) {
        return threshold;
      }
    }

    // Fall back to default
    self.default_threshold
  }

  /// Check if a file should be ignored based on ignore patterns
  pub fn should_ignore<P: AsRef<Path>>(&self, file_path: P) -> bool {
    let path_str = file_path.as_ref().to_string_lossy();

    for pattern in &self.ignore_patterns {
      if Self::matches_pattern(&path_str, pattern) {
        return true;
      }
    }
    false
  }

  /// Load global configuration from the crate's .violet.json5
  fn load_global_config() -> Result<ConfigFile> {
    // Find the global config relative to the current executable or use a fallback
    let global_config_path = Self::find_global_config_path()?;

    if global_config_path.exists() {
      Self::load_config_file(&global_config_path).with_context(|| {
        format!("Failed to load global config from {}", global_config_path.display())
      })
    } else {
      // If no global config found, return default
      Ok(ConfigFile::default())
    }
  }

  /// Load project-specific configuration from current working directory
  fn load_project_config() -> Result<Option<ConfigFile>> {
    let current_dir = std::env::current_dir().context("Failed to get current working directory")?;

    let project_config_path = current_dir.join(".violet.json5");

    if project_config_path.exists() {
      let config = Self::load_config_file(&project_config_path).with_context(|| {
        format!("Failed to load project config from {}", project_config_path.display())
      })?;
      Ok(Some(config))
    } else {
      Ok(None)
    }
  }

  /// Find the global configuration file path
  fn find_global_config_path() -> Result<PathBuf> {
    // Try to find the config relative to the current executable's location
    // This allows for development and installed scenarios

    if let Ok(exe_path) = std::env::current_exe() {
      // In development: executable is in target/debug/violet or target/release/violet
      // Config would be in crates/violet/.violet.json5
      if let Some(target_dir) = exe_path.parent().and_then(|p| p.parent()) {
        // Check if we're in a target directory (development)
        if target_dir.file_name().map(|n| n == "target").unwrap_or(false) {
          if let Some(project_root) = target_dir.parent() {
            let dev_config = project_root.join("crates/violet/.violet.json5");
            if dev_config.exists() {
              return Ok(dev_config);
            }
          }
        }
      }
    }

    // Fallback: look for config in a standard location relative to executable
    // For installed binaries, this could be alongside the binary
    if let Ok(exe_path) = std::env::current_exe() {
      if let Some(exe_dir) = exe_path.parent() {
        let installed_config = exe_dir.join(".violet.json5");
        if installed_config.exists() {
          return Ok(installed_config);
        }
      }
    }

    // Final fallback: return a default path (will be checked for existence later)
    Ok(PathBuf::from(".violet.global.json5"))
  }

  /// Load a single configuration file
  fn load_config_file(path: &Path) -> Result<ConfigFile> {
    let content = std::fs::read_to_string(path)
      .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    json5::from_str(&content)
      .with_context(|| format!("Failed to parse JSON5 config file: {}", path.display()))
  }

  /// Merge global and project configurations
  fn merge_configs(global: ConfigFile, project: Option<ConfigFile>) -> Self {
    let project = project.unwrap_or_default();

    // Get the default threshold value before declaring our variable
    let default_default = default_threshold();

    // Start with global default threshold
    let mut default_threshold = global.complexity.thresholds.default;

    // Override with project default if specified
    if project.complexity.thresholds.default != default_default {
      default_threshold = project.complexity.thresholds.default;
    }

    // Merge thresholds: start with global extensions, then add/override with project
    let mut thresholds = global.complexity.thresholds.extensions.clone();
    for (ext, threshold) in project.complexity.thresholds.extensions {
      thresholds.insert(ext, threshold);
    }

    // Merge ignore patterns: deduplicate with global first, project second
    let mut ignore_set = HashSet::new();
    let mut ignore_patterns = Vec::new();

    // Add global patterns first
    for pattern in global.ignore {
      if ignore_set.insert(pattern.clone()) {
        ignore_patterns.push(pattern);
      }
    }

    // Add project patterns second
    for pattern in project.ignore {
      if ignore_set.insert(pattern.clone()) {
        ignore_patterns.push(pattern);
      }
    }

    Self { thresholds, ignore_patterns, default_threshold }
  }

  /// Enhanced glob-like pattern matching for ignore patterns
  fn matches_pattern(path: &str, pattern: &str) -> bool {
    // Handle different glob patterns

    // Directory patterns: "target/**" matches target/ and all subdirectories
    if pattern.ends_with("/**") {
      let prefix = &pattern[..pattern.len() - 3];
      return path.starts_with(prefix);
    }

    // File extension patterns: "*.json" matches any file ending in .json
    if pattern.starts_with("*.") {
      let extension = &pattern[1..]; // Include the dot: ".json"
      return path.ends_with(extension);
    }

    // General wildcard patterns: "test*file" matches "test123file"
    if pattern.contains('*') {
      if let Some(star_pos) = pattern.find('*') {
        let prefix = &pattern[..star_pos];
        let suffix = &pattern[star_pos + 1..];
        return path.starts_with(prefix) && path.ends_with(suffix);
      }
    }

    // Exact filename match: ".DS_Store" matches exactly ".DS_Store"
    path == pattern || path.ends_with(&format!("/{}", pattern))
  }
}
