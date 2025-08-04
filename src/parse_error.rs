use crate::predicate::{Op, RhsValue, Selector};
use std::fmt;

use crate::parser::pratt_parser::Rule;

/// Improve Pest error messages by translating rule names to user-friendly descriptions
fn improve_pest_error_message(error: &str) -> String {
    // Common rule translations
    let error = error
        .replace("string_regex", "regex operator (~=)")
        .replace("string_eq", "equals operator (==)")
        .replace("string_ne", "not equals operator (!=)")
        .replace("string_contains", "contains operator")
        .replace("string_in", "in operator")
        .replace("eq", "== or =")
        .replace("ne", "!=")
        .replace("in_", "'in'")
        .replace("bare_string", "unquoted value")
        .replace("quoted_string", "quoted value")
        .replace("bare_name", "'name'")
        .replace("bare_stem", "'stem'")
        .replace("bare_extension", "'extension' or 'ext'")
        .replace("bare_parent", "'parent'")
        .replace("bare_full", "'full'")
        .replace("string_selector", "string selector (name, path, etc.)")
        .replace("numeric_selector", "numeric selector (size, depth)")
        .replace(
            "temporal_selector",
            "time selector (modified, created, accessed)",
        )
        .replace("typed_predicate", "valid expression");

    // If the error mentions expected rules, add a hint
    if error.contains("expected") && error.contains("at line") {
        // Check if it looks like a selector at the start
        if let Some(pos) = error.find(" at line 1, column") {
            let before = &error[..pos];
            if before.contains("expected") && !before.contains("operator") {
                return format!("{}\n\nHint: Did you forget to use a path prefix? Try 'path.name', 'path.extension', etc.", error);
            }
        }
    }

    error
}

#[derive(Debug)]
pub enum ParseError {
    /// Pest grammar/syntax error - preserves location info
    Syntax(Box<pest::error::Error<Rule>>),

    /// Structural errors during AST construction
    Structure {
        kind: StructureErrorKind,
        location: Option<(usize, usize)>,
    },

    /// Errors from predicate parsing
    Predicate {
        selector: Selector,
        operator: Op,
        value: RhsValue,
        source: PredicateParseError,
    },
}

#[derive(Debug)]
pub enum StructureErrorKind {
    InvalidSelector {
        found: String,
    },
    MissingToken {
        expected: &'static str,
        context: &'static str,
    },
    UnexpectedRule {
        rule: Rule,
    },
    InvalidToken {
        expected: &'static str,
        found: String,
    },
    ExpectedRule {
        expected: Rule,
        found: Rule,
    },
    ExpectedOneOf {
        expected: &'static [Rule],
        found: Rule,
    },
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

    /// Value type not compatible with selector/operator
    IncompatibleValue {
        expected: &'static str,
        found: String,
    },

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
    // =========================================================================
    // Builder methods for consistent error construction
    // =========================================================================

    /// Create a missing token error
    pub fn missing_token(expected: &'static str, context: &'static str) -> Self {
        ParseError::Structure {
            kind: StructureErrorKind::MissingToken { expected, context },
            location: None,
        }
    }

    /// Create an unexpected rule error
    pub fn unexpected_rule(rule: Rule, location: Option<(usize, usize)>) -> Self {
        ParseError::Structure {
            kind: StructureErrorKind::UnexpectedRule { rule },
            location,
        }
    }

    /// Create an invalid token error
    pub fn invalid_token(expected: &'static str, found: impl Into<String>) -> Self {
        ParseError::Structure {
            kind: StructureErrorKind::InvalidToken {
                expected,
                found: found.into(),
            },
            location: None,
        }
    }

    /// Create an invalid selector error
    pub fn invalid_selector(found: impl Into<String>) -> Self {
        ParseError::Structure {
            kind: StructureErrorKind::InvalidSelector {
                found: found.into(),
            },
            location: None,
        }
    }

    /// Create an expected rule error
    pub fn expected_rule(expected: Rule, found: Rule) -> Self {
        ParseError::Structure {
            kind: StructureErrorKind::ExpectedRule { expected, found },
            location: None,
        }
    }

    /// Create an expected one of rules error
    pub fn expected_one_of(expected: &'static [Rule], found: Rule) -> Self {
        ParseError::Structure {
            kind: StructureErrorKind::ExpectedOneOf { expected, found },
            location: None,
        }
    }

    /// Add location information to an error
    pub fn with_location(mut self, location: (usize, usize)) -> Self {
        if let ParseError::Structure {
            location: ref mut loc,
            ..
        } = self
        {
            *loc = Some(location);
        }
        self
    }

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
            ParseError::Predicate {
                selector,
                operator,
                source,
                ..
            } => match source {
                PredicateParseError::Regex(_) => Some(
                    "Regex syntax error. Common issues:\n\
                         • Use .* instead of * for wildcard\n\
                         • Escape special characters: \\., \\(, \\[\n\
                         • Check regex syntax at regex101.com"
                        .to_string(),
                ),
                PredicateParseError::Temporal(_) => Some(format!(
                    "Time format examples:\n\
                         • Relative: -7.days, -1.hour, -30.minutes\n\
                         • Absolute: 2024-01-15, today, yesterday\n\
                         • Selector {:?} with operator {:?} expects temporal values",
                    selector, operator
                )),
                PredicateParseError::IncompatibleOperation { reason } => Some(format!(
                    "{}\nValid operators for {:?} depend on the value type",
                    reason, selector
                )),
                PredicateParseError::Numeric(_) => {
                    Some("Expected a numeric value (positive integer)".to_string())
                }
                _ => None,
            },
            ParseError::Structure {
                kind: StructureErrorKind::InvalidSelector { found },
                ..
            } => Some(format!(
                "Unknown selector '{}'. Valid selectors:\n\
                     • path (or path.full) - complete file path\n\
                     • path.parent - directory containing file\n\
                     • path.name - filename with extension\n\
                     • path.stem - filename without extension\n\
                     • path.extension - file extension\n\
                     • type - file, dir, or symlink\n\
                     • contents - search file contents\n\
                     • size - file size (supports KB/MB/GB)\n\
                     • modified, created, accessed - time selectors",
                found
            )),
            _ => None,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Syntax(e) => {
                // Improve Pest's error messages by translating rule names
                let error_str = e.to_string();
                let improved = improve_pest_error_message(&error_str);
                write!(f, "{}", improved)
            }
            ParseError::Structure { kind, .. } => {
                write!(f, "{}", kind)
            }
            ParseError::Predicate {
                selector,
                operator,
                value,
                source,
            } => {
                write!(
                    f,
                    "Invalid {} for @{:?} {:?} {:?}: {}",
                    source.value_description(),
                    selector,
                    operator,
                    value,
                    source
                )
            }
        }
    }
}

impl fmt::Display for StructureErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StructureErrorKind::InvalidSelector { found } => {
                write!(f, "Invalid selector: @{}", found)
            }
            StructureErrorKind::MissingToken { expected, context } => {
                write!(f, "Expected {} in {}", expected, context)
            }
            StructureErrorKind::UnexpectedRule { rule } => {
                write!(f, "Unexpected grammar rule: {:?}", rule)
            }
            StructureErrorKind::InvalidToken { expected, found } => {
                write!(f, "Invalid token: expected {}, found '{}'", expected, found)
            }
            StructureErrorKind::ExpectedRule { expected, found } => {
                write!(f, "Expected {:?}, found {:?}", expected, found)
            }
            StructureErrorKind::ExpectedOneOf { expected, found } => {
                let rules: Vec<String> = expected.iter().map(|r| format!("{:?}", r)).collect();
                write!(
                    f,
                    "Expected one of [{}], found {:?}",
                    rules.join(", "),
                    found
                )
            }
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
            PredicateParseError::IncompatibleValue { .. } => "value type",
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
            PredicateParseError::IncompatibleValue { expected, found } => {
                write!(f, "Type mismatch: expected {}, found {}", expected, found)
            }
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
