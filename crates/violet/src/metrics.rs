//! Code complexity metrics calculation
//! 
//! Analyzes parsed syntax trees to calculate various complexity metrics
//! including cyclomatic complexity, nesting depth, and line counts.

use tree_sitter::{Node, Tree};
use crate::{ComplexityMetrics, Language};

/// Calculate complexity metrics for a function node
pub fn calculate_function_metrics(node: Node, source_code: &str, language: Language) -> ComplexityMetrics {
    let start_line = node.start_position().row + 1; // 1-indexed
    let end_line = node.end_position().row + 1;
    let line_count = end_line - start_line + 1;
    
    ComplexityMetrics {
        param_count: crate::parser::get_parameter_count(node, language),
        line_count,
        max_depth: calculate_nesting_depth(node),
        cyclomatic_complexity: calculate_cyclomatic_complexity(node, language),
        start_line,
        end_line,
    }
}

/// Calculate complexity metrics for an entire file
pub fn calculate_file_metrics(tree: &Tree, source_code: &str, language: Language) -> ComplexityMetrics {
    let root_node = tree.root_node();
    let line_count = source_code.lines().count();
    
    ComplexityMetrics {
        param_count: 0, // Not applicable for files
        line_count,
        max_depth: calculate_nesting_depth(root_node),
        cyclomatic_complexity: calculate_cyclomatic_complexity(root_node, language),
        start_line: 1,
        end_line: line_count,
    }
}

/// Calculate the maximum nesting depth in a syntax node
pub fn calculate_nesting_depth(node: Node) -> usize {
    calculate_depth_recursive(node, 0)
}

/// Recursively calculate nesting depth
fn calculate_depth_recursive(node: Node, current_depth: usize) -> usize {
    let mut max_depth = current_depth;
    
    // Check if this node increases nesting depth
    let depth_increment = if is_nesting_node(node) { 1 } else { 0 };
    let new_depth = current_depth + depth_increment;
    
    // Recursively check children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            let child_depth = calculate_depth_recursive(child, new_depth);
            max_depth = max_depth.max(child_depth);
        }
    }
    
    max_depth
}

/// Check if a node represents a nesting construct
fn is_nesting_node(node: Node) -> bool {
    matches!(node.kind(),
        // Control flow structures
        "if_statement" | "else_clause" | "elif_clause" |
        "while_statement" | "for_statement" | "for_in_statement" |
        "switch_statement" | "case_clause" | "default_clause" |
        "try_statement" | "catch_clause" | "finally_clause" |
        "with_statement" |
        
        // Blocks and scopes
        "block" | "compound_statement" | "statement_block" |
        
        // Function definitions (nested functions)
        "function_declaration" | "function_expression" | "arrow_function" |
        "function_definition" | "async_function_definition" |
        "method_definition" | "method" |
        
        // Class definitions
        "class_declaration" | "class_definition" |
        
        // Other nesting constructs
        "object_expression" | "array_expression" |
        "do_statement" | "labeled_statement"
    )
}

/// Calculate cyclomatic complexity for a syntax node
pub fn calculate_cyclomatic_complexity(node: Node, language: Language) -> usize {
    let mut complexity = 1; // Base complexity
    calculate_complexity_recursive(node, language, &mut complexity);
    complexity
}

/// Recursively calculate cyclomatic complexity
fn calculate_complexity_recursive(node: Node, language: Language, complexity: &mut usize) {
    // Check if this node adds to complexity
    if is_complexity_node(node, language) {
        *complexity += 1;
    }
    
    // Recursively check children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            calculate_complexity_recursive(child, language, complexity);
        }
    }
}

/// Check if a node adds to cyclomatic complexity
fn is_complexity_node(node: Node, language: Language) -> bool {
    match language {
        Language::JavaScript | Language::TypeScript => {
            matches!(node.kind(),
                "if_statement" | "else_clause" |
                "while_statement" | "for_statement" | "for_in_statement" | "for_of_statement" |
                "switch_statement" | "case_clause" |
                "try_statement" | "catch_clause" |
                "conditional_expression" | "ternary_expression" |
                "logical_expression" | "binary_expression" |
                "do_statement"
            ) || is_logical_operator(node)
        }
        Language::Python => {
            matches!(node.kind(),
                "if_statement" | "elif_clause" | "else_clause" |
                "while_statement" | "for_statement" |
                "try_statement" | "except_clause" | "finally_clause" |
                "with_statement" |
                "conditional_expression" |
                "boolean_operator" | "comparison_operator"
            )
        }
        Language::Rust => {
            matches!(node.kind(),
                "if_expression" | "else_clause" |
                "while_expression" | "for_expression" | "loop_expression" |
                "match_expression" | "match_arm" |
                "try_expression" |
                "binary_expression" | "range_expression"
            )
        }
        Language::Bash => {
            matches!(node.kind(),
                "if_statement" | "elif_clause" | "else_clause" |
                "while_statement" | "for_statement" |
                "case_statement" | "case_item" |
                "test_command" | "binary_expression"
            )
        }
        Language::Go => {
            matches!(node.kind(),
                "if_statement" | "else_clause" |
                "for_statement" | "range_clause" |
                "switch_statement" | "expression_case" | "type_case" |
                "select_statement" | "communication_case" |
                "binary_expression"
            )
        }
        Language::Ruby => {
            matches!(node.kind(),
                "if" | "unless" | "elsif" | "else" |
                "while" | "until" | "for" |
                "case" | "when" |
                "begin" | "rescue" | "ensure" |
                "binary" | "and" | "or"
            )
        }
    }
}

/// Check if a node is a logical operator that adds complexity
fn is_logical_operator(node: Node) -> bool {
    if node.kind() == "binary_expression" {
        // Check if it's a logical AND or OR
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if matches!(child.kind(), "&&" | "||" | "and" | "or") {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if source code contains ignore directives for specific rules
pub fn has_ignore_directive(source_code: &str, line_number: usize, rule: &str) -> bool {
    let lines: Vec<&str> = source_code.lines().collect();
    
    // Convert to 0-indexed and check the line itself and the line before for ignore comments
    let zero_indexed_line = line_number.saturating_sub(1);
    for check_line in [zero_indexed_line.saturating_sub(1), zero_indexed_line] {
        if let Some(line) = lines.get(check_line) {
            if line.contains(&format!("violet-ignore {}", rule)) ||
               line.contains("violet-ignore all") ||
               line.contains(&format!("sentinel-ignore {}", rule)) || // V1 compatibility
               line.contains("sentinel-ignore all") {
                return true;
            }
        }
    }
    
    false
}

/// Extract the function name from a function node (for reporting)
pub fn get_function_name(node: Node, language: Language, source_code: &[u8]) -> Option<String> {
    match language {
        Language::JavaScript | Language::TypeScript => {
            // Look for identifier in function declaration
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "identifier" {
                        return Some(child.utf8_text(source_code).unwrap_or("").to_string());
                    }
                }
            }
            None
        }
        Language::Python => {
            // Look for identifier after 'def'
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "identifier" {
                        return Some(child.utf8_text(source_code).unwrap_or("").to_string());
                    }
                }
            }
            None
        }
        Language::Rust => {
            // Look for identifier in function_item
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "identifier" {
                        return Some(child.utf8_text(source_code).unwrap_or("").to_string());
                    }
                }
            }
            None
        }
        Language::Bash => {
            // Bash function names are typically the first word
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "word" {
                        return Some(child.utf8_text(source_code).unwrap_or("").to_string());
                    }
                }
            }
            None
        }
        Language::Go => {
            // Look for identifier in function declaration
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "identifier" {
                        return Some(child.utf8_text(source_code).unwrap_or("").to_string());
                    }
                }
            }
            None
        }
        Language::Ruby => {
            // Look for identifier in method
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if matches!(child.kind(), "identifier" | "constant") {
                        return Some(child.utf8_text(source_code).unwrap_or("").to_string());
                    }
                }
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::LanguageParser;
    
    #[test]
    fn test_nesting_depth() {
        // Test with a simple nested structure
        let mut parser = LanguageParser::new(Language::JavaScript).unwrap();
        let source = r#"
            function test() {
                if (true) {
                    if (false) {
                        return 1;
                    }
                }
                return 0;
            }
        "#;
        
        let tree = parser.parse(source).unwrap();
        let functions = crate::parser::extract_function_nodes(&tree, Language::JavaScript);
        assert!(!functions.is_empty());
        
        let depth = calculate_nesting_depth(functions[0]);
        assert!(depth >= 3); // function -> if -> if
    }
    
    #[test]
    fn test_cyclomatic_complexity() {
        let mut parser = LanguageParser::new(Language::JavaScript).unwrap();
        let source = r#"
            function test(x) {
                if (x > 0) {
                    return 1;
                } else if (x < 0) {
                    return -1;
                } else {
                    return 0;
                }
            }
        "#;
        
        let tree = parser.parse(source).unwrap();
        let functions = crate::parser::extract_function_nodes(&tree, Language::JavaScript);
        assert!(!functions.is_empty());
        
        let complexity = calculate_cyclomatic_complexity(functions[0], Language::JavaScript);
        assert!(complexity > 1); // Should have multiple paths
    }
    
    #[test]
    fn test_ignore_directive() {
        let source = r#"
            // violet-ignore complexity
            function complex() {
                if (a) {
                    if (b) {
                        if (c) {
                            return 1;
                        }
                    }
                }
            }
        "#;
        
        assert!(has_ignore_directive(source, 2, "complexity"));
        assert!(!has_ignore_directive(source, 5, "complexity"));
    }
} 