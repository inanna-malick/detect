use crate::predicate::{Op, RhsValue};
use std::fmt;
use thiserror::Error;

use crate::parser::pratt_parser::Rule;

/// Wrapper for pest errors to provide custom formatting
#[derive(Debug)]
pub struct PestError(pub Box<pest::error::Error<Rule>>);

impl fmt::Display for PestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let user_friendly = self.0.clone().renamed_rules(rule_to_user_friendly);
        write!(f, "{}", user_friendly)?;

        // Add hint for bare selectors
        if let pest::error::ErrorVariant::ParsingError { positives, .. } = &self.0.variant {
            if positives.iter().any(|r| {
                matches!(
                    r,
                    Rule::bare_name | Rule::bare_extension | Rule::bare_parent
                )
            }) {
                if let pest::error::LineColLocation::Pos((1, col)) = self.0.line_col {
                    if col <= 10 {
                        write!(f, "\n\nHint: Selectors should be prefixed with 'path.' (e.g., 'path.name')")?;
                    }
                }
            }
        }
        Ok(())
    }
}

impl std::error::Error for PestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.0)
    }
}

/// Transform Pest rule names to user-friendly descriptions
fn rule_to_user_friendly(rule: &Rule) -> String {
    match rule {
        // Operators
        Rule::string_regex => "regex operator (~=)",
        Rule::string_eq | Rule::eq => "equals (==)",
        Rule::string_ne | Rule::ne => "not equals (!=)",
        Rule::string_contains => "contains",
        Rule::string_in | Rule::in_ => "in [...]",
        Rule::gt => ">",
        Rule::gteq => ">=",
        Rule::lt => "<",
        Rule::lteq => "<=",

        // Values
        Rule::bare_string => "unquoted value",
        Rule::quoted_string => "quoted string",
        Rule::string_value => "string value",
        Rule::numeric_value => "numeric value",
        Rule::temporal_value => "time value",
        Rule::set_literal => "set [item1, item2, ...]",

        // Selectors
        Rule::bare_name => "'name' or 'stem'",
        Rule::bare_filename => "'filename'",
        Rule::bare_extension => "'extension' or 'ext'",
        Rule::bare_parent => "'parent'",
        Rule::bare_full => "'full' or 'path'",
        Rule::path_selector => "path selector",
        Rule::string_selector => "string selector",
        Rule::numeric_selector => "numeric selector",
        Rule::temporal_selector => "time selector",

        // Predicates
        Rule::typed_predicate => "expression",
        Rule::string_predicate => "string comparison",
        Rule::numeric_predicate => "numeric comparison",
        Rule::temporal_predicate => "time comparison",

        // Special
        Rule::EOI => "end of expression",
        _ => return format!("{:?}", rule).to_lowercase().replace('_', " "),
    }
    .to_owned()
}

#[derive(Debug, Error)]
pub enum ParseError {
    /// Pest grammar/syntax error
    #[error(transparent)]
    Syntax(#[from] PestError),

    /// Structural errors during AST construction
    #[error("{kind}")]
    Structure {
        kind: StructureErrorKind,
        location: Option<(usize, usize)>,
    },

    /// Errors from predicate parsing
    #[error("Invalid {source} for {selector} {operator} {value}")]
    Predicate {
        selector: &'static str,
        operator: Op,
        value: RhsValue,
        #[source]
        source: PredicateParseError,
    },

    /// Internal parser error
    #[error("Internal parser error: {0}")]
    Internal(&'static str),
}

#[derive(Debug, Error)]
pub enum StructureErrorKind {
    #[error("Invalid selector: {found}")]
    InvalidSelector { found: String },
    #[error("Unexpected rule: {rule:?}")]
    UnexpectedRule { rule: Rule },
    #[error("Expected {expected}, found '{found}'")]
    InvalidToken {
        expected: &'static str,
        found: String,
    },
}

#[derive(Debug, Error)]
pub enum PredicateParseError {
    #[error("Invalid regex pattern")]
    Regex(#[from] regex::Error),

    #[error("Invalid number")]
    Numeric(#[from] std::num::ParseIntError),

    #[error("Invalid time: {0}")]
    Temporal(String),

    #[error("DFA compilation failed: {0}")]
    Dfa(String),

    #[error("Incompatible: {0}")]
    Incompatible(String),

    #[error("Invalid set literal")]
    SetParse(#[from] serde_json::Error),

    #[error("Unknown selector: {0}")]
    UnknownSelector(String),

    #[error("Unknown operator: {0}")]
    UnknownOperator(String),

    #[error("Operator '{operator}' is not compatible with selector '{selector}'")]
    IncompatibleOperator { selector: String, operator: String },

    #[error("Expected {expected} value, found: {found}")]
    InvalidValue { expected: String, found: String },

    #[error("Internal error: {0}")]
    Internal(String),
}

impl ParseError {
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
            ParseError::Syntax(e) => match e.0.line_col {
                pest::error::LineColLocation::Pos((line, col)) => Some((line, col)),
                pest::error::LineColLocation::Span((line, col), _) => Some((line, col)),
            },
            ParseError::Structure { location, .. } => *location,
            _ => None,
        }
    }
}

use std::sync::Arc;

/// Structured error type for detect operations
#[derive(Debug, Error)]
pub enum DetectError {
    /// Parse error with the input expression and optional source
    #[error("{}", .error)]
    ParseError {
        /// The original parse error
        #[source]
        error: ParseError,
        /// The original source text for diagnostic display
        source: Option<Arc<str>>,
    },
    /// Any other error (gradual migration path)
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// Note: We don't implement From<DetectError> for anyhow::Error because
// anyhow already has a blanket impl for all Error types.
// Use anyhow::Error::new(detect_error) or detect_error.into() as needed.

impl From<ParseError> for DetectError {
    fn from(err: ParseError) -> Self {
        DetectError::ParseError {
            error: err,
            source: None,
        }
    }
}

impl DetectError {
    /// Attach source text to a parse error for diagnostic display
    pub fn with_source(mut self, source: impl Into<Arc<str>>) -> Self {
        if let DetectError::ParseError {
            source: ref mut src,
            ..
        } = self
        {
            *src = Some(source.into());
        }
        self
    }

    /// Create a parse error with source text
    pub fn parse_with_source(error: ParseError, source: impl Into<Arc<str>>) -> Self {
        DetectError::ParseError {
            error,
            source: Some(source.into()),
        }
    }
}
