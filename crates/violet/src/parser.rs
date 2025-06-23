//! Tree-sitter parsing for multiple languages
//! 
//! Handles parsing source code files using tree-sitter parsers
//! and extracting syntax nodes for analysis.

use std::path::Path;
use tree_sitter::{Language as TSLanguage, Parser, Tree, Node};

use crate::{Language, Result, VioletError};

/// Language parser that wraps tree-sitter functionality
pub struct LanguageParser {
    parser: Parser,
    language: Language,
}

impl LanguageParser {
    /// Create a new parser for the specified language
    pub fn new(language: Language) -> Result<Self> {
        let mut parser = Parser::new();
        let ts_language = get_tree_sitter_language(language)?;
        
        parser.set_language(ts_language)
            .map_err(|e| VioletError::Parser(format!("Failed to set language: {}", e)))?;
            
        Ok(Self { parser, language })
    }
    
    /// Parse source code and return the syntax tree
    pub fn parse(&mut self, source_code: &str) -> Result<Tree> {
        self.parser
            .parse(source_code, None)
            .ok_or_else(|| VioletError::Parser("Failed to parse source code".to_string()))
    }
    
    /// Get the language this parser handles
    pub fn language(&self) -> Language {
        self.language
    }
}

/// Get the appropriate tree-sitter language for our Language enum
fn get_tree_sitter_language(language: Language) -> Result<TSLanguage> {
    match language {
        Language::JavaScript => Ok(tree_sitter_javascript::language()),
        Language::TypeScript => Ok(tree_sitter_typescript::language_typescript()),
        Language::Python => Ok(tree_sitter_python::language()),
        Language::Rust => Ok(tree_sitter_rust::language()),
        Language::Bash => Ok(tree_sitter_bash::language()),
        Language::Go => Ok(tree_sitter_go::language()),
        Language::Ruby => Ok(tree_sitter_ruby::language()),
    }
}

/// Detect language from file path
pub fn detect_language<P: AsRef<Path>>(path: P) -> Option<Language> {
    let path = path.as_ref();
    let extension = path.extension()?.to_str()?;
    Language::from_extension(extension)
}

/// Extract function nodes from a syntax tree
pub fn extract_function_nodes<'a>(tree: &'a Tree, language: Language) -> Vec<Node<'a>> {
    let root_node = tree.root_node();
    let mut functions = Vec::new();
    extract_functions_recursive(root_node, language, &mut functions);
    functions
}

/// Recursively extract function nodes based on language-specific patterns
fn extract_functions_recursive<'a>(node: Node<'a>, language: Language, functions: &mut Vec<Node<'a>>) {
    let function_types = get_function_node_types(language);
    
    if function_types.contains(&node.kind()) {
        functions.push(node);
    }
    
    // Recursively check children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            extract_functions_recursive(child, language, functions);
        }
    }
}

/// Get function node type names for each language
fn get_function_node_types(language: Language) -> &'static [&'static str] {
    match language {
        Language::JavaScript | Language::TypeScript => &[
            "function_declaration",
            "function_expression", 
            "arrow_function",
            "method_definition",
        ],
        Language::Python => &[
            "function_definition",
            "async_function_definition",
        ],
        Language::Rust => &[
            "function_item",
            "closure_expression",
        ],
        Language::Bash => &[
            "function_definition",
        ],
        Language::Go => &[
            "function_declaration",
            "method_declaration",
        ],
        Language::Ruby => &[
            "method",
            "singleton_method",
        ],
    }
}

/// Get parameter count from a function node
pub fn get_parameter_count(node: Node, language: Language) -> usize {
    match language {
        Language::JavaScript | Language::TypeScript => {
            if let Some(params_node) = find_child_by_type(node, "formal_parameters") {
                count_parameters_js_like(params_node)
            } else {
                0
            }
        }
        Language::Python => {
            if let Some(params_node) = find_child_by_type(node, "parameters") {
                count_parameters_python(params_node)
            } else {
                0
            }
        }
        Language::Rust => {
            if let Some(params_node) = find_child_by_type(node, "parameters") {
                count_parameters_rust(params_node)
            } else {
                0
            }
        }
        Language::Bash => {
            // Bash functions don't have explicit parameters in the signature
            0
        }
        Language::Go => {
            if let Some(params_node) = find_child_by_type(node, "parameter_list") {
                count_parameters_go(params_node)
            } else {
                0
            }
        }
        Language::Ruby => {
            if let Some(params_node) = find_child_by_type(node, "method_parameters") {
                count_parameters_ruby(params_node)
            } else {
                0
            }
        }
    }
}

/// Find a child node by its type
fn find_child_by_type<'a>(node: Node<'a>, type_name: &str) -> Option<Node<'a>> {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == type_name {
                return Some(child);
            }
        }
    }
    None
}

/// Count parameters in JavaScript/TypeScript style
fn count_parameters_js_like(params_node: Node) -> usize {
    let mut count = 0;
    for i in 0..params_node.child_count() {
        if let Some(child) = params_node.child(i) {
            if matches!(child.kind(), "identifier" | "assignment_pattern" | "rest_pattern") {
                count += 1;
            }
        }
    }
    count
}

/// Count parameters in Python style
fn count_parameters_python(params_node: Node) -> usize {
    let mut count = 0;
    for i in 0..params_node.child_count() {
        if let Some(child) = params_node.child(i) {
            if matches!(child.kind(), "identifier" | "default_parameter" | "typed_parameter") {
                count += 1;
            }
        }
    }
    count
}

/// Count parameters in Rust style
fn count_parameters_rust(params_node: Node) -> usize {
    let mut count = 0;
    for i in 0..params_node.child_count() {
        if let Some(child) = params_node.child(i) {
            if child.kind() == "parameter" {
                count += 1;
            }
        }
    }
    count
}

/// Count parameters in Go style
fn count_parameters_go(params_node: Node) -> usize {
    let mut count = 0;
    for i in 0..params_node.child_count() {
        if let Some(child) = params_node.child(i) {
            if child.kind() == "parameter_declaration" {
                count += 1;
            }
        }
    }
    count
}

/// Count parameters in Ruby style
fn count_parameters_ruby(params_node: Node) -> usize {
    let mut count = 0;
    for i in 0..params_node.child_count() {
        if let Some(child) = params_node.child(i) {
            if matches!(child.kind(), "identifier" | "optional_parameter" | "splat_parameter") {
                count += 1;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("test.js"), Some(Language::JavaScript));
        assert_eq!(detect_language("test.ts"), Some(Language::TypeScript));
        assert_eq!(detect_language("test.py"), Some(Language::Python));
        assert_eq!(detect_language("test.rs"), Some(Language::Rust));
        assert_eq!(detect_language("test.sh"), Some(Language::Bash));
        assert_eq!(detect_language("test.go"), Some(Language::Go));
        assert_eq!(detect_language("test.rb"), Some(Language::Ruby));
        assert_eq!(detect_language("test.unknown"), None);
    }
    
    #[test]
    fn test_javascript_parser() -> Result<()> {
        let mut parser = LanguageParser::new(Language::JavaScript)?;
        let source = "function test(a, b, c) { return a + b + c; }";
        let tree = parser.parse(source)?;
        
        let functions = extract_function_nodes(&tree, Language::JavaScript);
        assert_eq!(functions.len(), 1);
        
        let param_count = get_parameter_count(functions[0], Language::JavaScript);
        assert_eq!(param_count, 3);
        
        Ok(())
    }
} 