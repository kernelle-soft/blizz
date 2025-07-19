use anyhow::{Context, Result};
use glob::Pattern;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Merged configuration from global defaults and project overrides
#[derive(Debug, Clone, Default)]
pub struct VioletConfig {
  pub thresholds: HashMap<String, f64>,
  pub ignore_patterns: Vec<String>,
  pub ignore_content_patterns: Vec<String>,
  pub default_threshold: f64,
}

impl VioletConfig {
  /// Create a new VioletConfig with default empty patterns
  pub fn new(thresholds: HashMap<String, f64>, ignore_patterns: Vec<String>, default_threshold: f64) -> Self {
    Self {
      thresholds,
      ignore_patterns,
      ignore_content_patterns: vec![],
      default_threshold,
    }
  }

  /// Create a new VioletConfig with all required fields
  pub fn with_content_patterns(
    thresholds: HashMap<String, f64>, 
    ignore_patterns: Vec<String>, 
    ignore_content_patterns: Vec<String>,
    default_threshold: f64
  ) -> Self {
    Self {
      thresholds,
      ignore_patterns,
      ignore_content_patterns,
      default_threshold,
    }
  }
}

/// Raw .violet.json5 file format
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ConfigFile {
  #[serde(default)]
  pub complexity: ComplexityConfig,
  #[serde(default)]
  pub ignore_files: Vec<String>,
  #[serde(default)]
  pub ignore_patterns: Vec<String>,
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

  /// Per-extension thresholds (e.g., ".rs": 7.0)
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

fn default_global_config() -> ConfigFile {
  ConfigFile {
    complexity: ComplexityConfig { thresholds: ThresholdConfig::default() },
    ignore_files: get_default_ignored_files(),
    ignore_patterns: vec![],
  }
}

/// Load and merge global + project configurations
pub fn load_config() -> Result<VioletConfig> {
  let global_config = load_global_config()?;
  let project_config = load_project_config()?;

  Ok(merge_configs(global_config, project_config))
}

pub fn get_threshold_for_file<P: AsRef<Path>>(config: &VioletConfig, file_path: P) -> f64 {
  let path = file_path.as_ref();

  if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
    let ext_key = format!(".{extension}");
    if let Some(&threshold) = config.thresholds.get(&ext_key) {
      return threshold;
    }
  }

  config.default_threshold
}

pub fn should_ignore_file<P: AsRef<Path>>(config: &VioletConfig, file_path: P) -> bool {
  let path_str = file_path.as_ref().to_string_lossy();

  // Handle both "./path" and "path" formats
  let normalized_path =
    if let Some(stripped) = path_str.strip_prefix("./") { stripped } else { &path_str };

  for pattern in &config.ignore_patterns {
    if matches_pattern(&path_str, pattern) || matches_pattern(normalized_path, pattern) {
      return true;
    }
  }
  false
}

fn load_global_config() -> Result<ConfigFile> {
  Ok(default_global_config())
}

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

fn load_config_file(path: &Path) -> Result<ConfigFile> {
  let content = std::fs::read_to_string(path)
    .with_context(|| format!("Failed to read config file: {}", path.display()))?;

  json5::from_str(&content)
    .with_context(|| format!("Failed to parse JSON5 config file: {}", path.display()))
}

/// Merge ignore patterns, removing duplicates
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

fn merge_configs(global: ConfigFile, project: Option<ConfigFile>) -> VioletConfig {
  let project = project.unwrap_or_default();

  // Only use project default if it was explicitly changed
  let default_threshold = if project.complexity.thresholds.default != default_threshold() {
    project.complexity.thresholds.default
  } else {
    global.complexity.thresholds.default
  };

  // Project overrides global for specific extensions
  let mut thresholds = global.complexity.thresholds.extensions.clone();
  for (ext, threshold) in project.complexity.thresholds.extensions {
    thresholds.insert(ext, threshold);
  }

  let ignore_patterns = merge_ignore_patterns(global.ignore_files, project.ignore_files);
  let ignore_content_patterns = merge_ignore_patterns(global.ignore_patterns, project.ignore_patterns);

  VioletConfig { thresholds, ignore_patterns, ignore_content_patterns, default_threshold }
}

/// Enhanced glob matching with filename fallback
fn matches_pattern(path: &str, pattern: &str) -> bool {
  let glob_pattern = match Pattern::new(pattern) {
    Ok(p) => p,
    Err(_) => return false,
  };

  if glob_pattern.matches(path) {
    return true;
  }

  // If pattern has no path separators, try matching as filename anywhere
  if !pattern.contains('/') && !pattern.contains('\\') {
    if let Ok(filename_pattern) = Pattern::new(&format!("*/{pattern}")) {
      if filename_pattern.matches(path) {
        return true;
      }
    }
  }

  false
}

fn get_default_ignored_files() -> Vec<String> {
  vec![
    // Build artifacts and dependencies
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
    // Binary files
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
    // Config and documentation
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

    let config = VioletConfig { thresholds, default_threshold: 7.0, ..Default::default() };

    assert_eq!(get_threshold_for_file(&config, "main.rs"), 8.0);
    assert_eq!(get_threshold_for_file(&config, "script.js"), 6.0);
    assert_eq!(get_threshold_for_file(&config, "config.json"), 7.0);
    assert_eq!(get_threshold_for_file(&config, "README.md"), 7.0);
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
      ..Default::default()
    };

    assert!(should_ignore_file(&config, "target/debug/main"));
    assert!(should_ignore_file(&config, "target/"));
    assert!(!should_ignore_file(&config, "src/target"));

    assert!(should_ignore_file(&config, "package.json"));
    assert!(should_ignore_file(&config, "path/to/config.json"));
    assert!(!should_ignore_file(&config, "config.yaml"));

    assert!(should_ignore_file(&config, ".DS_Store"));
    assert!(should_ignore_file(&config, "some/path/.DS_Store"));
    assert!(!should_ignore_file(&config, "DS_Store"));

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
      ..Default::default()
    };

    assert!(should_ignore_file(&config, "src/main.rs"));
    assert!(should_ignore_file(&config, "./src/main.rs"));
  }

  #[test]
  fn test_merge_configs_defaults() {
    let global = ConfigFile {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 8.0, extensions: HashMap::new() },
      },
      ignore_files: vec!["global_pattern".to_string()],
      ..Default::default()
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
      ignore_files: vec!["global1".to_string(), "global2".to_string()],
      ..Default::default()
    };

    let mut project_thresholds = HashMap::new();
    project_thresholds.insert(".rs".to_string(), 9.0);
    project_thresholds.insert(".py".to_string(), 5.0);

    let project = ConfigFile {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig {
          default: 6.5,
          extensions: project_thresholds,
        },
      },
      ignore_files: vec!["project1".to_string(), "global1".to_string()],
      ..Default::default()
    };

    let result = merge_configs(global, Some(project));

    assert_eq!(result.default_threshold, 6.5);

    assert_eq!(result.thresholds.get(".rs"), Some(&9.0));
    assert_eq!(result.thresholds.get(".js"), Some(&6.0));
    assert_eq!(result.thresholds.get(".py"), Some(&5.0));

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
      ..Default::default()
    };

    let project = ConfigFile {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig {
          default: 6.0,
          extensions: HashMap::new(),
        },
      },
      ..Default::default()
    };

    let result = merge_configs(global, Some(project));

    assert_eq!(result.default_threshold, 8.0);
  }

  #[test]
  fn test_default_global_config() {
    let config = default_global_config();

    assert_eq!(config.complexity.thresholds.default, 6.0);

    assert!(config.ignore_files.contains(&"node_modules/**".to_string()));
    assert!(config.ignore_files.contains(&"target/**".to_string()));
    assert!(config.ignore_files.contains(&".git/**".to_string()));

    assert!(config.ignore_files.contains(&"*.png".to_string()));
    assert!(config.ignore_files.contains(&"*.pdf".to_string()));

    assert!(config.ignore_files.contains(&"*.md".to_string()));
    assert!(config.ignore_files.contains(&"*.json".to_string()));
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
    assert_eq!(config.complexity.thresholds.default, 6.0);
    assert!(config.ignore_files.is_empty());
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
    assert!(!matches_pattern("file.rs", ""));
    assert!(!matches_pattern("", "pattern"));
    assert!(matches_pattern("", ""));

    assert!(matches_pattern("test123file", "test*file"));
    assert!(matches_pattern("prefix_middle_suffix", "prefix*suffix"));
    assert!(!matches_pattern("wrong_middle_suffix", "prefix*different"));

    assert!(matches_pattern("file with spaces.txt", "*.txt"));
    assert!(matches_pattern("file-with-dashes.rs", "*.rs"));
    assert!(matches_pattern("file.with.dots.json", "*.json"));

    assert!(matches_pattern("anything", "*"));
    assert!(matches_pattern("prefix123", "prefix*"));
    assert!(matches_pattern("123suffix", "*suffix"));
  }

  #[test]
  fn test_matches_pattern_multiple_wildcards() {
    assert!(matches_pattern("test123file456", "test*file*"));
    assert!(matches_pattern("prefix_middle_end_suffix", "prefix*end*suffix"));
    assert!(!matches_pattern("wrong_middle_suffix", "prefix*end*suffix"));

    assert!(matches_pattern("mydebugfile", "*debug*"));
    assert!(matches_pattern("app_debug_info.txt", "*debug*"));
    assert!(matches_pattern("debug.log", "*debug*"));
    assert!(!matches_pattern("release.log", "*debug*"));

    assert!(matches_pattern("test_spec_helper.rb", "test*spec*"));
    assert!(matches_pattern("test123spec456", "test*spec*"));
    assert!(!matches_pattern("testfile", "test*spec*"));

    assert!(matches_pattern("anything", "**"));
    assert!(matches_pattern("anything", "*anything*"));
    assert!(matches_pattern("", "*"));
    assert!(matches_pattern("", "**"));
  }

  #[test]
  fn test_threshold_for_file_edge_cases() {
    let config =
      VioletConfig::new(HashMap::new(), vec![], 6.0);

    assert_eq!(get_threshold_for_file(&config, "README"), 6.0);
    assert_eq!(get_threshold_for_file(&config, "Makefile"), 6.0);

    assert_eq!(get_threshold_for_file(&config, "file.tar.gz"), 6.0);

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
        "*debug*".to_string(),
        "*test*spec*".to_string(),
      ],
      default_threshold: 6.0,
      ..Default::default()
    };

    assert!(should_ignore_file(&config, "test123file"));
    assert!(should_ignore_file(&config, "testABCfile"));
    assert!(!should_ignore_file(&config, "filetest"));

    assert!(should_ignore_file(&config, "build/"));
    assert!(should_ignore_file(&config, "build/output"));
    assert!(should_ignore_file(&config, "build/nested/deep"));
    assert!(!should_ignore_file(&config, "src/build"));

    assert!(should_ignore_file(&config, "temp123.tmp"));
    assert!(should_ignore_file(&config, "tempfile.tmp"));
    assert!(!should_ignore_file(&config, "file.temp"));

    assert!(should_ignore_file(&config, "mydebugfile"));
    assert!(should_ignore_file(&config, "debug.log"));
    assert!(should_ignore_file(&config, "app_debug_info.txt"));
    assert!(!should_ignore_file(&config, "release.log"));

    assert!(should_ignore_file(&config, "test_spec_helper.rb"));
    assert!(should_ignore_file(&config, "unit_test_integration_spec.js"));
    assert!(!should_ignore_file(&config, "regular_file.rb"));
  }

  #[test]
  fn test_default_threshold_value() {
    assert_eq!(default_threshold(), 6.0);

    let config = ThresholdConfig::default();
    assert_eq!(config.default, 6.0);

    let global_config = default_global_config();
    assert_eq!(global_config.complexity.thresholds.default, 6.0);
  }

  #[test]
  fn test_default_ignore_patterns_coverage() {
    let patterns = get_default_ignored_files();

    assert!(patterns.len() > 10);
    assert!(patterns.len() < 50);

    assert!(patterns.iter().any(|p| p.contains("node_modules")));
    assert!(patterns.iter().any(|p| p.contains("target")));
    assert!(patterns.iter().any(|p| p.contains(".git")));

    assert!(patterns.iter().any(|p| p.contains("*.png")));
    assert!(patterns.iter().any(|p| p.contains("*.pdf")));

    assert!(patterns.iter().any(|p| p.contains("*.md")));
    assert!(patterns.iter().any(|p| p.contains("*.json")));
    assert!(patterns.iter().any(|p| p.contains("*.toml")));
  }

  #[test]
  fn test_default_global_config_comprehensive() {
    let config = default_global_config();
    
    assert_eq!(config.complexity.thresholds.default, 6.0);
    assert!(config.complexity.thresholds.extensions.is_empty());
    assert!(!config.ignore_files.is_empty());
    
    let has_directories = config.ignore_files.iter().any(|p| p.contains("node_modules") || p.contains("target"));
    let has_binaries = config.ignore_files.iter().any(|p| p.contains("*.png") || p.contains("*.pdf"));
    let has_configs = config.ignore_files.iter().any(|p| p.contains("*.json") || p.contains("*.toml"));
    
    assert!(has_directories);
    assert!(has_binaries);
    assert!(has_configs);
  }

  #[test]
  fn test_load_config_file_json5_parsing() {
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    let mut temp_file = NamedTempFile::new().unwrap();
    let valid_json5 = r#"{
      complexity: {
        thresholds: {
          default: 8.0,
          ".rs": 10.0,
          ".js": 6.0
        }
      },
      ignore_files: [
        "*.test",
        "temp/**"
      ]
    }"#;
    
    temp_file.write_all(valid_json5.as_bytes()).unwrap();
    let result = load_config_file(temp_file.path());
    
    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.complexity.thresholds.default, 8.0);
    assert_eq!(config.complexity.thresholds.extensions.get(".rs"), Some(&10.0));
    assert_eq!(config.complexity.thresholds.extensions.get(".js"), Some(&6.0));
    assert_eq!(config.ignore_files.len(), 2);
    assert!(config.ignore_files.contains(&"*.test".to_string()));
    assert!(config.ignore_files.contains(&"temp/**".to_string()));
  }

  #[test]
  fn test_load_config_file_invalid_json5() {
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    let mut temp_file = NamedTempFile::new().unwrap();
    let invalid_json5 = r#"{
      complexity: {
        thresholds: {
          default: "not_a_number",
        }
      },
      ignore: [
        "*.test"
        "temp/**"
      ]
    }"#;
    
    temp_file.write_all(invalid_json5.as_bytes()).unwrap();
    let result = load_config_file(temp_file.path());
    
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to parse JSON5"));
  }

  #[test]
  fn test_load_config_file_nonexistent() {
    use std::path::Path;
    
    let nonexistent_path = Path::new("/this/path/does/not/exist.json5");
    let result = load_config_file(nonexistent_path);
    
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to read config file"));
  }

  #[test]
  fn test_merge_ignore_patterns_order_preservation() {
    let global = vec!["first".to_string(), "second".to_string()];
    let project = vec!["third".to_string(), "fourth".to_string()];
    
    let result = merge_ignore_patterns(global, project);
    
    assert_eq!(result.len(), 4);
    let first_two: Vec<&String> = result.iter().take(2).collect();
    assert!(first_two.contains(&&"first".to_string()));
    assert!(first_two.contains(&&"second".to_string()));
  }

  #[test]
  fn test_merge_ignore_patterns_complex_deduplication() {
    let global = vec![
      "pattern1".to_string(),
      "pattern2".to_string(),
      "pattern3".to_string(),
      "pattern1".to_string(),
    ];
    let project = vec![
      "pattern2".to_string(),
      "pattern4".to_string(),
      "pattern5".to_string(),
      "pattern4".to_string(),
    ];
    
    let result = merge_ignore_patterns(global, project);
    
    assert_eq!(result.len(), 5);
    assert!(result.contains(&"pattern1".to_string()));
    assert!(result.contains(&"pattern2".to_string()));
    assert!(result.contains(&"pattern3".to_string()));
    assert!(result.contains(&"pattern4".to_string()));
    assert!(result.contains(&"pattern5".to_string()));
  }

  #[test]
  fn test_get_threshold_for_file_various_extensions() {
    let mut thresholds = HashMap::new();
    thresholds.insert(".rs".to_string(), 8.0);
    thresholds.insert(".py".to_string(), 7.0);
    thresholds.insert(".js".to_string(), 6.0);
    thresholds.insert(".java".to_string(), 9.0);
    
    let config = VioletConfig { 
      thresholds, 
      default_threshold: 5.0,
      ..Default::default()
    };
    
    assert_eq!(get_threshold_for_file(&config, "main.rs"), 8.0);
    assert_eq!(get_threshold_for_file(&config, "script.py"), 7.0);
    assert_eq!(get_threshold_for_file(&config, "app.js"), 6.0);
    assert_eq!(get_threshold_for_file(&config, "App.java"), 9.0);
    
    assert_eq!(get_threshold_for_file(&config, "config.toml"), 5.0);
    assert_eq!(get_threshold_for_file(&config, "README.md"), 5.0);
    
    assert_eq!(get_threshold_for_file(&config, "src/main.rs"), 8.0);
    assert_eq!(get_threshold_for_file(&config, "./scripts/build.py"), 7.0);
    assert_eq!(get_threshold_for_file(&config, "/usr/local/bin/tool.unknown"), 5.0);
  }

  #[test]
  fn test_should_ignore_file_complex_scenarios() {
    let config = VioletConfig {
      thresholds: HashMap::new(),
      ignore_patterns: vec![
        "exact_file.txt".to_string(),
        "prefix_*".to_string(),
        "*_suffix.rs".to_string(),
        "dir/**".to_string(),
        "**/*.temp".to_string(),
        "nested/**/deep.json".to_string(),
      ],
      default_threshold: 6.0,
      ..Default::default()
    };
    
    assert!(should_ignore_file(&config, "exact_file.txt"));
    assert!(!should_ignore_file(&config, "exact_file.rs"));
    
    assert!(should_ignore_file(&config, "prefix_anything"));
    assert!(should_ignore_file(&config, "prefix_test.rs"));
    assert!(!should_ignore_file(&config, "test_prefix"));
    
    assert!(should_ignore_file(&config, "test_suffix.rs"));
    assert!(should_ignore_file(&config, "my_suffix.rs"));
    assert!(!should_ignore_file(&config, "suffix_test.rs"));
    
    assert!(should_ignore_file(&config, "dir/file.txt"));
    assert!(should_ignore_file(&config, "dir/subdir/file.txt"));
    assert!(!should_ignore_file(&config, "otherdir/file.txt"));
    
    assert!(should_ignore_file(&config, "any/path/file.temp"));
    assert!(should_ignore_file(&config, "file.temp"));
    assert!(should_ignore_file(&config, "very/deep/path/file.temp"));
    
    assert!(should_ignore_file(&config, "nested/anything/deep.json"));
    assert!(should_ignore_file(&config, "nested/very/long/path/deep.json"));
    assert!(should_ignore_file(&config, "nested/deep.json"));
  }

  #[test]
  fn test_should_ignore_file_path_normalization() {
    let config = VioletConfig {
      thresholds: HashMap::new(),
      ignore_patterns: vec![
        "src/main.rs".to_string(),
        "tests/integration.rs".to_string(),
      ],
      default_threshold: 6.0,
      ..Default::default()
    };
    
    assert!(should_ignore_file(&config, "src/main.rs"));
    assert!(should_ignore_file(&config, "./src/main.rs"));
    assert!(should_ignore_file(&config, "tests/integration.rs"));
    assert!(should_ignore_file(&config, "./tests/integration.rs"));
    
    assert!(!should_ignore_file(&config, "src/lib.rs"));
    assert!(!should_ignore_file(&config, "tests/unit.rs"));
    assert!(!should_ignore_file(&config, "other/main.rs"));
  }

  #[test]
  fn test_matches_pattern_globstar_edge_cases() {
    assert!(matches_pattern("anything/file.txt", "**/file.txt"));
    assert!(matches_pattern("deep/nested/file.txt", "**/file.txt"));
    assert!(matches_pattern("file.txt", "**/file.txt"));
    
    assert!(matches_pattern("src/anything", "src/**"));
    assert!(matches_pattern("src/deep/nested", "src/**"));
    assert!(matches_pattern("src/", "src/**"));
    
    assert!(matches_pattern("start/anything/end", "start/**/end"));
    assert!(matches_pattern("start/deep/nested/end", "start/**/end"));
    assert!(matches_pattern("start/end", "start/**/end"));
    
    assert!(matches_pattern("a/anything/b/anything/c", "a/**/b/**/c"));
    assert!(matches_pattern("a/x/b/y/c", "a/**/b/**/c"));
  }

  #[test]
  fn test_config_file_serde_edge_cases() {
    let minimal = ConfigFile::default();
    assert_eq!(minimal.complexity.thresholds.default, 6.0);
    assert!(minimal.complexity.thresholds.extensions.is_empty());
    assert!(minimal.ignore_files.is_empty());
    
    let threshold_config = ThresholdConfig::default();
    assert_eq!(threshold_config.default, 6.0);
    assert!(threshold_config.extensions.is_empty());
  }

  #[test]
  fn test_violet_config_creation_edge_cases() {
    let empty_config = VioletConfig {
      thresholds: HashMap::new(),
      ignore_patterns: vec![],
      default_threshold: 10.0,
      ..Default::default()
    };
    
    assert_eq!(empty_config.default_threshold, 10.0);
    assert!(empty_config.thresholds.is_empty());
    assert!(empty_config.ignore_patterns.is_empty());
    
    let mut many_thresholds = HashMap::new();
    for i in 1..20 {
      many_thresholds.insert(format!(".ext{}", i), i as f64);
    }
    
    let large_config = VioletConfig {
      thresholds: many_thresholds.clone(),
      ignore_patterns: vec!["pattern".to_string(); 100],
      default_threshold: 15.0,
      ..Default::default()
    };
    
    assert_eq!(large_config.thresholds.len(), 19);
    assert_eq!(large_config.ignore_patterns.len(), 100);
    assert_eq!(large_config.default_threshold, 15.0);
  }
}
