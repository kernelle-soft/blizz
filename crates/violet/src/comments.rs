//! Comment analysis for detecting obvious/redundant comments
//!
//! This module provides functionality to detect "no-duh" comments that state the obvious
//! without adding meaningful context or documentation value.

use regex::Regex;

/// Result of analyzing comments in a code chunk
#[derive(Debug, Clone)]
pub struct CommentAnalysisResult {
    pub obvious_comments: Vec<ObviousComment>,
    pub has_violations: bool,
}

/// Represents an obvious comment that was detected
#[derive(Debug, Clone)]
pub struct ObviousComment {
    pub line_number: usize,
    pub comment_text: String,
    pub reason: String,
    pub code_line: Option<String>,
}

/// Types of obvious comment patterns we can detect
#[derive(Debug, Clone)]
enum CommentPattern {
    /// Comments that just translate code to English
    CodeTranslation,
    /// Comments that state what a return statement does
    ObviousReturn,
    /// Comments about variable initialization that add no context
    VariableInitialization,
    /// Comments that just repeat loop constructs
    LoopDescription,
    /// Comments that just state what an assignment does
    ObviousAssignment,
}

impl CommentPattern {
    fn get_regex(&self) -> Regex {
        match self {
            CommentPattern::CodeTranslation => {
                // Matches comments like "set x to", "assign", "initialize"
                Regex::new(r"(?i)(set\s+\w+\s+to|assign|initialize)").unwrap()
            }
            CommentPattern::ObviousReturn => {
                // Matches comments like "return true", "return false", "return result"
                Regex::new(r"(?i)return\s+(true|false|null|none|\w+)").unwrap()
            }
            CommentPattern::VariableInitialization => {
                // Matches comments like "initialize variable", "declare variable"
                Regex::new(r"(?i)(initialize|declare)\s+(variable|var)").unwrap()
            }
            CommentPattern::LoopDescription => {
                // Matches comments like "loop through", "iterate over"
                Regex::new(r"(?i)(loop\s+through|iterate\s+over|for\s+each)").unwrap()
            }
            CommentPattern::ObviousAssignment => {
                // Matches comments like "set", "assign" followed by simple assignments
                Regex::new(r"(?i)(set|assign)\s+\w+").unwrap()
            }
        }
    }

    fn get_reason(&self) -> &'static str {
        match self {
            CommentPattern::CodeTranslation => "Comment just translates code to English",
            CommentPattern::ObviousReturn => "Comment obviously states what return statement does",
            CommentPattern::VariableInitialization => "Comment adds no context to variable initialization",
            CommentPattern::LoopDescription => "Comment obviously describes loop construct",
            CommentPattern::ObviousAssignment => "Comment obviously describes assignment",
        }
    }
}

/// Extract comments from a line of code
fn extract_comment(line: &str) -> Option<String> {
    // Handle different comment styles
    if let Some(pos) = line.find("//") {
        let comment = line[pos + 2..].trim();
        if !comment.is_empty() {
            return Some(comment.to_string());
        }
    }
    
    if let Some(pos) = line.find('#') {
        let comment = line[pos + 1..].trim();
        if !comment.is_empty() {
            return Some(comment.to_string());
        }
    }
    
    // Handle /* */ style comments on single lines
    if let Some(start) = line.find("/*") {
        if let Some(end) = line.find("*/") {
            if end > start {
                let comment = line[start + 2..end].trim();
                if !comment.is_empty() {
                    return Some(comment.to_string());
                }
            }
        }
    }
    
    None
}

/// Check if a comment is obvious given the context of the next line
fn is_obvious_comment(comment: &str, next_line: Option<&str>) -> Option<CommentPattern> {
    // Check more specific patterns first, then general ones
    let patterns = [
        CommentPattern::VariableInitialization,
        CommentPattern::ObviousReturn,
        CommentPattern::LoopDescription,
        CommentPattern::ObviousAssignment,
        CommentPattern::CodeTranslation,
    ];

    for pattern in &patterns {
        if pattern.get_regex().is_match(comment) {
            // Additional context-based checking
            if let Some(code) = next_line {
                if is_pattern_match_with_code(pattern, comment, code) {
                    return Some(pattern.clone());
                }
            } else if matches!(pattern, CommentPattern::ObviousReturn | CommentPattern::VariableInitialization) {
                return Some(pattern.clone());
            }
        }
    }

    None
}

/// Check if the comment pattern matches with the actual code
fn is_pattern_match_with_code(pattern: &CommentPattern, comment: &str, code: &str) -> bool {
    let code_lower = code.trim().to_lowercase();
    let _comment_lower = comment.to_lowercase();

    match pattern {
        CommentPattern::ObviousReturn => {
            code_lower.starts_with("return")
        }
        CommentPattern::VariableInitialization => {
            // Check if next line is a variable declaration/initialization
            code_lower.contains('=') && (
                code_lower.contains("let ") || 
                code_lower.contains("var ") || 
                code_lower.contains("const ") ||
                code_lower.contains("auto ") ||
                Regex::new(r"^\s*\w+\s*=").unwrap().is_match(&code_lower)
            )
        }
        CommentPattern::LoopDescription => {
            code_lower.contains("for ") || 
            code_lower.contains("while ") || 
            code_lower.contains("loop")
        }
        CommentPattern::ObviousAssignment => {
            code_lower.contains('=') && !code_lower.contains("==") && !code_lower.contains("!=")
        }
        CommentPattern::CodeTranslation => {
            // This is more generic, could be refined
            true
        }
    }
}

/// Analyze a chunk of code for obvious comments
pub fn analyze_comments(chunk_content: &str) -> CommentAnalysisResult {
    let lines: Vec<&str> = chunk_content.lines().collect();
    let mut obvious_comments = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if let Some(comment) = extract_comment(line) {
            let next_line = if i + 1 < lines.len() {
                Some(lines[i + 1])
            } else {
                None
            };

            if let Some(pattern) = is_obvious_comment(&comment, next_line) {
                obvious_comments.push(ObviousComment {
                    line_number: i + 1,
                    comment_text: comment,
                    reason: pattern.get_reason().to_string(),
                    code_line: next_line.map(|s| s.to_string()),
                });
            }
        }
    }

    CommentAnalysisResult {
        has_violations: !obvious_comments.is_empty(),
        obvious_comments,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_comment_double_slash() {
        let line = "let x = 5; // Set x to 5";
        let comment = extract_comment(line);
        assert_eq!(comment, Some("Set x to 5".to_string()));
    }

    #[test]
    fn test_extract_comment_hash() {
        let line = "x = 5  # Set x to 5";
        let comment = extract_comment(line);
        assert_eq!(comment, Some("Set x to 5".to_string()));
    }

    #[test]
    fn test_extract_comment_block() {
        let line = "let x = 5; /* Set x to 5 */";
        let comment = extract_comment(line);
        assert_eq!(comment, Some("Set x to 5".to_string()));
    }

    #[test]
    fn test_extract_comment_none() {
        let line = "let x = 5;";
        let comment = extract_comment(line);
        assert_eq!(comment, None);
    }

    #[test]
    fn test_obvious_assignment_comment() {
        let chunk = "// Set x to 5\nlet x = 5;";
        let result = analyze_comments(chunk);
        
        assert!(result.has_violations);
        assert_eq!(result.obvious_comments.len(), 1);
        assert_eq!(result.obvious_comments[0].comment_text, "Set x to 5");
        assert!(result.obvious_comments[0].reason.contains("assignment"));
    }

    #[test]
    fn test_obvious_return_comment() {
        let chunk = "// Return true\nreturn true;";
        let result = analyze_comments(chunk);
        
        assert!(result.has_violations);
        assert_eq!(result.obvious_comments.len(), 1);
        assert_eq!(result.obvious_comments[0].comment_text, "Return true");
        assert!(result.obvious_comments[0].reason.contains("return statement"));
    }

    #[test]
    fn test_obvious_loop_comment() {
        let chunk = "// Loop through items\nfor item in items {";
        let result = analyze_comments(chunk);
        
        assert!(result.has_violations);
        assert_eq!(result.obvious_comments.len(), 1);
        assert!(result.obvious_comments[0].reason.contains("loop construct"));
    }

    #[test]
    fn test_good_comment_not_flagged() {
        let chunk = "// Calculate the compound interest using the formula\nlet result = principal * (1 + rate).pow(time);";
        let result = analyze_comments(chunk);
        
        assert!(!result.has_violations);
        assert_eq!(result.obvious_comments.len(), 0);
    }

    #[test]
    fn test_variable_initialization_comment() {
        let chunk = "// Initialize variable\nlet count = 0;";
        let result = analyze_comments(chunk);
        
        assert!(result.has_violations);
        assert_eq!(result.obvious_comments.len(), 1);
        assert!(result.obvious_comments[0].reason.contains("variable initialization"));
    }

    #[test]
    fn test_multiple_obvious_comments() {
        let chunk = "// Set x to 5\nlet x = 5;\n// Return the result\nreturn x;";
        let result = analyze_comments(chunk);
        
        assert!(result.has_violations);
        assert_eq!(result.obvious_comments.len(), 2);
    }

    #[test]
    fn test_mixed_comments() {
        let chunk = "// This calculates the user's age based on birth year\nlet age = current_year - birth_year;\n// Set flag to true\nlet flag = true;";
        let result = analyze_comments(chunk);
        
        assert!(result.has_violations);
        assert_eq!(result.obvious_comments.len(), 1);
        assert_eq!(result.obvious_comments[0].comment_text, "Set flag to true");
    }

    #[test]
    fn test_empty_comments_ignored() {
        let chunk = "//\nlet x = 5;\n/* */\nlet y = 10;";
        let result = analyze_comments(chunk);
        
        assert!(!result.has_violations);
        assert_eq!(result.obvious_comments.len(), 0);
    }
}