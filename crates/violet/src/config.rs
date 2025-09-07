use anyhow::{Context, Result};
use glob::Pattern;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Configuration file format
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct VioletConfig {
  #[serde(default)]
  pub complexity: ComplexityConfig,
  #[serde(default)]
  pub ignore_files: Vec<String>,
  #[serde(default)]
  pub ignore_patterns: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ComplexityConfig {
  #[serde(default)]
  pub thresholds: ThresholdConfig,
  #[serde(default)]
  pub penalties: PenaltyConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ThresholdConfig {
  #[serde(default = "default_threshold")]
  pub default: f64,

  /// Per-extension thresholds (e.g., ".rs": 7.0)
  #[serde(flatten)]
  pub extensions: HashMap<String, f64>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PenaltyConfig {
  #[serde(default = "default_depth_penalty")]
  pub depth: f64,
  #[serde(default = "default_verbosity_penalty")]
  pub verbosity: f64,
  #[serde(default = "default_syntactics_penalty")]
  pub syntactics: f64,
}

impl Default for PenaltyConfig {
  fn default() -> Self {
    Self {
      depth: default_depth_penalty(),
      verbosity: default_verbosity_penalty(),
      syntactics: default_syntactics_penalty(),
    }
  }
}

impl Default for ThresholdConfig {
  fn default() -> Self {
    Self { default: default_threshold(), extensions: HashMap::new() }
  }
}

fn default_threshold() -> f64 {
  8.0
}

fn default_depth_penalty() -> f64 {
  std::f64::consts::E
}

fn default_verbosity_penalty() -> f64 {
  1.025
}

fn default_syntactics_penalty() -> f64 {
  1.15
}

fn default_global_config() -> VioletConfig {
  VioletConfig {
    complexity: ComplexityConfig {
      thresholds: ThresholdConfig::default(),
      penalties: PenaltyConfig::default(),
    },
    ignore_files: get_default_ignored_files(),
    ignore_patterns: vec![],
  }
}

/// Load and merge global + project configurations
pub fn load_config() -> Result<VioletConfig> {
  let global_config = default_global_config();
  let project_config = load_project_config()?;

  Ok(merge(global_config, project_config))
}

/// Get the threshold for a file based on its extension
pub fn get_threshold<P: AsRef<Path>>(config: &VioletConfig, file_path: P) -> f64 {
  let path = file_path.as_ref();

  if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
    let ext_key = format!(".{extension}");
    if let Some(&threshold) = config.complexity.thresholds.extensions.get(&ext_key) {
      return threshold;
    }
  }

  config.complexity.thresholds.default
}

pub fn should_ignore_file<P: AsRef<Path>>(config: &VioletConfig, file_path: P) -> bool {
  let path_str = file_path.as_ref().to_string_lossy();

  // Handle both "./path" and "path" formats
  let normalized_path =
    if let Some(stripped) = path_str.strip_prefix("./") { stripped } else { &path_str };

  for pattern in config.ignore_files.iter() {
    if matches_glob(&path_str, pattern) || matches_glob(normalized_path, pattern) {
      return true;
    }
  }
  false
}

fn load_project_config() -> Result<Option<VioletConfig>> {
  let current_dir = std::env::current_dir().context("Failed to get current working directory")?;

  let project_config_path = current_dir.join("violet.yaml");

  if project_config_path.exists() {
    let config = load_config_file(&project_config_path).with_context(|| {
      format!("Failed to load project config from {}", project_config_path.display())
    })?;
    Ok(Some(config))
  } else {
    Ok(None)
  }
}

fn load_config_file(path: &Path) -> Result<VioletConfig> {
  let content = std::fs::read_to_string(path)
    .with_context(|| format!("Failed to read config file: {}", path.display()))?;

  serde_yaml::from_str(&content)
    .with_context(|| format!("Failed to parse YAML config file: {}", path.display()))
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

fn merge(global: VioletConfig, project: Option<VioletConfig>) -> VioletConfig {
  let project = project.unwrap_or_default();

  let merged_thresholds = merge_threshold_configs(&global, &project);
  let merged_penalties = merge_penalty_configs(&global, &project);
  let merged_ignores = merge_ignore_configs(&global, &project);

  build_merged_config(merged_thresholds, merged_penalties, merged_ignores)
}

fn merge_threshold_configs(global: &VioletConfig, project: &VioletConfig) -> ThresholdConfig {
  let default_threshold = determine_default_threshold(global, project);
  let extensions = merge_extension_thresholds(global, project);

  ThresholdConfig { default: default_threshold, extensions }
}

fn determine_default_threshold(global: &VioletConfig, project: &VioletConfig) -> f64 {
  if project.complexity.thresholds.default != default_threshold() {
    project.complexity.thresholds.default
  } else {
    global.complexity.thresholds.default
  }
}

fn merge_extension_thresholds(
  global: &VioletConfig,
  project: &VioletConfig,
) -> HashMap<String, f64> {
  let mut thresholds = global.complexity.thresholds.extensions.clone();
  for (ext, threshold) in &project.complexity.thresholds.extensions {
    thresholds.insert(ext.clone(), *threshold);
  }
  thresholds
}

fn merge_penalty_configs(global: &VioletConfig, project: &VioletConfig) -> PenaltyConfig {
  PenaltyConfig {
    depth: if project.complexity.penalties.depth != default_depth_penalty() {
      project.complexity.penalties.depth
    } else {
      global.complexity.penalties.depth
    },
    verbosity: if project.complexity.penalties.verbosity != default_verbosity_penalty() {
      project.complexity.penalties.verbosity
    } else {
      global.complexity.penalties.verbosity
    },
    syntactics: if project.complexity.penalties.syntactics != default_syntactics_penalty() {
      project.complexity.penalties.syntactics
    } else {
      global.complexity.penalties.syntactics
    },
  }
}

fn merge_ignore_configs(
  global: &VioletConfig,
  project: &VioletConfig,
) -> (Vec<String>, Vec<String>) {
  let ignore_files =
    merge_ignore_patterns(global.ignore_files.clone(), project.ignore_files.clone());
  let ignore_patterns =
    merge_ignore_patterns(global.ignore_patterns.clone(), project.ignore_patterns.clone());
  (ignore_files, ignore_patterns)
}

fn build_merged_config(
  thresholds: ThresholdConfig,
  penalties: PenaltyConfig,
  (ignore_files, ignore_patterns): (Vec<String>, Vec<String>),
) -> VioletConfig {
  VioletConfig {
    complexity: ComplexityConfig { thresholds, penalties },
    ignore_files,
    ignore_patterns,
  }
}

/// Enhanced glob matching with filename fallback
fn matches_glob(path: &str, pattern: &str) -> bool {
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

// violet ignore chunk -- just a list of files that are ignored by default
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
    "*.ttf".to_string(),
    "*.woff".to_string(),
    "*.woff2".to_string(),
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
    "*.profraw".to_string(),
    "*.ico".to_string(),
    "*.svg".to_string(),
    "*.webp".to_string(),
    "*.avif".to_string(),
    "*.heic".to_string(),
    "*.heif".to_string(),
    "*.mp4".to_string(),
    "*.mov".to_string(),
    "*.avi".to_string(),
    "*.wmv".to_string(),
    "*.mkv".to_string(),
    "*.flv".to_string(),
    "*.mpeg".to_string(),
    "*.mpg".to_string(),
    "*.m4v".to_string(),
    "*.m4a".to_string(),
    "*.mp3".to_string(),
    "*.wav".to_string(),
    "*.ogg".to_string(),
    "*.flac".to_string(),
    "*.aac".to_string(),
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
    assert!(matches_glob(".DS_Store", ".DS_Store"));
    assert!(matches_glob("path/to/.DS_Store", ".DS_Store"));
    assert!(!matches_glob("other.file", ".DS_Store"));
  }

  #[test]
  fn test_matches_pattern_directory_glob() {
    assert!(matches_glob("target/", "target/**"));
    assert!(matches_glob("target/debug", "target/**"));
    assert!(matches_glob("target/debug/deps/violet", "target/**"));
    assert!(!matches_glob("src/target", "target/**"));
    assert!(!matches_glob("other/", "target/**"));
  }

  #[test]
  fn test_matches_pattern_file_extension() {
    assert!(matches_glob("config.json", "*.json"));
    assert!(matches_glob("path/to/config.json", "*.json"));
    assert!(matches_glob("package.json5", "*.json5"));
    assert!(!matches_glob("config.yaml", "*.json"));
    assert!(!matches_glob("jsonfile", "*.json"));
  }

  #[test]
  fn test_matches_pattern_wildcard() {
    assert!(matches_glob("testfile", "test*"));
    assert!(matches_glob("test123file", "test*file"));
    assert!(matches_glob("prefix_middle_suffix", "prefix*suffix"));
    assert!(!matches_glob("wrongprefix_suffix", "prefix*suffix"));
    assert!(!matches_glob("prefix_wrong", "prefix*suffix"));
  }

  #[test]
  fn test_threshold_for_file() {
    let mut thresholds = HashMap::new();
    thresholds.insert(".rs".to_string(), 8.0);
    thresholds.insert(".js".to_string(), 6.0);

    let config = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 7.0, extensions: thresholds },
        penalties: PenaltyConfig::default(),
      },
      ..Default::default()
    };

    assert_eq!(get_threshold(&config, "main.rs"), 8.0);
    assert_eq!(get_threshold(&config, "script.js"), 6.0);
    assert_eq!(get_threshold(&config, "config.json"), 7.0);
    assert_eq!(get_threshold(&config, "README.md"), 7.0);
  }

  #[test]
  fn test_should_ignore() {
    let config = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 7.0, extensions: HashMap::new() },
        penalties: PenaltyConfig::default(),
      },
      ignore_files: vec![
        "target/**".to_string(),
        "*.json".to_string(),
        ".DS_Store".to_string(),
        "test*".to_string(),
      ],
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
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 7.0, extensions: HashMap::new() },
        penalties: PenaltyConfig::default(),
      },
      ignore_files: vec!["src/main.rs".to_string()],
      ..Default::default()
    };

    assert!(should_ignore_file(&config, "src/main.rs"));
    assert!(should_ignore_file(&config, "./src/main.rs"));
  }

  #[test]
  fn test_merge_configs_defaults() {
    let global = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 8.0, extensions: HashMap::new() },
        penalties: PenaltyConfig::default(),
      },
      ignore_files: vec!["global_pattern".to_string()],
      ..Default::default()
    };

    let result = merge(global, None);

    assert_eq!(result.complexity.thresholds.default, 8.0);
    assert_eq!(result.ignore_files, vec!["global_pattern"]);
  }

  #[test]
  fn test_merge_configs_project_overrides() {
    let mut global_thresholds = HashMap::new();
    global_thresholds.insert(".rs".to_string(), 8.0);
    global_thresholds.insert(".js".to_string(), 6.0);

    let global = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 7.0, extensions: global_thresholds },
        penalties: PenaltyConfig::default(),
      },
      ignore_files: vec!["global1".to_string(), "global2".to_string()],
      ..Default::default()
    };

    let mut project_thresholds = HashMap::new();
    project_thresholds.insert(".rs".to_string(), 9.0);
    project_thresholds.insert(".py".to_string(), 5.0);

    let project = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 6.5, extensions: project_thresholds },
        penalties: PenaltyConfig::default(),
      },
      ignore_files: vec!["project1".to_string(), "global1".to_string()],
      ..Default::default()
    };

    let result = merge(global, Some(project));

    assert_eq!(result.complexity.thresholds.default, 6.5);

    assert_eq!(result.complexity.thresholds.extensions.get(".rs"), Some(&9.0));
    assert_eq!(result.complexity.thresholds.extensions.get(".js"), Some(&6.0));
    assert_eq!(result.complexity.thresholds.extensions.get(".py"), Some(&5.0));

    assert_eq!(result.ignore_files.len(), 3);
    assert!(result.ignore_files.contains(&"global1".to_string()));
    assert!(result.ignore_files.contains(&"global2".to_string()));
    assert!(result.ignore_files.contains(&"project1".to_string()));
  }

  #[test]
  fn test_merge_configs_project_default_not_changed() {
    let global = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 8.0, extensions: HashMap::new() },
        penalties: PenaltyConfig::default(),
      },
      ..Default::default()
    };

    let project = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 6.0, extensions: HashMap::new() },
        penalties: PenaltyConfig::default(),
      },
      ..Default::default()
    };

    let result = merge(global, Some(project));

    assert_eq!(result.complexity.thresholds.default, 6.0);
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
    assert!(!matches_glob("file.rs", ""));
    assert!(!matches_glob("", "pattern"));
    assert!(matches_glob("", ""));

    assert!(matches_glob("test123file", "test*file"));
    assert!(matches_glob("prefix_middle_suffix", "prefix*suffix"));
    assert!(!matches_glob("wrong_middle_suffix", "prefix*different"));

    assert!(matches_glob("file with spaces.txt", "*.txt"));
    assert!(matches_glob("file-with-dashes.rs", "*.rs"));
    assert!(matches_glob("file.with.dots.json", "*.json"));

    assert!(matches_glob("anything", "*"));
    assert!(matches_glob("prefix123", "prefix*"));
    assert!(matches_glob("123suffix", "*suffix"));
  }

  #[test]
  fn test_matches_pattern_multiple_wildcards() {
    assert!(matches_glob("test123file456", "test*file*"));
    assert!(matches_glob("prefix_middle_end_suffix", "prefix*end*suffix"));
    assert!(!matches_glob("wrong_middle_suffix", "prefix*end*suffix"));

    assert!(matches_glob("mydebugfile", "*debug*"));
    assert!(matches_glob("app_debug_info.txt", "*debug*"));
    assert!(matches_glob("debug.log", "*debug*"));
    assert!(!matches_glob("release.log", "*debug*"));

    assert!(matches_glob("test_spec_helper.rb", "test*spec*"));
    assert!(matches_glob("test123spec456", "test*spec*"));
    assert!(!matches_glob("testfile", "test*spec*"));

    assert!(matches_glob("anything", "**"));
    assert!(matches_glob("anything", "*anything*"));
    assert!(matches_glob("", "*"));
    assert!(matches_glob("", "**"));
  }

  #[test]
  fn test_threshold_for_file_edge_cases() {
    let config = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 6.0, extensions: HashMap::new() },
        penalties: PenaltyConfig::default(),
      },
      ..Default::default()
    };

    assert_eq!(get_threshold(&config, "README"), 6.0);
    assert_eq!(get_threshold(&config, "Makefile"), 6.0);

    assert_eq!(get_threshold(&config, "file.tar.gz"), 6.0);

    assert_eq!(get_threshold(&config, ""), 6.0);
  }

  #[test]
  fn test_should_ignore_complex_patterns() {
    let config = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 6.0, extensions: HashMap::new() },
        penalties: PenaltyConfig::default(),
      },
      ignore_files: vec![
        "test*file".to_string(),
        "build/**".to_string(),
        "temp*.tmp".to_string(),
        "*debug*".to_string(),
        "*test*spec*".to_string(),
      ],
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
  fn test_should_ignore_font_files() {
    let config = default_global_config();

    // Test various font file types should be ignored
    assert!(should_ignore_file(&config, "fonts/MyFont.ttf"));
    assert!(should_ignore_file(&config, "assets/font.woff"));
    assert!(should_ignore_file(&config, "styles/icons.woff2"));
    assert!(should_ignore_file(&config, "MyFont.ttf"));
    assert!(should_ignore_file(&config, "font.woff"));
    assert!(should_ignore_file(&config, "icons.woff2"));

    // Test that other files are not ignored
    assert!(!should_ignore_file(&config, "main.rs"));
    assert!(!should_ignore_file(&config, "config.js"));
    assert!(!should_ignore_file(&config, "font_loader.py"));
  }

  #[test]
  fn test_load_config_file_yaml_parsing() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut temp_file = NamedTempFile::new().unwrap();
    let valid_yaml = r#"complexity:
  thresholds:
    default: 8.0
    ".rs": 10.0
    ".js": 6.0
ignore_files:
  - "*.test"
  - "temp/**"
"#;

    temp_file.write_all(valid_yaml.as_bytes()).unwrap();
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
  fn test_load_config_file_invalid_yaml() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut temp_file = NamedTempFile::new().unwrap();
    let invalid_yaml = r#"complexity:
  thresholds:
    default: "not_a_number"
ignore_files:
  - "*.test"
  - missing_dash_here
    "temp/**"
"#;

    temp_file.write_all(invalid_yaml.as_bytes()).unwrap();
    let result = load_config_file(temp_file.path());

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to parse YAML"));
  }

  #[test]
  fn test_load_config_file_nonexistent() {
    use std::path::Path;

    let nonexistent_path = Path::new("/this/path/does/not/exist.yaml");
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
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 5.0, extensions: thresholds },
        penalties: PenaltyConfig::default(),
      },
      ..Default::default()
    };

    assert_eq!(get_threshold(&config, "main.rs"), 8.0);
    assert_eq!(get_threshold(&config, "script.py"), 7.0);
    assert_eq!(get_threshold(&config, "app.js"), 6.0);
    assert_eq!(get_threshold(&config, "App.java"), 9.0);

    assert_eq!(get_threshold(&config, "config.toml"), 5.0);
    assert_eq!(get_threshold(&config, "README.md"), 5.0);

    assert_eq!(get_threshold(&config, "src/main.rs"), 8.0);
    assert_eq!(get_threshold(&config, "./scripts/build.py"), 7.0);
    assert_eq!(get_threshold(&config, "/usr/local/bin/tool.unknown"), 5.0);
  }

  #[test]
  fn test_should_ignore_file_complex_scenarios() {
    let config = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 6.0, extensions: HashMap::new() },
        penalties: PenaltyConfig::default(),
      },
      ignore_files: vec![
        "exact_file.txt".to_string(),
        "prefix_*".to_string(),
        "*_suffix.rs".to_string(),
        "dir/**".to_string(),
        "**/*.temp".to_string(),
        "nested/**/deep.json".to_string(),
      ],
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
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 6.0, extensions: HashMap::new() },
        penalties: PenaltyConfig::default(),
      },
      ignore_files: vec!["src/main.rs".to_string(), "tests/integration.rs".to_string()],
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
    assert!(matches_glob("anything/file.txt", "**/file.txt"));
    assert!(matches_glob("deep/nested/file.txt", "**/file.txt"));
    assert!(matches_glob("file.txt", "**/file.txt"));

    assert!(matches_glob("src/anything", "src/**"));
    assert!(matches_glob("src/deep/nested", "src/**"));
    assert!(matches_glob("src/", "src/**"));

    assert!(matches_glob("start/anything/end", "start/**/end"));
    assert!(matches_glob("start/deep/nested/end", "start/**/end"));
    assert!(matches_glob("start/end", "start/**/end"));

    assert!(matches_glob("a/anything/b/anything/c", "a/**/b/**/c"));
    assert!(matches_glob("a/x/b/y/c", "a/**/b/**/c"));
  }

  #[test]
  fn test_violet_config_creation_edge_cases() {
    let empty_config = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 10.0, extensions: HashMap::new() },
        penalties: PenaltyConfig::default(),
      },
      ignore_files: vec![],
      ..Default::default()
    };

    assert_eq!(empty_config.complexity.thresholds.default, 10.0);
    assert!(empty_config.complexity.thresholds.extensions.is_empty());
    assert!(empty_config.ignore_files.is_empty());

    let mut many_thresholds = HashMap::new();
    for i in 1..20 {
      many_thresholds.insert(format!(".ext{i}"), i as f64);
    }

    let large_config = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 15.0, extensions: many_thresholds.clone() },
        penalties: PenaltyConfig::default(),
      },
      ignore_files: vec!["pattern".to_string(); 100],
      ..Default::default()
    };

    assert_eq!(large_config.complexity.thresholds.extensions.len(), 19);
    assert_eq!(large_config.ignore_files.len(), 100);
    assert_eq!(large_config.complexity.thresholds.default, 15.0);
  }

  #[test]
  fn test_merge_penalty_configs_global_wins() {
    let global = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig::default(),
        penalties: PenaltyConfig { depth: 3.0, verbosity: 1.10, syntactics: 1.20 },
      },
      ..Default::default()
    };

    let project = VioletConfig::default(); // Uses default penalties

    let result = merge(global, Some(project));

    // Global should win when project uses defaults
    assert_eq!(result.complexity.penalties.depth, 3.0);
    assert_eq!(result.complexity.penalties.verbosity, 1.10);
    assert_eq!(result.complexity.penalties.syntactics, 1.20);
  }

  #[test]
  fn test_merge_penalty_configs_project_overrides() {
    let global = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig::default(),
        penalties: PenaltyConfig { depth: 3.0, verbosity: 1.10, syntactics: 1.20 },
      },
      ..Default::default()
    };

    let project = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig::default(),
        penalties: PenaltyConfig {
          depth: 4.0,       // Override
          verbosity: 1.05,  // Back to default (should use global)
          syntactics: 1.30, // Override
        },
      },
      ..Default::default()
    };

    let result = merge(global, Some(project));

    assert_eq!(result.complexity.penalties.depth, 4.0); // Project override
    assert_eq!(result.complexity.penalties.verbosity, 1.05); // Project override
    assert_eq!(result.complexity.penalties.syntactics, 1.30); // Project override
  }

  #[test]
  fn test_penalty_config_creation() {
    let penalty_config = PenaltyConfig { depth: 2.5, verbosity: 1.08, syntactics: 1.22 };

    assert_eq!(penalty_config.depth, 2.5);
    assert_eq!(penalty_config.verbosity, 1.08);
    assert_eq!(penalty_config.syntactics, 1.22);
  }

  #[test]
  fn test_full_config_with_custom_penalties() {
    let mut extensions = HashMap::new();
    extensions.insert(".rs".to_string(), 8.0);

    let config = VioletConfig {
      complexity: ComplexityConfig {
        thresholds: ThresholdConfig { default: 7.0, extensions },
        penalties: PenaltyConfig { depth: 3.0, verbosity: 1.10, syntactics: 1.25 },
      },
      ignore_files: vec!["*.test".to_string()],
      ..Default::default()
    };

    assert_eq!(config.complexity.thresholds.default, 7.0);
    assert_eq!(config.complexity.thresholds.extensions.get(".rs"), Some(&8.0));
    assert_eq!(config.complexity.penalties.depth, 3.0);
    assert_eq!(config.complexity.penalties.verbosity, 1.10);
    assert_eq!(config.complexity.penalties.syntactics, 1.25);
    assert_eq!(config.ignore_files.len(), 1);
  }

  #[test]
  fn test_load_config_file_with_penalties() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut temp_file = NamedTempFile::new().unwrap();
    let config_with_penalties = r#"{
      complexity: {
        thresholds: {
          default: 8.0,
          ".rs": 10.0,
          ".js": 6.0
        },
        penalties: {
          depth: 2.5,
          verbosity: 1.08,
          syntactics: 1.22
        }
      },
      ignore_files: [
        "*.test",
        "temp/**"
      ]
    }"#;

    temp_file.write_all(config_with_penalties.as_bytes()).unwrap();
    let result = load_config_file(temp_file.path());

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.complexity.thresholds.default, 8.0);
    assert_eq!(config.complexity.thresholds.extensions.get(".rs"), Some(&10.0));
    assert_eq!(config.complexity.thresholds.extensions.get(".js"), Some(&6.0));
    assert_eq!(config.complexity.penalties.depth, 2.5);
    assert_eq!(config.complexity.penalties.verbosity, 1.08);
    assert_eq!(config.complexity.penalties.syntactics, 1.22);
    assert_eq!(config.ignore_files.len(), 2);
    assert!(config.ignore_files.contains(&"*.test".to_string()));
    assert!(config.ignore_files.contains(&"temp/**".to_string()));
  }

  #[test]
  fn test_load_config_file_with_partial_penalties() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut temp_file = NamedTempFile::new().unwrap();
    let config_with_partial_penalties = r#"complexity:
  penalties:
    depth: 3.0
    # verbosity and syntactics should use defaults
"#;

    temp_file.write_all(config_with_partial_penalties.as_bytes()).unwrap();
    let result = load_config_file(temp_file.path());

    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.complexity.penalties.depth, 3.0);
    assert_eq!(config.complexity.penalties.verbosity, 1.025); // Default
    assert_eq!(config.complexity.penalties.syntactics, 1.15); // Default
  }
}
