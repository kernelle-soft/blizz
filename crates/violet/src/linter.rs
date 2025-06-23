//! Rule enforcement and violation detection

use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use tree_sitter::Node;

use crate::{
    config::{Config, RuleThresholds},
    parser::{LanguageParser, detect_language, extract_function_nodes},
    metrics::{calculate_function_metrics, calculate_file_metrics, has_ignore_directive, get_function_name},
    Language, Violation, Severity, ComplexityMetrics, Result, VioletError,
};

/// Main linter that analyzes files and generates violations
pub struct Linter {
    config: Config,
}

impl Linter {
    /// Create a new linter with the given configuration
    pub fn new(config: Config) -> Self {
        Self { config }
    }
    
    /// Analyze a single file and return violations
    pub fn analyze_file<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<Violation>> {
        let file_path = file_path.as_ref();
        
        // Check if file should be ignored
        if self.config.should_ignore_file(file_path) {
            return Ok(Vec::new());
        }
        
        // Detect language
        let language = match detect_language(file_path) {
            Some(lang) => lang,
            None => return Ok(Vec::new()), // Skip unsupported files
        };
        
        // Read file content
        let source_code = std::fs::read_to_string(file_path)?;
        
        // Parse the file
        let mut parser = LanguageParser::new(language)?;
        let tree = parser.parse(&source_code)?;
        
        let mut violations = Vec::new();
        
        // Analyze file-level metrics
        let file_metrics = calculate_file_metrics(&tree, &source_code, language);
        violations.extend(self.check_file_violations(file_path, &file_metrics, &source_code, language));
        
        // Analyze function-level metrics
        let functions = extract_function_nodes(&tree, language);
        for function_node in functions {
            let function_metrics = calculate_function_metrics(function_node, &source_code, language);
            violations.extend(self.check_function_violations(
                file_path,
                function_node,
                &function_metrics,
                &source_code,
                language,
            ));
        }
        
        Ok(violations)
    }
    
    /// Check function-level violations
    fn check_function_violations(
        &self,
        file_path: &Path,
        function_node: Node,
        metrics: &ComplexityMetrics,
        source_code: &str,
        language: Language,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();
        let rules = self.config.get_rules_for_language(language);
        let function_name = get_function_name(function_node, language, source_code.as_bytes())
            .unwrap_or_else(|| "<anonymous>".to_string());
        
        // Check parameter count
        if metrics.param_count > rules.max_params {
            if !has_ignore_directive(source_code, metrics.start_line, "max-params") {
                violations.push(Violation {
                    rule: "max-params".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "Function '{}' has too many parameters: {} (max: {})",
                        function_name, metrics.param_count, rules.max_params
                    ),
                    file: file_path.to_path_buf(),
                    line: metrics.start_line,
                    column: None,
                    suggestion: Some("Consider using an options object".to_string()),
                });
            }
        }
        
        // Check function length
        if metrics.line_count > rules.max_function_lines {
            if !has_ignore_directive(source_code, metrics.start_line, "function-length") {
                violations.push(Violation {
                    rule: "function-length".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "Function '{}' is too long: {} lines (max: {})",
                        function_name, metrics.line_count, rules.max_function_lines
                    ),
                    file: file_path.to_path_buf(),
                    line: metrics.start_line,
                    column: None,
                    suggestion: Some("Extract logical blocks into helper functions".to_string()),
                });
            }
        }
        
        // Check function nesting depth
        if metrics.max_depth > rules.max_function_depth {
            if !has_ignore_directive(source_code, metrics.start_line, "function-depth") {
                violations.push(Violation {
                    rule: "function-depth".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "Function '{}' has excessive nesting depth: {} (max: {})",
                        function_name, metrics.max_depth, rules.max_function_depth
                    ),
                    file: file_path.to_path_buf(),
                    line: metrics.start_line,
                    column: None,
                    suggestion: Some("Use early returns or extract nested logic into helper functions".to_string()),
                });
            }
        }
        
        // Check cyclomatic complexity
        if metrics.cyclomatic_complexity > rules.max_complexity {
            if !has_ignore_directive(source_code, metrics.start_line, "complexity") {
                violations.push(Violation {
                    rule: "complexity".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "Function '{}' is too complex: {} (max: {})",
                        function_name, metrics.cyclomatic_complexity, rules.max_complexity
                    ),
                    file: file_path.to_path_buf(),
                    line: metrics.start_line,
                    column: None,
                    suggestion: Some("Break down complex logic into smaller functions".to_string()),
                });
            }
        }
        
        violations
    }
    
    /// Check file-level violations
    fn check_file_violations(
        &self,
        file_path: &Path,
        metrics: &ComplexityMetrics,
        source_code: &str,
        language: Language,
    ) -> Vec<Violation> {
        let mut violations = Vec::new();
        let rules = self.config.get_rules_for_language(language);
        
        // Check file length
        if metrics.line_count > rules.max_file_lines {
            if !has_ignore_directive(source_code, 1, "file-length") {
                violations.push(Violation {
                    rule: "file-length".to_string(),
                    severity: Severity::Warning,
                    message: format!(
                        "File is too long: {} lines (max: {})",
                        metrics.line_count, rules.max_file_lines
                    ),
                    file: file_path.to_path_buf(),
                    line: 1,
                    column: None,
                    suggestion: Some("Split into smaller modules".to_string()),
                });
            }
        }
        
        violations
    }
}

/// Format violations for display
pub fn format_violations(violations: &[Violation]) -> String {
    if violations.is_empty() {
        return "✨ No violations found! Your code is beautiful.".to_string();
    }
    
    let mut output = String::new();
    let mut current_file: Option<&PathBuf> = None;
    
    for violation in violations {
        // Print file header if it's a new file
        if current_file != Some(&violation.file) {
            if current_file.is_some() {
                output.push('\n');
            }
            output.push_str(&format!("📁 {}\n", violation.file.display()));
            current_file = Some(&violation.file);
        }
        
        // Format violation
        let severity_icon = match violation.severity {
            Severity::Error => "❌",
            Severity::Warning => "⚠️",
            Severity::Info => "ℹ️",
        };
        
        output.push_str(&format!(
            "  {}:{} {} [{}] {}\n",
            violation.line,
            violation.column.map(|c| c.to_string()).unwrap_or_else(|| "1".to_string()),
            severity_icon,
            violation.rule,
            violation.message
        ));
        
        if let Some(suggestion) = &violation.suggestion {
            output.push_str(&format!("    💡 {}\n", suggestion));
        }
    }
    
    // Summary
    let error_count = violations.iter().filter(|v| v.severity == Severity::Error).count();
    let warning_count = violations.iter().filter(|v| v.severity == Severity::Warning).count();
    let info_count = violations.iter().filter(|v| v.severity == Severity::Info).count();
    
    output.push_str(&format!(
        "\n📊 Summary: {} errors, {} warnings, {} info\n",
        error_count, warning_count, info_count
    ));
    
    output
} 