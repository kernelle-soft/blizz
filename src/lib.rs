// CLI functionality moved from main.rs for better testability
use std::path::Path;
use std::fs;
use walkdir::WalkDir;
use clap::{Parser, ValueEnum};

/// Violet - Code Complexity Artisan 🎨
/// 
/// Every line of code should be a masterpiece.
#[derive(Parser)]
#[command(name = \"violet\")]
#[command(about = \"A code complexity analyzer that enforces functional programming principles\")]
#[command(version)]
pub struct Cli {
    /// Files or directories to analyze
    pub paths: Vec<String>,

    /// Output format
    #[arg(short, long, default_value = \"pretty\")]
    pub format: OutputFormat,

    /// Output violations as JSON (deprecated, use --format json)
    #[arg(long)]
    pub json: bool,

    /// Output violations in compact format (deprecated, use --format compact)  
    #[arg(long)]
    pub compact: bool,

    /// Configuration file path
    #[arg(short, long)]
    pub config: Option<String>,
}

#[derive(Debug, Clone, ValueEnum, PartialEq)]
pub enum OutputFormat {
    Pretty,
    Json,
    Compact,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Pretty
    }
}

impl From<&Cli> for OutputFormat {
    fn from(cli: &Cli) -> Self {
        if cli.json {
            OutputFormat::Json
        } else if cli.compact {
            OutputFormat::Compact
        } else {
            cli.format.clone()
        }
    }
}

pub fn collect_files(paths: &[String]) -> Result<Vec<String>> {
    let mut files = Vec::new();
    
    for path in paths {
        let path = Path::new(path);
        if !path.exists() {
            return Err(VioletError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(\"Path does not exist: {}\", path.display())
            )));
        }
        
        if path.is_file() {
            if is_supported_file(path) {
                files.push(path.to_string_lossy().to_string());
            }
        } else if path.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                let entry_path = entry.path();
                if entry_path.is_file() && is_supported_file(entry_path) {
                    files.push(entry_path.to_string_lossy().to_string());
                }
            }
        }
    }
    
    Ok(files)
}

pub fn is_supported_file(path: &Path) -> bool {
    if let Some(extension) = path.extension() {
        if let Some(ext_str) = extension.to_str() {
            Language::from_extension(ext_str).is_some()
        } else {
            false
        }
    } else {
        false
    }
}

pub fn process_files(file_paths: &[String], config: &config::Config) -> Result<Vec<Violation>> {
    use crate::{parser::LanguageParser, linter::Linter};
    
    let mut all_violations = Vec::new();
    
    for file_path in file_paths {
        let path = Path::new(file_path);
        
        if config.should_ignore_file(path) {
            continue;
        }
        
        let content = fs::read_to_string(path)
            .map_err(VioletError::Io)?;
            
        if let Some(language) = crate::parser::detect_language(path) {
            let mut parser = LanguageParser::new(language)?;
            let tree = parser.parse(&content)?;
            
            let linter = Linter::new(config.clone());
            let violations = linter.lint_file(&tree, &content, language, path)?;
            
            all_violations.extend(violations);
        }
    }
    
    Ok(all_violations)
}

pub fn format_violations(violations: &[Violation], format: &OutputFormat) -> String {
    match format {
        OutputFormat::Pretty => format_pretty(violations),
        OutputFormat::Json => format_json(violations),
        OutputFormat::Compact => format_compact(violations),
    }
}

fn format_pretty(violations: &[Violation]) -> String {
    use colored::Colorize;
    
    if violations.is_empty() {
        return format!(\"{}\\n\\n{}\", 
            \"✨ Beautiful! No violations found.\".bright_green().bold(),
            \"Your code is a masterpiece! 🎨\".bright_cyan()
        );
    }
    
    let mut output = String::new();
    
    // Group violations by file
    let mut by_file: std::collections::BTreeMap<&Path, Vec<&Violation>> = std::collections::BTreeMap::new();
    for violation in violations {
        by_file.entry(&violation.file).or_default().push(violation);
    }
    
    for (file, file_violations) in by_file {
        output.push_str(&format!(\"\\n{}\\n\", file.display().to_string().bright_white().bold()));
        
        for violation in file_violations {
            let severity_icon = match violation.severity {
                Severity::Error => \"🚨 Error\".bright_red(),
                Severity::Warning => \"⚠️  Warning\".bright_yellow(), 
                Severity::Info => \"ℹ️  Info\".bright_blue(),
            };
            
            output.push_str(&format!(
                \"  {} {}:{}:{} {} {}\\n\",
                severity_icon,
                file.display(),
                violation.line,
                violation.column,
                violation.rule.bright_cyan(),
                violation.message.white()
            ));
        }
    }
    
    output.push_str(&format!(\"\\n{} {} violations found\\n\", 
        \"🎯\".bright_magenta(),
        violations.len().to_string().bright_white().bold()
    ));
    
    output
}

fn format_json(violations: &[Violation]) -> String {
    let output = serde_json::json!({
        \"violations\": violations,
        \"summary\": {
            \"total\": violations.len(),
            \"by_severity\": {
                \"error\": violations.iter().filter(|v| v.severity == Severity::Error).count(),
                \"warning\": violations.iter().filter(|v| v.severity == Severity::Warning).count(),
                \"info\": violations.iter().filter(|v| v.severity == Severity::Info).count(),
            }
        }
    });
    
    serde_json::to_string_pretty(&output).unwrap_or_else(|_| \"{}\".to_string())
}

fn format_compact(violations: &[Violation]) -> String {
    if violations.is_empty() {
        return String::new();
    }
    
    violations.iter()
        .map(|v| format!(\"{}:{}:{} {} {}\", 
            v.file.display(), v.line, v.column, v.rule, v.message))
        .collect::<Vec<_>>()
        .join(\"\\n\")
}

pub fn get_exit_code(violations: &[Violation]) -> i32 {
    if violations.is_empty() { 0 } else { 1 }
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_cli_parsing() {
    let cli = Cli::try_parse_from(&[\"violet\", \"src/\"]).unwrap();
    assert_eq!(cli.paths, vec![\"src/\"]);
    assert_eq!(cli.format, OutputFormat::Pretty);
    assert!(!cli.json);
    assert!(!cli.compact);
}

#[test]
fn test_output_format_from_flags() {
    let cli = Cli::try_parse_from(&[\"violet\", \"--json\", \"src/\"]).unwrap();
    assert_eq!(OutputFormat::from(&cli), OutputFormat::Json);

    let cli = Cli::try_parse_from(&[\"violet\", \"--compact\", \"src/\"]).unwrap();
    assert_eq!(OutputFormat::from(&cli), OutputFormat::Compact);

    let cli = Cli::try_parse_from(&[\"violet\", \"src/\"]).unwrap();
    assert_eq!(OutputFormat::from(&cli), OutputFormat::Pretty);
}

#[test]
fn test_format_violations_pretty() {
    let violations = vec![
        Violation {
            file: \"test.js\".into(),
            line: 1,
            column: 10,
            rule: \"max-params\".to_string(),
            message: \"Too many parameters\".to_string(),
            severity: Severity::Error,
        },
        Violation {
            file: \"test.js\".into(),
            line: 5,
            column: 15,
            rule: \"function-length\".to_string(),
            message: \"Function too long\".to_string(),
            severity: Severity::Warning,
        },
    ];

    let output = format_violations(&violations, &OutputFormat::Pretty);
    assert!(output.contains(\"🚨 Error\"));
    assert!(output.contains(\"⚠️  Warning\"));
    assert!(output.contains(\"test.js:1:10\"));
    assert!(output.contains(\"test.js:5:15\"));
    assert!(output.contains(\"max-params\"));
    assert!(output.contains(\"function-length\"));
}

#[test]
fn test_format_violations_json() {
    let violations = vec![
        Violation {
            file: \"test.js\".into(),
            line: 1,
            column: 10,
            rule: \"max-params\".to_string(),
            message: \"Too many parameters\".to_string(),
            severity: Severity::Error,
        },
    ];

    let output = format_violations(&violations, &OutputFormat::Json);
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    
    assert_eq!(parsed[\"violations\"].as_array().unwrap().len(), 1);
    let violation = &parsed[\"violations\"][0];
    assert_eq!(violation[\"file\"], \"test.js\");
    assert_eq!(violation[\"line\"], 1);
    assert_eq!(violation[\"rule\"], \"max-params\");
}

#[test]
fn test_format_violations_compact() {
    let violations = vec![
        Violation {
            file: \"test.js\".into(),
            line: 1,
            column: 10,
            rule: \"max-params\".to_string(),
            message: \"Too many parameters\".to_string(),
            severity: Severity::Error,
        },
    ];

    let output = format_violations(&violations, &OutputFormat::Compact);
    assert!(output.contains(\"test.js:1:10\"));
    assert!(output.contains(\"max-params\"));
    assert!(!output.contains(\"🚨\")); // No emojis in compact format
}

#[test]
fn test_format_violations_empty() {
    let violations = vec![];
    
    let pretty = format_violations(&violations, &OutputFormat::Pretty);
    assert!(pretty.contains(\"✨\"));
    assert!(pretty.contains(\"No violations\"));

    let json = format_violations(&violations, &OutputFormat::Json);
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed[\"violations\"].as_array().unwrap().len(), 0);

    let compact = format_violations(&violations, &OutputFormat::Compact);
    assert!(compact.is_empty());
}

    #[test]
    fn test_collect_files() {
        let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create test files
    fs::write(temp_path.join(\"test.js\"), \"function test() {}\").unwrap();
    fs::write(temp_path.join(\"test.py\"), \"def test(): pass\").unwrap();
    fs::write(temp_path.join(\"README.md\"), \"# Test\").unwrap();
    
    // Create subdirectory
    let subdir = temp_path.join(\"src\");
    fs::create_dir(&subdir).unwrap();
    fs::write(subdir.join(\"main.rs\"), \"fn main() {}\").unwrap();

    let files = collect_files(&[temp_path.to_str().unwrap().to_string()]).unwrap();
    
    // Should find .js, .py, and .rs files, but not .md
    assert_eq!(files.len(), 3);
    assert!(files.iter().any(|f| f.ends_with(\"test.js\")));
    assert!(files.iter().any(|f| f.ends_with(\"test.py\")));
    assert!(files.iter().any(|f| f.ends_with(\"main.rs\")));
    assert!(!files.iter().any(|f| f.ends_with(\"README.md\")));
}

    #[test]
    fn test_collect_files_single_file() {
        let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    let test_file = temp_path.join(\"test.js\");
    fs::write(&test_file, \"function test() {}\").unwrap();

    let files = collect_files(&[test_file.to_str().unwrap().to_string()]).unwrap();
    
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with(\"test.js\"));
}

#[test]
fn test_collect_files_nonexistent_path() {
    let result = collect_files(&[\"/nonexistent/path\".to_string()]);
    assert!(result.is_err());
}

#[test]
fn test_is_supported_file() {
    assert!(is_supported_file(Path::new(\"test.js\")));
    assert!(is_supported_file(Path::new(\"test.ts\")));
    assert!(is_supported_file(Path::new(\"test.py\")));
    assert!(is_supported_file(Path::new(\"test.rs\")));
    assert!(is_supported_file(Path::new(\"test.go\")));
    assert!(is_supported_file(Path::new(\"test.rb\")));
    assert!(is_supported_file(Path::new(\"test.sh\")));
    
    assert!(!is_supported_file(Path::new(\"test.txt\")));
    assert!(!is_supported_file(Path::new(\"README.md\")));
    assert!(!is_supported_file(Path::new(\"Cargo.toml\")));
}

    #[test]
    fn test_process_files() {
        let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    
    // Create a file with violations
    let test_file = temp_path.join(\"test.js\");
    fs::write(&test_file, r#\"
        function testWithManyParams(a, b, c, d, e, f) {
            if (a > b) {
                if (c > d) {
                    if (e > f) {
                        if (a > c) {
                            return a + b + c + d + e + f;
                        }
                    }
                }
            }
            return 0;
        }
    \"#).unwrap();

    let config = config::Config::default();
    let violations = process_files(&[test_file.to_str().unwrap().to_string()], &config).unwrap();
    
    assert!(!violations.is_empty());
    // Should have violations for max-params and function-depth
    let param_violations: Vec<_> = violations.iter().filter(|v| v.rule == \"max-params\").collect();
    let depth_violations: Vec<_> = violations.iter().filter(|v| v.rule == \"function-depth\").collect();
    
    assert!(!param_violations.is_empty());
    assert!(!depth_violations.is_empty());
}

    #[test]
    fn test_process_files_with_ignore() {
        let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    
    // Create a file with ignore directive
    let test_file = temp_path.join(\"test.js\");
    fs::write(&test_file, r#\"
        // violet-ignore max-params
        function testWithManyParams(a, b, c, d, e, f) {
            return a + b + c + d + e + f;
        }
    \"#).unwrap();

    let config = config::Config::default();
    let violations = process_files(&[test_file.to_str().unwrap().to_string()], &config).unwrap();
    
    // Should have no max-params violations due to ignore directive
    let param_violations: Vec<_> = violations.iter().filter(|v| v.rule == \"max-params\").collect();
    assert!(param_violations.is_empty());
}

    #[test]
    fn test_process_files_clean_code() {
        let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();
    
    // Create a clean file
    let test_file = temp_path.join(\"test.js\");
    fs::write(&test_file, r#\"
        function clean(a, b) {
            return a + b;
        }
    \"#).unwrap();

    let config = config::Config::default();
    let violations = process_files(&[test_file.to_str().unwrap().to_string()], &config).unwrap();
    
    assert!(violations.is_empty());
}

#[test]
fn test_get_exit_code() {
    assert_eq!(get_exit_code(&vec![]), 0);
    
    let violations = vec![
        Violation {
            file: \"test.js\".into(),
            line: 1,
            column: 10,
            rule: \"max-params\".to_string(),
            message: \"Too many parameters\".to_string(),
            severity: Severity::Warning,
        },
    ];
    assert_eq!(get_exit_code(&violations), 1);
    
    let violations = vec![
        Violation {
            file: \"test.js\".into(),
            line: 1,
            column: 10,
            rule: \"max-params\".to_string(),
            message: \"Too many parameters\".to_string(),
            severity: Severity::Error,
        },
    ];
    assert_eq!(get_exit_code(&violations), 1);
}

#[test]
fn test_output_format_default() {
    assert_eq!(OutputFormat::default(), OutputFormat::Pretty);
}

    #[test]
    fn test_cli_config_option() {
        let cli = Cli::try_parse_from(&[\"violet\", \"--config\", \"custom.json\", \"src/\"]).unwrap();
        assert_eq!(cli.config, Some(\"custom.json\".to_string()));
    }
} 