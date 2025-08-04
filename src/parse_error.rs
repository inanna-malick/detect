use crate::predicate::{Op, RhsValue, Selector};
use std::fmt;

use crate::parser::pratt_parser::Rule;

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
            ParseError::Syntax(_e) => {
                // Syntax errors get their hints added in the Display impl
                // via renamed_rules() and contextual analysis
                None
            }
            ParseError::Predicate {
                selector,
                operator,
                source,
                ..
            } => match source {
                PredicateParseError::Regex(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("repetition operator") || err_str.contains("quantifier") {
                        Some(
                            "Regex error: Invalid use of * quantifier.\n\n\
                             Common fixes:\n\
                             • Use .* for 'any characters' (not just *)\n\
                             • Use \\* to match literal asterisk\n\
                             • Pattern '*.txt' should be '.*\\.txt'\n\n\
                             Examples:\n\
                             • contents ~= \".*TODO.*\"   # Find TODO anywhere\n\
                             • path.name ~= \"test.*\"     # Files starting with 'test'"
                                .to_string(),
                        )
                    } else if err_str.contains("unclosed") || err_str.contains("unmatched") {
                        Some(
                            "Regex error: Unclosed group or character class.\n\n\
                             Common fixes:\n\
                             • Close character classes: [a-z] not [a-z\n\
                             • Close groups: (abc) not (abc\n\
                             • Escape brackets: \\[ and \\] for literal brackets\n\n\
                             Test your regex at: https://regex101.com/"
                                .to_string(),
                        )
                    } else {
                        Some(
                            "Regex syntax error. Common issues:\n\
                                 • Use .* instead of * for wildcard\n\
                                 • Escape special characters: \\., \\(, \\[\n\
                                 • Check regex syntax at regex101.com"
                                .to_string(),
                        )
                    }
                }
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
            } if found == "suffix" => Some(
                "'suffix' has been removed. Use 'path.extension' or 'extension' instead.\n\n\
                 Examples:\n\
                 • path.extension == rs     # Rust files\n\
                 • extension in [js, ts]    # JavaScript/TypeScript files"
                    .to_string(),
            ),
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
                // Clone the error to use renamed_rules() which takes ownership
                // This is the idiomatic pattern from the Rust community
                let user_friendly = e.clone().renamed_rules(rule_to_user_friendly);

                // Format the error with Pest's built-in formatting
                write!(f, "{}", user_friendly)?;

                // Add contextual hints based on the error
                if let pest::error::ErrorVariant::ParsingError { positives, .. } = &e.variant {
                    // Check if it looks like a bare selector without path prefix
                    let bare_selectors = [
                        Rule::bare_name,
                        Rule::bare_stem,
                        Rule::bare_extension,
                        Rule::bare_parent,
                    ];

                    if positives.iter().any(|r| bare_selectors.contains(r)) {
                        // Check if we're at the start of the expression
                        if let pest::error::LineColLocation::Pos((1, col)) = e.line_col {
                            if col <= 10 {
                                write!(f, "\n\nHint: Selectors like 'name', 'stem', 'extension' should be prefixed with 'path.' (e.g., 'path.name', 'path.extension')")?;
                            }
                        }
                    }
                }

                Ok(())
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
