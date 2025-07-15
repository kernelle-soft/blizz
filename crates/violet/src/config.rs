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

#[derive(Debug, Deserialize, Serialize)]
pub struct ThresholdConfig {
  #[serde(default = "default_threshold")]
  pub default: f64,

  /// File extension specific thresholds (e.g., ".rs": 7.0, ".md": 5.0)
  #[serde(flatten)]
  pub extensions: HashMap<String, f64>,
}

impl Default for ThresholdConfig {
  fn default() -> Self {
    Self { default: default_threshold(), extensions: HashMap::new() }
  }
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

    // Also check with normalized path (strip leading ./ if present)
    let normalized_path = if path_str.starts_with("./") { &path_str[2..] } else { &path_str };

    for pattern in &self.ignore_patterns {
      if Self::matches_pattern(&path_str, pattern)
        || Self::matches_pattern(normalized_path, pattern)
      {
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
      // If no global config found, use hardcoded sensible defaults
      Ok(Self::default_global_config())
    }
  }

  /// Provide sensible default global configuration for installed binaries
  fn default_global_config() -> ConfigFile {
    ConfigFile {
      complexity: ComplexityConfig { thresholds: ThresholdConfig::default() },
      ignore: vec![
        // Common directories
        "node_modules/**".to_string(),
        "target/**".to_string(),
        "build/**".to_string(),
        "dist/**".to_string(),
        ".git/**".to_string(),
        ".cargo/**".to_string(),
        ".github/**".to_string(),
        ".vscode/**".to_string(),
        ".DS_Store".to_string(),
        ".idea/**".to_string(),
        ".cursor/**".to_string(),
        // Binary file extensions
        "*.png".to_string(),
        "*.jpg".to_string(),
        "*.jpeg".to_string(),
        "*.gif".to_string(),
        "*.pdf".to_string(),
        "*.zip".to_string(),
        "*.tar".to_string(),
        "*.gz".to_string(),
        "*.rlib".to_string(),
        "*.so".to_string(),
        "*.dylib".to_string(),
        "*.dll".to_string(),
        // Common config/text/text-based files
        "*.md".to_string(),
        "*.mdc".to_string(),
        "*.txt".to_string(),
        "*.yaml".to_string(),
        "*.yml".to_string(),
        "*.xml".to_string(),
        "*.html".to_string(),
        "*.json".to_string(),
        "*.json5".to_string(),
        "*.toml".to_string(),
        "*.lock".to_string(),
      ],
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

    // For installed binaries, embed default global config inline
    // rather than failing to find an external file
    // This returns a path that doesn't exist, triggering use of hardcoded defaults
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

// violet ignore chunk
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_matches_pattern_exact() {
    assert!(VioletConfig::matches_pattern(".DS_Store", ".DS_Store"));
    assert!(VioletConfig::matches_pattern("path/to/.DS_Store", ".DS_Store"));
    assert!(!VioletConfig::matches_pattern("other.file", ".DS_Store"));
  }

  #[test]
  fn test_matches_pattern_directory_glob() {
    assert!(VioletConfig::matches_pattern("target/", "target/**"));
    assert!(VioletConfig::matches_pattern("target/debug", "target/**"));
    assert!(VioletConfig::matches_pattern("target/debug/deps/violet", "target/**"));
    assert!(!VioletConfig::matches_pattern("src/target", "target/**"));
    assert!(!VioletConfig::matches_pattern("other/", "target/**"));
  }

  #[test]
  fn test_matches_pattern_file_extension() {
    assert!(VioletConfig::matches_pattern("config.json", "*.json"));
    assert!(VioletConfig::matches_pattern("path/to/config.json", "*.json"));
    assert!(VioletConfig::matches_pattern("package.json5", "*.json5"));
    assert!(!VioletConfig::matches_pattern("config.yaml", "*.json"));
    assert!(!VioletConfig::matches_pattern("jsonfile", "*.json"));
  }

  #[test]
  fn test_matches_pattern_wildcard() {
    assert!(VioletConfig::matches_pattern("testfile", "test*"));
    assert!(VioletConfig::matches_pattern("test123file", "test*file"));
    assert!(VioletConfig::matches_pattern("prefix_middle_suffix", "prefix*suffix"));
    assert!(!VioletConfig::matches_pattern("wrongprefix_suffix", "prefix*suffix"));
    assert!(!VioletConfig::matches_pattern("prefix_wrong", "prefix*suffix"));
  }

  #[test]
  fn test_threshold_for_file() {
    let mut thresholds = HashMap::new();
    thresholds.insert(".rs".to_string(), 8.0);
    thresholds.insert(".js".to_string(), 6.0);

    let config = VioletConfig { thresholds, ignore_patterns: vec![], default_threshold: 7.0 };

    assert_eq!(config.threshold_for_file("main.rs"), 8.0);
    assert_eq!(config.threshold_for_file("script.js"), 6.0);
    assert_eq!(config.threshold_for_file("config.json"), 7.0); // default
    assert_eq!(config.threshold_for_file("README.md"), 7.0); // default
  }

  #[test]
  fn test_should_ignore() {
    let config = VioletConfig {
      thresholds: HashMap::new(),
      ignore_patterns: vec![
        "target/**".to_string(),
        "*.json".to_string(),
        ".DS_Store".to_string(),
        "test*".to_string(),
      ],
      default_threshold: 7.0,
    };

    // Directory patterns
    assert!(config.should_ignore("target/debug/main"));
    assert!(config.should_ignore("target/"));
    assert!(!config.should_ignore("src/target"));

    // File extension patterns
    assert!(config.should_ignore("package.json"));
    assert!(config.should_ignore("path/to/config.json"));
    assert!(!config.should_ignore("config.yaml"));

    // Exact matches
    assert!(config.should_ignore(".DS_Store"));
    assert!(config.should_ignore("some/path/.DS_Store"));
    assert!(!config.should_ignore("DS_Store"));

    // Wildcard patterns
    assert!(config.should_ignore("testfile"));
    assert!(config.should_ignore("test123"));
    assert!(!config.should_ignore("file_test"));
  }

  #[test]
  fn test_should_ignore_normalized_paths() {
    let config = VioletConfig {
      thresholds: HashMap::new(),
      ignore_patterns: vec!["src/main.rs".to_string()],
      default_threshold: 7.0,
    };

    assert!(config.should_ignore("src/main.rs"));
    assert!(config.should_ignore("./src/main.rs")); // normalized
  }

  #[test]
  fn test_merge_configs_defaults() {
    let global = ConfigFile {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 8.0, extensions: HashMap::new() },
      },
      ignore: vec!["global_pattern".to_string()],
    };

    let result = VioletConfig::merge_configs(global, None);

    assert_eq!(result.default_threshold, 8.0);
    assert_eq!(result.ignore_patterns, vec!["global_pattern"]);
  }

  #[test]
  fn test_merge_configs_project_overrides() {
    let mut global_thresholds = HashMap::new();
    global_thresholds.insert(".rs".to_string(), 8.0);
    global_thresholds.insert(".js".to_string(), 6.0);

    let global = ConfigFile {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 7.0, extensions: global_thresholds },
      },
      ignore: vec!["global1".to_string(), "global2".to_string()],
    };

    let mut project_thresholds = HashMap::new();
    project_thresholds.insert(".rs".to_string(), 9.0); // override
    project_thresholds.insert(".py".to_string(), 5.0); // new

    let project = ConfigFile {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig {
          default: 6.5, // override
          extensions: project_thresholds,
        },
      },
      ignore: vec!["project1".to_string(), "global1".to_string()], // global1 duplicate
    };

    let result = VioletConfig::merge_configs(global, Some(project));

    // Default threshold should be overridden
    assert_eq!(result.default_threshold, 6.5);

    // Extension thresholds should be merged with project taking precedence
    assert_eq!(result.thresholds.get(".rs"), Some(&9.0)); // overridden
    assert_eq!(result.thresholds.get(".js"), Some(&6.0)); // from global
    assert_eq!(result.thresholds.get(".py"), Some(&5.0)); // from project

    // Ignore patterns should be merged and deduplicated
    assert_eq!(result.ignore_patterns.len(), 3);
    assert!(result.ignore_patterns.contains(&"global1".to_string()));
    assert!(result.ignore_patterns.contains(&"global2".to_string()));
    assert!(result.ignore_patterns.contains(&"project1".to_string()));
  }

  #[test]
  fn test_merge_configs_project_default_not_changed() {
    let global = ConfigFile {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 8.0, extensions: HashMap::new() },
      },
      ignore: vec![],
    };

    // Project with default threshold same as the global default (7.0)
    let project = ConfigFile {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig {
          default: 7.0, // This is the hardcoded default
          extensions: HashMap::new(),
        },
      },
      ignore: vec![],
    };

    let result = VioletConfig::merge_configs(global, Some(project));

    // Should keep global default since project didn't really override it
    assert_eq!(result.default_threshold, 8.0);
  }

  #[test]
  fn test_default_global_config() {
    let config = VioletConfig::default_global_config();

    // Should have reasonable defaults
    assert_eq!(config.complexity.thresholds.default, 7.0);

    // Should ignore common build/dependency directories
    assert!(config.ignore.contains(&"node_modules/**".to_string()));
    assert!(config.ignore.contains(&"target/**".to_string()));
    assert!(config.ignore.contains(&".git/**".to_string()));

    // Should ignore binary file types
    assert!(config.ignore.contains(&"*.png".to_string()));
    assert!(config.ignore.contains(&"*.pdf".to_string()));

    // Should ignore common config/text files
    assert!(config.ignore.contains(&"*.md".to_string()));
    assert!(config.ignore.contains(&"*.json".to_string()));
  }

  #[test]
  fn test_threshold_config_default() {
    let config = ThresholdConfig::default();
    assert_eq!(config.default, 7.0);
    assert!(config.extensions.is_empty());
  }

  #[test]
  fn test_config_file_default() {
    let config = ConfigFile::default();
    assert_eq!(config.complexity.thresholds.default, 7.0);
    assert!(config.ignore.is_empty());
  }
}
