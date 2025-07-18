//! Configuration loading and merging for Violet
//!
//! Handles loading global defaults from the crate and project-specific overrides
//! from the current working directory, with proper threshold and ignore list merging.

use anyhow::{Context, Result};
use glob::Pattern;
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
  6.0
}

/// Provide sensible default global configuration for installed binaries
fn default_global_config() -> ConfigFile {
  ConfigFile {
    complexity: ComplexityConfig { thresholds: ThresholdConfig::default() },
    ignore: get_default_ignore_patterns(),
  }
}

/// Load configuration by merging global defaults with project overrides
pub fn load_config() -> Result<VioletConfig> {
  let global_config = load_global_config()?;
  let project_config = load_project_config()?;

  Ok(merge_configs(global_config, project_config))
}

/// Get the appropriate threshold for a given file path
pub fn get_threshold_for_file<P: AsRef<Path>>(config: &VioletConfig, file_path: P) -> f64 {
  let path = file_path.as_ref();

  // Get file extension
  if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
    let ext_key = format!(".{extension}");
    if let Some(&threshold) = config.thresholds.get(&ext_key) {
      return threshold;
    }
  }

  // Fall back to default
  config.default_threshold
}

/// Check if a file should be ignored
pub fn should_ignore_file<P: AsRef<Path>>(config: &VioletConfig, file_path: P) -> bool {
  let path_str = file_path.as_ref().to_string_lossy();

  // strip leading ./ if present
  let normalized_path =
    if let Some(stripped) = path_str.strip_prefix("./") { stripped } else { &path_str };

  for pattern in &config.ignore_patterns {
    if matches_pattern(&path_str, pattern) || matches_pattern(normalized_path, pattern) {
      return true;
    }
  }
  false
}

/// Load global configuration from the crate's .violet.json5
fn load_global_config() -> Result<ConfigFile> {
  // Find the global config relative to the current executable or use a fallback
  let global_config_path = find_global_config_path()?;

  if global_config_path.exists() {
    load_config_file(&global_config_path).with_context(|| {
      format!("Failed to load global config from {}", global_config_path.display())
    })
  } else {
    // If no global config found, use hardcoded sensible defaults
    Ok(default_global_config())
  }
}

/// Load project-specific configuration from current working directory
fn load_project_config() -> Result<Option<ConfigFile>> {
  let current_dir = std::env::current_dir().context("Failed to get current working directory")?;

  let project_config_path = current_dir.join(".violet.json5");

  if project_config_path.exists() {
    let config = load_config_file(&project_config_path).with_context(|| {
      format!("Failed to load project config from {}", project_config_path.display())
    })?;
    Ok(Some(config))
  } else {
    Ok(None)
  }
}

/// Check for development config path (in target/../crates/violet/.violet.json5)
fn try_development_config() -> Option<PathBuf> {
  let exe_path = std::env::current_exe().ok()?;
  let target_dir = exe_path.parent()?.parent()?;

  if target_dir.file_name()? != "target" {
    return None;
  }

  let project_root = target_dir.parent()?;
  let dev_config = project_root.join("crates/violet/.violet.json5");

  if dev_config.exists() {
    Some(dev_config)
  } else {
    None
  }
}

/// Check for installed config path (alongside executable)
fn try_installed_config() -> Option<PathBuf> {
  let exe_path = std::env::current_exe().ok()?;
  let exe_dir = exe_path.parent()?;
  let installed_config = exe_dir.join(".violet.json5");

  if installed_config.exists() {
    Some(installed_config)
  } else {
    None
  }
}

/// Find the global configuration file path
fn find_global_config_path() -> Result<PathBuf> {
  if let Some(config) = try_development_config() {
    return Ok(config);
  }

  if let Some(config) = try_installed_config() {
    return Ok(config);
  }

  // Default path that triggers hardcoded defaults
  Ok(PathBuf::from(".violet.global.json5"))
}

/// Load a single configuration file
fn load_config_file(path: &Path) -> Result<ConfigFile> {
  let content = std::fs::read_to_string(path)
    .with_context(|| format!("Failed to read config file: {}", path.display()))?;

  json5::from_str(&content)
    .with_context(|| format!("Failed to parse JSON5 config file: {}", path.display()))
}

/// Merge ignore patterns from global and project configs, removing duplicates
fn merge_ignore_patterns(
  global_patterns: Vec<String>,
  project_patterns: Vec<String>,
) -> Vec<String> {
  let mut ignore_set = HashSet::new();
  let mut result = Vec::new();

  for pattern in global_patterns.into_iter().chain(project_patterns) {
    if ignore_set.insert(pattern.clone()) {
      result.push(pattern);
    }
  }

  result
}

/// Merge global and project configurations
fn merge_configs(global: ConfigFile, project: Option<ConfigFile>) -> VioletConfig {
  let project = project.unwrap_or_default();

  // Use project default threshold if different from default, otherwise use global
  let default_threshold = if project.complexity.thresholds.default != default_threshold() {
    project.complexity.thresholds.default
  } else {
    global.complexity.thresholds.default
  };

  // Merge thresholds: start with global, override with project
  let mut thresholds = global.complexity.thresholds.extensions.clone();
  for (ext, threshold) in project.complexity.thresholds.extensions {
    thresholds.insert(ext, threshold);
  }

  let ignore_patterns = merge_ignore_patterns(global.ignore, project.ignore);

  VioletConfig { thresholds, ignore_patterns, default_threshold }
}

/// Enhanced glob-like pattern matching for ignore patterns
fn matches_pattern(path: &str, pattern: &str) -> bool {
  // Try to create a glob pattern
  let glob_pattern = match Pattern::new(pattern) {
    Ok(p) => p,
    Err(_) => return false, // Invalid pattern
  };

  // Direct glob match
  if glob_pattern.matches(path) {
    return true;
  }

  // Special case: if pattern doesn't contain path separators,
  // also try matching it as a filename anywhere in the path
  if !pattern.contains('/') && !pattern.contains('\\') {
    // Try matching as "*/pattern" to catch files in any directory
    if let Ok(filename_pattern) = Pattern::new(&format!("*/{pattern}")) {
      if filename_pattern.matches(path) {
        return true;
      }
    }
  }

  false
}

// violet ignore chunk - This is just the default configuration. Nothing too complex.
fn get_default_ignore_patterns() -> Vec<String> {
  vec![
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
    "*.pyc".to_string(),
    "*.pyo".to_string(),
    "*.pyd".to_string(),
    "*.pyw".to_string(),
    "*.pyz".to_string(),
    "*.pywz".to_string(),
    "*.pyzw".to_string(),
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
    "*.tasks".to_string(),
  ]
}

// violet ignore chunk
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_matches_pattern_exact() {
    assert!(matches_pattern(".DS_Store", ".DS_Store"));
    assert!(matches_pattern("path/to/.DS_Store", ".DS_Store"));
    assert!(!matches_pattern("other.file", ".DS_Store"));
  }

  #[test]
  fn test_matches_pattern_directory_glob() {
    assert!(matches_pattern("target/", "target/**"));
    assert!(matches_pattern("target/debug", "target/**"));
    assert!(matches_pattern("target/debug/deps/violet", "target/**"));
    assert!(!matches_pattern("src/target", "target/**"));
    assert!(!matches_pattern("other/", "target/**"));
  }

  #[test]
  fn test_matches_pattern_file_extension() {
    assert!(matches_pattern("config.json", "*.json"));
    assert!(matches_pattern("path/to/config.json", "*.json"));
    assert!(matches_pattern("package.json5", "*.json5"));
    assert!(!matches_pattern("config.yaml", "*.json"));
    assert!(!matches_pattern("jsonfile", "*.json"));
  }

  #[test]
  fn test_matches_pattern_wildcard() {
    assert!(matches_pattern("testfile", "test*"));
    assert!(matches_pattern("test123file", "test*file"));
    assert!(matches_pattern("prefix_middle_suffix", "prefix*suffix"));
    assert!(!matches_pattern("wrongprefix_suffix", "prefix*suffix"));
    assert!(!matches_pattern("prefix_wrong", "prefix*suffix"));
  }

  #[test]
  fn test_threshold_for_file() {
    let mut thresholds = HashMap::new();
    thresholds.insert(".rs".to_string(), 8.0);
    thresholds.insert(".js".to_string(), 6.0);

    let config = VioletConfig { thresholds, ignore_patterns: vec![], default_threshold: 7.0 };

    assert_eq!(get_threshold_for_file(&config, "main.rs"), 8.0);
    assert_eq!(get_threshold_for_file(&config, "script.js"), 6.0);
    assert_eq!(get_threshold_for_file(&config, "config.json"), 7.0); // default
    assert_eq!(get_threshold_for_file(&config, "README.md"), 7.0); // default
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
    assert!(should_ignore_file(&config, "target/debug/main"));
    assert!(should_ignore_file(&config, "target/"));
    assert!(!should_ignore_file(&config, "src/target"));

    // File extension patterns
    assert!(should_ignore_file(&config, "package.json"));
    assert!(should_ignore_file(&config, "path/to/config.json"));
    assert!(!should_ignore_file(&config, "config.yaml"));

    // Exact matches
    assert!(should_ignore_file(&config, ".DS_Store"));
    assert!(should_ignore_file(&config, "some/path/.DS_Store"));
    assert!(!should_ignore_file(&config, "DS_Store"));

    // Wildcard patterns
    assert!(should_ignore_file(&config, "testfile"));
    assert!(should_ignore_file(&config, "test123"));
    assert!(!should_ignore_file(&config, "file_test"));
  }

  #[test]
  fn test_should_ignore_normalized_paths() {
    let config = VioletConfig {
      thresholds: HashMap::new(),
      ignore_patterns: vec!["src/main.rs".to_string()],
      default_threshold: 7.0,
    };

    assert!(should_ignore_file(&config, "src/main.rs"));
    assert!(should_ignore_file(&config, "./src/main.rs")); // normalized
  }

  #[test]
  fn test_merge_configs_defaults() {
    let global = ConfigFile {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 8.0, extensions: HashMap::new() },
      },
      ignore: vec!["global_pattern".to_string()],
    };

    let result = merge_configs(global, None);

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

    let result = merge_configs(global, Some(project));

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

    // Project with default threshold same as the global default (6.0)
    let project = ConfigFile {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig {
          default: 6.0, // This is the hardcoded default
          extensions: HashMap::new(),
        },
      },
      ignore: vec![],
    };

    let result = merge_configs(global, Some(project));

    // Should keep global default since project didn't really override it
    assert_eq!(result.default_threshold, 8.0);
  }

  #[test]
  fn test_default_global_config() {
    let config = default_global_config();

    // Should have reasonable defaults
    assert_eq!(config.complexity.thresholds.default, 6.0);

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
    assert_eq!(config.default, 6.0);
    assert!(config.extensions.is_empty());
  }

  #[test]
  fn test_config_file_default() {
    let config = ConfigFile::default();
    assert_eq!(config.complexity.thresholds.default, 6.0); // Updated to match new default
    assert!(config.ignore.is_empty());
  }

  #[test]
  fn test_merge_ignore_patterns_deduplication() {
    let global = vec!["pattern1".to_string(), "pattern2".to_string(), "pattern3".to_string()];
    let project = vec!["pattern2".to_string(), "pattern4".to_string(), "pattern1".to_string()];

    let result = merge_ignore_patterns(global, project);

    assert_eq!(result.len(), 4);
    assert!(result.contains(&"pattern1".to_string()));
    assert!(result.contains(&"pattern2".to_string()));
    assert!(result.contains(&"pattern3".to_string()));
    assert!(result.contains(&"pattern4".to_string()));
  }

  #[test]
  fn test_merge_ignore_patterns_empty() {
    let result = merge_ignore_patterns(vec![], vec![]);
    assert!(result.is_empty());

    let result = merge_ignore_patterns(vec!["pattern".to_string()], vec![]);
    assert_eq!(result, vec!["pattern".to_string()]);

    let result = merge_ignore_patterns(vec![], vec!["pattern".to_string()]);
    assert_eq!(result, vec!["pattern".to_string()]);
  }

  #[test]
  fn test_matches_pattern_edge_cases() {
    // Empty patterns
    assert!(!matches_pattern("file.rs", ""));
    assert!(!matches_pattern("", "pattern"));
    assert!(matches_pattern("", ""));

    // Single wildcards with different content
    assert!(matches_pattern("test123file", "test*file"));
    assert!(matches_pattern("prefix_middle_suffix", "prefix*suffix"));
    assert!(!matches_pattern("wrong_middle_suffix", "prefix*different"));

    // Special characters in paths
    assert!(matches_pattern("file with spaces.txt", "*.txt"));
    assert!(matches_pattern("file-with-dashes.rs", "*.rs"));
    assert!(matches_pattern("file.with.dots.json", "*.json"));

    // Wildcard edge cases
    assert!(matches_pattern("anything", "*"));
    assert!(matches_pattern("prefix123", "prefix*"));
    assert!(matches_pattern("123suffix", "*suffix"));
  }

  #[test]
  fn test_matches_pattern_multiple_wildcards() {
    // Multiple wildcards
    assert!(matches_pattern("test123file456", "test*file*"));
    assert!(matches_pattern("prefix_middle_end_suffix", "prefix*end*suffix"));
    assert!(!matches_pattern("wrong_middle_suffix", "prefix*end*suffix"));

    // Wildcard in middle of text
    assert!(matches_pattern("mydebugfile", "*debug*"));
    assert!(matches_pattern("app_debug_info.txt", "*debug*"));
    assert!(matches_pattern("debug.log", "*debug*"));
    assert!(!matches_pattern("release.log", "*debug*"));

    // Complex multi-wildcard patterns
    assert!(matches_pattern("test_spec_helper.rb", "test*spec*"));
    assert!(matches_pattern("test123spec456", "test*spec*"));
    assert!(!matches_pattern("testfile", "test*spec*"));

    // Edge cases with multiple wildcards
    assert!(matches_pattern("anything", "**"));
    assert!(matches_pattern("anything", "*anything*"));
    assert!(matches_pattern("", "*"));
    assert!(matches_pattern("", "**"));
  }

  #[test]
  fn test_threshold_for_file_edge_cases() {
    let config =
      VioletConfig { thresholds: HashMap::new(), ignore_patterns: vec![], default_threshold: 6.0 };

    // Files without extensions
    assert_eq!(get_threshold_for_file(&config, "README"), 6.0);
    assert_eq!(get_threshold_for_file(&config, "Makefile"), 6.0);

    // Files with multiple extensions
    assert_eq!(get_threshold_for_file(&config, "file.tar.gz"), 6.0);

    // Empty file name
    assert_eq!(get_threshold_for_file(&config, ""), 6.0);
  }

  #[test]
  fn test_should_ignore_complex_patterns() {
    let config = VioletConfig {
      thresholds: HashMap::new(),
      ignore_patterns: vec![
        "test*file".to_string(),
        "build/**".to_string(),
        "temp*.tmp".to_string(),
        "*debug*".to_string(),     // Multi-wildcard pattern
        "*test*spec*".to_string(), // Complex multi-wildcard pattern
      ],
      default_threshold: 6.0,
    };

    // Complex wildcard patterns
    assert!(should_ignore_file(&config, "test123file"));
    assert!(should_ignore_file(&config, "testABCfile"));
    assert!(!should_ignore_file(&config, "filetest"));

    // Directory patterns
    assert!(should_ignore_file(&config, "build/"));
    assert!(should_ignore_file(&config, "build/output"));
    assert!(should_ignore_file(&config, "build/nested/deep"));
    assert!(!should_ignore_file(&config, "src/build")); // doesn't start with "build"

    // Extension patterns
    assert!(should_ignore_file(&config, "temp123.tmp"));
    assert!(should_ignore_file(&config, "tempfile.tmp"));
    assert!(!should_ignore_file(&config, "file.temp"));

    // Multi-wildcard patterns
    assert!(should_ignore_file(&config, "mydebugfile")); // Now works with *debug*
    assert!(should_ignore_file(&config, "debug.log"));
    assert!(should_ignore_file(&config, "app_debug_info.txt"));
    assert!(!should_ignore_file(&config, "release.log"));

    // Complex multi-wildcard patterns
    assert!(should_ignore_file(&config, "test_spec_helper.rb"));
    assert!(should_ignore_file(&config, "unit_test_integration_spec.js"));
    assert!(!should_ignore_file(&config, "regular_file.rb"));
  }

  #[test]
  fn test_default_threshold_value() {
    // Verify the current default is 6.0
    assert_eq!(default_threshold(), 6.0);

    // Verify it's used consistently
    let config = ThresholdConfig::default();
    assert_eq!(config.default, 6.0);

    let global_config = default_global_config();
    assert_eq!(global_config.complexity.thresholds.default, 6.0);
  }

  #[test]
  fn test_default_ignore_patterns_coverage() {
    let patterns = get_default_ignore_patterns();

    // Should have a reasonable number of patterns (not too few, not excessive)
    assert!(patterns.len() > 10);
    assert!(patterns.len() < 50);

    // Should include key directories
    assert!(patterns.iter().any(|p| p.contains("node_modules")));
    assert!(patterns.iter().any(|p| p.contains("target")));
    assert!(patterns.iter().any(|p| p.contains(".git")));

    // Should include binary file types
    assert!(patterns.iter().any(|p| p.contains("*.png")));
    assert!(patterns.iter().any(|p| p.contains("*.pdf")));

    // Should include config/documentation files
    assert!(patterns.iter().any(|p| p.contains("*.md")));
    assert!(patterns.iter().any(|p| p.contains("*.json")));
    assert!(patterns.iter().any(|p| p.contains("*.toml")));
  }
}
