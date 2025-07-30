use std::fmt;
use crate::predicate::{Selector, Op};

use crate::parser::pratt_parser::Rule;

#[derive(Debug)]
pub enum ParseError {
    /// Pest grammar/syntax error - preserves location info
    Syntax(pest::error::Error<Rule>),
    
    /// Structural errors during AST construction
    Structure {
        kind: StructureErrorKind,
        location: Option<(usize, usize)>,
    },
    
    /// Errors from predicate parsing
    Predicate {
        selector: Selector,
        operator: Op,
        value: String,
        source: PredicateParseError,
    },
}

#[derive(Debug)]
pub enum StructureErrorKind {
    InvalidSelector { found: String },
    MissingToken { expected: &'static str, context: &'static str },
    UnexpectedRule { rule: Rule },
}

#[derive(Debug)]
pub enum PredicateParseError {
    /// Regex compilation failed
    Regex(regex::Error),
    
    /// Numeric parsing failed
    Numeric(std::num::ParseIntError),
    
    /// Temporal parsing failed
    Temporal(TemporalError),
    
    /// DFA compilation failed
    Dfa(String), // aho_corasick errors don't implement Error trait well
    
    /// Operator not compatible with selector
    IncompatibleOperation { reason: &'static str },
    
    /// JSON parsing for set literals
    SetParse(serde_json::Error),
}

#[derive(Debug)]
pub struct TemporalError {
    pub input: String,
    pub kind: TemporalErrorKind,
}

#[derive(Debug)]
pub enum TemporalErrorKind {
    InvalidFormat,
    UnknownUnit(String),
    ParseInt(std::num::ParseIntError),
    InvalidDate(chrono::ParseError),
}

impl ParseError {
    /// Get location info if available
    pub fn location(&self) -> Option<(usize, usize)> {
        match self {
            ParseError::Syntax(e) => match e.line_col {
                pest::error::LineColLocation::Pos((line, col)) => Some((line, col)),
                pest::error::LineColLocation::Span((line, col), _) => Some((line, col)),
            },
            ParseError::Structure { location, .. } => *location,
            _ => None,
        }
    }
    
    /// Get contextual hint for fixing the error
    pub fn hint(&self) -> Option<String> {
        match self {
            ParseError::Predicate { selector, operator, source, .. } => {
                match source {
                    PredicateParseError::Regex(_) => Some(
                        "Regex syntax error. Common issues:\n\
                         • Use .* instead of * for wildcard\n\
                         • Escape special characters: \\., \\(, \\[\n\
                         • Check regex syntax at regex101.com".to_string()
                    ),
                    PredicateParseError::Temporal(_) => Some(format!(
                        "Time format examples:\n\
                         • Relative: -7.days, -1.hour, -30.minutes\n\
                         • Absolute: 2024-01-15, today, yesterday\n\
                         • Selector {:?} with operator {:?} expects temporal values", 
                        selector, operator
                    )),
                    PredicateParseError::IncompatibleOperation { reason } => {
                        Some(format!("{}\nValid operators for {:?} depend on the value type", 
                            reason, selector))
                    },
                    PredicateParseError::Numeric(_) => Some(
                        "Expected a numeric value (positive integer)".to_string()
                    ),
                    _ => None,
                }
            },
            ParseError::Structure { kind: StructureErrorKind::InvalidSelector { found }, .. } => {
                Some(format!(
                    "Unknown selector '{}'. Valid selectors:\n\
                     • name (alias: filename), path (alias: filepath)\n\
                     • ext (alias: extension), size (alias: filesize)\n\
                     • type (alias: filetype), contents (alias: file)\n\
                     • modified (alias: mtime), created (alias: ctime), accessed (alias: atime)",
                    found
                ))
            },
            _ => None,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Syntax(e) => {
                // Pest already has good error formatting with location markers
                write!(f, "{}", e)
            },
            ParseError::Structure { kind, .. } => {
                write!(f, "{}", kind)
            },
            ParseError::Predicate { selector, operator, value, source } => {
                write!(f, "Invalid {} for @{:?} {:?} {}: {}", 
                    source.value_description(), selector, operator, value, source)
            },
        }
    }
}

impl fmt::Display for StructureErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StructureErrorKind::InvalidSelector { found } => {
                write!(f, "Invalid selector: @{}", found)
            },
            StructureErrorKind::MissingToken { expected, context } => {
                write!(f, "Expected {} in {}", expected, context)
            },
            StructureErrorKind::UnexpectedRule { rule } => {
                write!(f, "Unexpected grammar rule: {:?}", rule)
            },
        }
    }
}

impl PredicateParseError {
    pub fn value_description(&self) -> &'static str {
        match self {
            PredicateParseError::Regex(_) => "regex pattern",
            PredicateParseError::Numeric(_) => "numeric value",
            PredicateParseError::Temporal(_) => "time value",
            PredicateParseError::Dfa(_) => "content pattern",
            PredicateParseError::IncompatibleOperation { .. } => "operation",
            PredicateParseError::SetParse(_) => "set literal",
        }
    }
}

impl fmt::Display for PredicateParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PredicateParseError::Regex(e) => write!(f, "{}", e),
            PredicateParseError::Numeric(e) => write!(f, "{}", e),
            PredicateParseError::Temporal(e) => write!(f, "{}", e),
            PredicateParseError::Dfa(e) => write!(f, "DFA compilation failed: {}", e),
            PredicateParseError::IncompatibleOperation { reason } => write!(f, "{}", reason),
            PredicateParseError::SetParse(e) => write!(f, "Invalid set literal: {}", e),
        }
    }
}

impl fmt::Display for TemporalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl fmt::Display for TemporalErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemporalErrorKind::InvalidFormat => write!(f, "Invalid time format"),
            TemporalErrorKind::UnknownUnit(unit) => write!(f, "Unknown time unit: '{}'", unit),
            TemporalErrorKind::ParseInt(e) => write!(f, "Invalid number: {}", e),
            TemporalErrorKind::InvalidDate(e) => write!(f, "Invalid date: {}", e),
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::Syntax(e) => Some(e),
            ParseError::Predicate { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl std::error::Error for PredicateParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PredicateParseError::Regex(e) => Some(e),
            PredicateParseError::Numeric(e) => Some(e),
            PredicateParseError::Temporal(e) => Some(e),
            PredicateParseError::SetParse(e) => Some(e),
            _ => None,
        }
    }
}

impl std::error::Error for TemporalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            TemporalErrorKind::ParseInt(e) => Some(e),
            TemporalErrorKind::InvalidDate(e) => Some(e),
            _ => None,
        }
    }
}

// Conversion from underlying error types
impl From<regex::Error> for PredicateParseError {
    fn from(e: regex::Error) -> Self {
        PredicateParseError::Regex(e)
    }
}

impl From<std::num::ParseIntError> for PredicateParseError {
    fn from(e: std::num::ParseIntError) -> Self {
        PredicateParseError::Numeric(e)
    }
}

impl From<serde_json::Error> for PredicateParseError {
    fn from(e: serde_json::Error) -> Self {
        PredicateParseError::SetParse(e)
    }
}

impl From<TemporalError> for PredicateParseError {
    fn from(e: TemporalError) -> Self {
        PredicateParseError::Temporal(e)
    }
}