//! Enhanced diagnostic error reporting using Miette
//! 
//! This module provides beautiful, compiler-quality error messages with context,
//! transforming Pest's basic errors into rich diagnostic output.

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

use crate::parse_error::{ParseError, PredicateParseError, StructureErrorKind};
use crate::parser::pratt_parser::Rule;

/// Main diagnostic error type providing rich error reporting
#[derive(Error, Debug, Diagnostic)]
pub enum DetectDiagnostic {
    /// Syntax errors from the Pest parser
    #[error("Syntax error in expression")]
    #[diagnostic(
        code(detect::syntax),
        url(docsrs)
    )]
    Syntax {
        /// The source code being parsed
        #[source_code]
        src: NamedSource<String>,
        
        /// The problematic span in the source
        #[label("unexpected token here")]
        bad_bit: SourceSpan,
        
        /// Additional help text
        #[help]
        help_text: Option<String>,
    },
    
    /// Invalid selector errors with helpful suggestions
    #[error("Invalid selector '{selector}'")]
    #[diagnostic(
        code(detect::invalid_selector)
    )]
    InvalidSelector {
        #[source_code]
        src: NamedSource<String>,
        
        #[label("unknown selector")]
        span: SourceSpan,
        
        selector: String,
        
        #[help]
        suggestion: Option<String>,
    },
    
    /// Regex compilation errors with detailed explanations
    #[error("Invalid regex pattern")]
    #[diagnostic(
        code(detect::regex)
    )]
    RegexError {
        #[source_code]
        src: NamedSource<String>,
        
        #[label("regex compilation failed here")]
        span: SourceSpan,
        
        /// The regex error message
        details: String,
        
        #[help]
        fix_suggestion: Option<String>,
    },
    
    /// Temporal expression errors
    #[error("Invalid time expression")]
    #[diagnostic(
        code(detect::temporal)
    )]
    TemporalError {
        #[source_code]
        src: NamedSource<String>,
        
        #[label("invalid time format")]
        span: SourceSpan,
        
        details: String,
        
        #[help]
        examples: Option<String>,
    },
    
    /// Numeric value errors
    #[error("Invalid numeric value")]
    #[diagnostic(
        code(detect::numeric)
    )]
    NumericError {
        #[source_code]
        src: NamedSource<String>,
        
        #[label("invalid number")]
        span: SourceSpan,
        
        details: String,
    },
    
    /// Operator mismatch errors
    #[error("Incompatible operator for selector")]
    #[diagnostic(
        code(detect::operator_mismatch)
    )]
    OperatorMismatch {
        #[source_code]
        src: NamedSource<String>,
        
        #[label("this selector")]
        selector_span: SourceSpan,
        
        #[label("doesn't support this operator")]
        operator_span: SourceSpan,
        
        details: String,
    },
}

/// Helper to add "did you mean?" suggestions for selectors
fn suggest_selector(invalid: &str) -> Option<String> {
    const VALID_SELECTORS: &[&str] = &[
        "path", "path.name", "path.extension", "path.stem", "path.parent",
        "name", "extension", "stem", "parent",
        "size", "type", "contents", 
        "modified", "created", "accessed",
    ];
    
    // Simple edit distance check
    let mut best_match = None;
    let mut best_distance = usize::MAX;
    
    for &valid in VALID_SELECTORS {
        let distance = levenshtein_distance(invalid, valid);
        if distance < best_distance && distance <= 2 {
            best_distance = distance;
            best_match = Some(valid);
        }
    }
    
    best_match.map(|s| format!("Did you mean '{}'?", s))
}

/// Simple Levenshtein distance implementation
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
    
    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }
    
    for (i, c1) in s1.chars().enumerate() {
        for (j, c2) in s2.chars().enumerate() {
            let cost = if c1 == c2 { 0 } else { 1 };
            matrix[i + 1][j + 1] = (matrix[i][j + 1] + 1)
                .min(matrix[i + 1][j] + 1)
                .min(matrix[i][j] + cost);
        }
    }
    
    matrix[len1][len2]
}

/// Convert a Pest error to a Miette diagnostic
pub fn pest_to_diagnostic(
    pest_err: &pest::error::Error<Rule>,
    source: &str,
    filename: Option<&str>,
) -> DetectDiagnostic {
    let span = match pest_err.location {
        pest::error::InputLocation::Pos(pos) => (pos, 0).into(),
        pest::error::InputLocation::Span((start, end)) => (start, end - start).into(),
    };
    
    let src = if let Some(name) = filename {
        NamedSource::new(name, source.to_string())
    } else {
        NamedSource::new("expression", source.to_string())
    };
    
    // Try to provide more specific error types based on the error
    let help_text = if let pest::error::ErrorVariant::ParsingError { positives, .. } = &pest_err.variant {
        // Check for common patterns and provide helpful hints
        if positives.iter().any(|r| matches!(r, 
            Rule::bare_name | Rule::bare_stem | Rule::bare_extension | Rule::bare_parent
        )) {
            Some("Selectors like 'name', 'stem', 'extension' should be prefixed with 'path.' (e.g., 'path.name')".to_string())
        } else if positives.iter().any(|r| matches!(r, Rule::string_value)) {
            Some("String values can be quoted (\"value\") or unquoted (value) if they don't contain special characters".to_string())
        } else if positives.iter().any(|r| matches!(r, Rule::temporal_value)) {
            Some("Time values can be relative (-7.days), absolute (2024-01-15), or keywords (today, yesterday)".to_string())
        } else {
            None
        }
    } else {
        None
    };
    
    DetectDiagnostic::Syntax {
        src,
        bad_bit: span,
        help_text,
    }
}

/// Convert our ParseError to a Miette diagnostic
pub fn parse_error_to_diagnostic(
    error: &ParseError,
    source: &str,
    filename: Option<&str>,
) -> DetectDiagnostic {
    let src = if let Some(name) = filename {
        NamedSource::new(name, source.to_string())
    } else {
        NamedSource::new("expression", source.to_string())
    };
    
    match error {
        ParseError::Syntax(pest_err) => pest_to_diagnostic(pest_err, source, filename),
        
        ParseError::Structure { kind, location } => {
            let span = if let Some((line, col)) = location {
                // Convert line/col to byte position (approximation)
                let pos = estimate_byte_position(source, *line, *col);
                (pos, 0).into()
            } else {
                (0, source.len()).into()
            };
            
            match kind {
                StructureErrorKind::InvalidSelector { found } => {
                    // Special handling for removed 'suffix' selector
                    let suggestion = if found == "suffix" {
                        Some("'suffix' has been removed. Use 'path.extension' or 'extension' instead.".to_string())
                    } else {
                        suggest_selector(found)
                    };
                    
                    DetectDiagnostic::InvalidSelector {
                        src,
                        span,
                        selector: found.clone(),
                        suggestion,
                    }
                }
                _ => DetectDiagnostic::Syntax {
                    src,
                    bad_bit: span,
                    help_text: Some(format!("{:?}", kind)),
                }
            }
        }
        
        ParseError::Predicate { source: predicate_err, .. } => {
            match predicate_err {
                PredicateParseError::Regex(e) => {
                    let err_str = e.to_string();
                    let fix = if err_str.contains("repetition operator") {
                        Some("Use .* for 'any characters' (not just *). Pattern '*.txt' should be '.*\\.txt'".to_string())
                    } else if err_str.contains("unclosed") {
                        Some("Check for unclosed brackets or groups. Close character classes: [a-z] not [a-z".to_string())
                    } else {
                        Some("Escape special characters: \\., \\(, \\[".to_string())
                    };
                    
                    DetectDiagnostic::RegexError {
                        src,
                        span: (0, source.len()).into(),
                        details: err_str,
                        fix_suggestion: fix,
                    }
                }
                PredicateParseError::Temporal(e) => {
                    DetectDiagnostic::TemporalError {
                        src,
                        span: (0, source.len()).into(),
                        details: e.to_string(),
                        examples: Some("• Relative: -7.days, -30.minutes\n• Absolute: 2024-01-15\n• Keywords: today, yesterday".to_string()),
                    }
                }
                PredicateParseError::Numeric(e) => {
                    DetectDiagnostic::NumericError {
                        src,
                        span: (0, source.len()).into(),
                        details: e.to_string(),
                    }
                }
                PredicateParseError::IncompatibleOperation { reason } => {
                    DetectDiagnostic::OperatorMismatch {
                        src: src.clone(),
                        selector_span: (0, 0).into(),
                        operator_span: (0, source.len()).into(),
                        details: reason.to_string(),
                    }
                }
                _ => DetectDiagnostic::Syntax {
                    src,
                    bad_bit: (0, source.len()).into(),
                    help_text: Some(predicate_err.to_string()),
                }
            }
        }
    }
}

/// Estimate byte position from line/column
fn estimate_byte_position(source: &str, line: usize, col: usize) -> usize {
    let mut current_line = 1;
    let mut last_newline = 0;
    
    for (i, ch) in source.char_indices() {
        if current_line == line {
            return (last_newline + col.saturating_sub(1)).min(source.len());
        }
        if ch == '\n' {
            current_line += 1;
            last_newline = i + 1;
        }
    }
    
    source.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("a", "a"), 0);
        assert_eq!(levenshtein_distance("a", "b"), 1);
        assert_eq!(levenshtein_distance("size", "siz"), 1);
        assert_eq!(levenshtein_distance("stem", "name"), 3);
    }
    
    #[test]
    fn test_suggest_selector() {
        assert_eq!(suggest_selector("siz"), Some("Did you mean 'size'?".to_string()));
        assert_eq!(suggest_selector("nam"), Some("Did you mean 'name'?".to_string()));
        assert_eq!(suggest_selector("modiifed"), None); // too many edits
        assert_eq!(suggest_selector("zzzzz"), None);
    }
}