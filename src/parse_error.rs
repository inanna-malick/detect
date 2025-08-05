use crate::predicate::{Op, RhsValue, Selector};
use std::fmt;
use thiserror::Error;

use crate::parser::pratt_parser::Rule;

/// Wrapper for pest errors to provide custom formatting
#[derive(Debug)]
pub struct PestError(pub Box<pest::error::Error<Rule>>);

impl fmt::Display for PestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Clone the error to use renamed_rules() which takes ownership
        let user_friendly = self.0.clone().renamed_rules(rule_to_user_friendly);
        write!(f, "{}", user_friendly)?;

        // Add contextual hints based on the error
        if let pest::error::ErrorVariant::ParsingError { positives, .. } = &self.0.variant {
            let bare_selectors = [
                Rule::bare_name,
                Rule::bare_stem,
                Rule::bare_extension,
                Rule::bare_parent,
            ];

            if positives.iter().any(|r| bare_selectors.contains(r)) {
                if let pest::error::LineColLocation::Pos((1, col)) = self.0.line_col {
                    if col <= 10 {
                        write!(f, "\n\nHint: Selectors like 'name', 'stem', 'extension' should be prefixed with 'path.' (e.g., 'path.name', 'path.extension')")?;
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
/// This follows the idiomatic pattern using renamed_rules()
fn rule_to_user_friendly(rule: &Rule) -> String {
    match rule {
        // Operators
        Rule::string_regex => "regex operator (~=)".to_owned(),
        Rule::string_eq => "equals operator (==)".to_owned(),
        Rule::string_ne => "not equals operator (!=)".to_owned(),
        Rule::string_contains => "contains operator".to_owned(),
        Rule::string_in => "'in' operator".to_owned(),
        Rule::eq => "== or =".to_owned(),
        Rule::ne => "!=".to_owned(),
        Rule::in_ => "'in'".to_owned(),
        Rule::gt => ">".to_owned(),
        Rule::gteq => ">=".to_owned(),
        Rule::lt => "<".to_owned(),
        Rule::lteq => "<=".to_owned(),

        // Values
        Rule::bare_string => "unquoted value".to_owned(),
        Rule::quoted_string => "quoted string".to_owned(),
        Rule::string_value => "string value".to_owned(),
        Rule::numeric_value => "numeric value".to_owned(),
        Rule::temporal_value => "time value".to_owned(),
        Rule::set_literal => "set [item1, item2, ...]".to_owned(),

        // Path selectors
        Rule::bare_name => "'name'".to_owned(),
        Rule::bare_stem => "'stem'".to_owned(),
        Rule::bare_extension => "'extension' or 'ext'".to_owned(),
        Rule::bare_parent => "'parent'".to_owned(),
        Rule::bare_full => "'full' or 'path'".to_owned(),
        Rule::path_selector => "path selector (path.name, path.stem, etc.)".to_owned(),

        // Other selectors
        Rule::string_selector => "string selector (name, path, contents, type)".to_owned(),
        Rule::numeric_selector => "numeric selector (size, depth)".to_owned(),
        Rule::temporal_selector => "time selector (modified, created, accessed)".to_owned(),

        // Predicates
        Rule::typed_predicate => "valid expression".to_owned(),
        Rule::string_predicate => "string comparison".to_owned(),
        Rule::numeric_predicate => "numeric comparison".to_owned(),
        Rule::temporal_predicate => "time comparison".to_owned(),

        // Special
        Rule::EOI => "end of expression".to_owned(),
        Rule::WHITESPACE => "whitespace".to_owned(),

        // Default: use lowercase version of the rule name
        _ => format!("{:?}", rule).to_lowercase().replace('_', " "),
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    /// Pest grammar/syntax error - preserves location info
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
        selector: Selector,
        operator: Op,
        value: RhsValue,
        #[source]
        source: PredicateParseError,
    },

    /// Internal parser error - indicates grammar invariant violation
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
    /// Regex compilation failed
    #[error("Invalid regex pattern")]
    Regex(#[from] regex::Error),

    /// Numeric parsing failed
    #[error("Invalid number")]
    Numeric(#[from] std::num::ParseIntError),

    /// Temporal parsing failed
    #[error("Invalid time value")]
    Temporal(#[from] TemporalError),

    /// DFA compilation failed
    #[error("DFA compilation failed: {0}")]
    Dfa(String), // aho_corasick errors don't implement Error trait well

    /// Operator not compatible with selector
    #[error("Incompatible operation: {reason}")]
    IncompatibleOperation { reason: &'static str },

    /// Value type not compatible with selector/operator
    #[error("Expected {expected}, found {found}")]
    IncompatibleValue {
        expected: &'static str,
        found: String,
    },

    /// JSON parsing for set literals
    #[error("Invalid set literal")]
    SetParse(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
#[error("Temporal parse error for '{input}': {kind}")]
pub struct TemporalError {
    pub input: String,
    pub kind: TemporalErrorKind,
}

#[derive(Debug, Error)]
pub enum TemporalErrorKind {
    #[error("Invalid format")]
    InvalidFormat,
    #[error("Unknown time unit: {0}")]
    UnknownUnit(String),
    #[error("Invalid number")]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("Invalid date")]
    InvalidDate(#[from] chrono::ParseError),
}

impl ParseError {
    // =========================================================================
    // Builder methods for consistent error construction
    // =========================================================================

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

    /// Create an invalid token error with location
    pub fn invalid_token_with_location(
        expected: &'static str,
        found: impl Into<String>,
        location: (usize, usize),
    ) -> Self {
        ParseError::Structure {
            kind: StructureErrorKind::InvalidToken {
                expected,
                found: found.into(),
            },
            location: Some(location),
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
