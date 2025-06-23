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
    use std::path::Path;

    // Helper function for tests
    fn parse_code(code: &str, language: Language) -> Result<Tree> {
        let mut parser = LanguageParser::new(language)?;
        parser.parse(code)
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(Path::new("test.js")), Some(Language::JavaScript));
        assert_eq!(detect_language(Path::new("test.mjs")), Some(Language::JavaScript));
        assert_eq!(detect_language(Path::new("test.ts")), Some(Language::TypeScript));
        assert_eq!(detect_language(Path::new("test.tsx")), Some(Language::TypeScript));
        assert_eq!(detect_language(Path::new("test.py")), Some(Language::Python));
        assert_eq!(detect_language(Path::new("test.rs")), Some(Language::Rust));
        assert_eq!(detect_language(Path::new("test.sh")), Some(Language::Bash));
        assert_eq!(detect_language(Path::new("test.go")), Some(Language::Go));
        assert_eq!(detect_language(Path::new("test.rb")), Some(Language::Ruby));
        assert_eq!(detect_language(Path::new("test.unknown")), None);
        assert_eq!(detect_language(Path::new("no_extension")), None);
    }

    #[test]
    fn test_detect_language_case_insensitive() {
        assert_eq!(detect_language(Path::new("TEST.JS")), Some(Language::JavaScript));
        assert_eq!(detect_language(Path::new("Test.Py")), Some(Language::Python));
        assert_eq!(detect_language(Path::new("file.RS")), Some(Language::Rust));
    }

    #[test]
    fn test_detect_language_complex_paths() {
        assert_eq!(detect_language(Path::new("src/components/Button.tsx")), Some(Language::TypeScript));
        assert_eq!(detect_language(Path::new("/absolute/path/to/script.sh")), Some(Language::Bash));
        assert_eq!(detect_language(Path::new("../relative/path/module.py")), Some(Language::Python));
    }

    #[test]
    fn test_javascript_parser() {
        let code = r#"
            function test(a, b, c) {
                if (a > b) {
                    if (b > c) {
                        return a + b + c;
                    }
                }
                return 0;
            }
        "#;
        
        let result = parse_code(code, Language::JavaScript);
        assert!(result.is_ok());
        let tree = result.unwrap();
        assert!(!tree.root_node().is_error());
    }

    #[test]
    fn test_typescript_parser() {
        let code = r#"
            interface User {
                name: string;
                age: number;
            }
            
            function greet(user: User): string {
                return `Hello, ${user.name}!`;
            }
        "#;
        
        let result = parse_code(code, Language::TypeScript);
        assert!(result.is_ok());
        let tree = result.unwrap();
        assert!(!tree.root_node().is_error());
    }

    #[test]
    fn test_python_parser() {
        let code = r#"
def fibonacci(n):
    if n <= 1:
        return n
    else:
        return fibonacci(n-1) + fibonacci(n-2)

class Calculator:
    def add(self, a, b):
        return a + b
        "#;
        
        let result = parse_code(code, Language::Python);
        assert!(result.is_ok());
        let tree = result.unwrap();
        assert!(!tree.root_node().is_error());
    }

    #[test]
    fn test_rust_parser() {
        let code = r#"
            fn main() {
                let x = 5;
                let y = {
                    let x = 3;
                    x + 1
                };
                println!("The value of y is: {}", y);
            }
            
            struct Point {
                x: i32,
                y: i32,
            }
        "#;
        
        let result = parse_code(code, Language::Rust);
        assert!(result.is_ok());
        let tree = result.unwrap();
        assert!(!tree.root_node().is_error());
    }

    #[test]
    fn test_bash_parser() {
        let code = r#"
            #!/bin/bash
            
            function backup_files() {
                local source_dir=$1
                local backup_dir=$2
                
                if [ ! -d "$backup_dir" ]; then
                    mkdir -p "$backup_dir"
                fi
                
                cp -r "$source_dir"/* "$backup_dir"/
            }
            
            backup_files "/home/user/documents" "/backup/documents"
        "#;
        
        let result = parse_code(code, Language::Bash);
        assert!(result.is_ok());
        let tree = result.unwrap();
        assert!(!tree.root_node().is_error());
    }

    #[test]
    fn test_go_parser() {
        let code = r#"
            package main
            
            import "fmt"
            
            func fibonacci(n int) int {
                if n <= 1 {
                    return n
                }
                return fibonacci(n-1) + fibonacci(n-2)
            }
            
            func main() {
                fmt.Println(fibonacci(10))
            }
        "#;
        
        let result = parse_code(code, Language::Go);
        assert!(result.is_ok());
        let tree = result.unwrap();
        assert!(!tree.root_node().is_error());
    }

    #[test]
    fn test_ruby_parser() {
        let code = r#"
            class Calculator
              def initialize
                @result = 0
              end
              
              def add(x, y)
                @result = x + y
              end
              
              def multiply(x, y)
                @result = x * y
              end
            end
            
            calc = Calculator.new
            calc.add(5, 3)
        "#;
        
        let result = parse_code(code, Language::Ruby);
        assert!(result.is_ok());
        let tree = result.unwrap();
        assert!(!tree.root_node().is_error());
    }

    #[test]
    fn test_parse_invalid_syntax() {
        let code = "function test( { invalid syntax }";
        let result = parse_code(code, Language::JavaScript);
        assert!(result.is_ok()); // Tree-sitter can parse partial/invalid code
        let tree = result.unwrap();
        // The tree might have error nodes, but tree-sitter doesn't fail completely
        assert!(tree.root_node().child_count() > 0);
    }

    #[test]
    fn test_parse_empty_code() {
        let result = parse_code("", Language::JavaScript);
        assert!(result.is_ok());
        let tree = result.unwrap();
        assert_eq!(tree.root_node().child_count(), 0);
    }

    #[test]
    fn test_parse_whitespace_only() {
        let result = parse_code("   \n\t  \n  ", Language::Python);
        assert!(result.is_ok());
        let tree = result.unwrap();
        // Whitespace-only code should parse successfully
        assert!(!tree.root_node().is_error());
    }

    #[test]
    fn test_parse_complex_nesting() {
        let code = r#"
            function complexNesting() {
                if (condition1) {
                    if (condition2) {
                        if (condition3) {
                            if (condition4) {
                                if (condition5) {
                                    return "deeply nested";
                                }
                            }
                        }
                    }
                }
                return "not so deep";
            }
        "#;
        
        let result = parse_code(code, Language::JavaScript);
        assert!(result.is_ok());
        let tree = result.unwrap();
        assert!(!tree.root_node().is_error());
    }

    #[test]
    fn test_parse_with_comments() {
        let code = r#"
            // This is a comment
            function test() {
                /* Multi-line
                   comment */
                return 42; // End of line comment
            }
        "#;
        
        let result = parse_code(code, Language::JavaScript);
        assert!(result.is_ok());
        let tree = result.unwrap();
        assert!(!tree.root_node().is_error());
    }

    #[test]
    fn test_parse_large_function() {
        let mut code = String::from("function largeFunction() {\n");
        for i in 0..100 {
            code.push_str(&format!("    let var{} = {};\n", i, i));
        }
        code.push_str("    return var99;\n}");
        
        let result = parse_code(&code, Language::JavaScript);
        assert!(result.is_ok());
        let tree = result.unwrap();
        assert!(!tree.root_node().is_error());
    }

    #[test]
    fn test_parse_unicode_content() {
        let code = r#"
            function greetInDifferentLanguages() {
                const greetings = {
                    english: "Hello",
                    spanish: "Hola",
                    japanese: "こんにちは",
                    arabic: "مرحبا",
                    emoji: "👋🌍"
                };
                return greetings;
            }
        "#;
        
        let result = parse_code(code, Language::JavaScript);
        assert!(result.is_ok());
        let tree = result.unwrap();
        assert!(!tree.root_node().is_error());
    }

    #[test]
    fn test_parser_consistency() {
        let code = "function test() { return 42; }";
        
        // Parse the same code multiple times to ensure consistency
        let result1 = parse_code(code, Language::JavaScript);
        let result2 = parse_code(code, Language::JavaScript);
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        
        let tree1 = result1.unwrap();
        let tree2 = result2.unwrap();
        
        // Trees should have the same structure
        assert_eq!(tree1.root_node().kind(), tree2.root_node().kind());
        assert_eq!(tree1.root_node().child_count(), tree2.root_node().child_count());
    }

    #[test]
    fn test_all_supported_languages_parse() {
        let test_codes = vec![
            (Language::JavaScript, "function test() { return 1; }"),
            (Language::TypeScript, "function test(): number { return 1; }"),
            (Language::Python, "def test():\n    return 1"),
            (Language::Rust, "fn test() -> i32 { 1 }"),
            (Language::Bash, "function test() { echo 1; }"),
            (Language::Go, "func test() int { return 1 }"),
            (Language::Ruby, "def test\n  1\nend"),
        ];
        
        for (language, code) in test_codes {
            let result = parse_code(code, language);
            assert!(result.is_ok(), "Failed to parse {} code: {}", language, code);
            let tree = result.unwrap();
            assert!(!tree.root_node().is_error(), "Parse error in {} code", language);
        }
    }
} 