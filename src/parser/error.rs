use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use super::raw::Rule;

/// Main error type for detect expressions, using miette for diagnostics
#[allow(dead_code)] // Fields are used by miette's derive macros
#[derive(Debug, Clone, Diagnostic, Error)]
pub enum DetectError {
    // Syntax errors from pest
    #[error("Syntax error at line {line}, column {col}")]
    #[diagnostic(code(detect::syntax))]
    Syntax {
        #[source_code]
        src: String,
        #[label("{expected_msg}")]
        span: SourceSpan,
        #[help]
        help: Option<String>,
        expected_msg: String,
        line: usize,
        col: usize,
    },

    // Typechecker errors with spans
    #[error("Unknown selector: {selector}")]
    #[diagnostic(code(detect::unknown_selector), help("Valid selectors: name, basename, ext, path, dir, size, type, depth, modified, created, accessed, content"))]
    UnknownSelector {
        selector: String,
        #[label("unknown selector")]
        span: SourceSpan,
        #[source_code]
        src: String,
    },

    #[error("Invalid {format} selector path: {path}")]
    #[diagnostic(
        code(detect::invalid_structured_path),
        help("Structured selectors use format: {format}:.path.to.field")
    )]
    InvalidStructuredPath {
        format: String,
        path: String,
        #[label("invalid path: {reason}")]
        span: SourceSpan,
        reason: String,
        #[source_code]
        src: String,
    },

    #[error("Unknown structured data format: '{format}'")]
    #[diagnostic(code(detect::unknown_structured_format))]
    UnknownStructuredFormat {
        format: String,
        #[label("unknown format")]
        span: SourceSpan,
        #[source_code]
        src: String,
        #[help]
        suggestions: Option<String>,
    },

    #[error("Unknown operator: {operator}")]
    #[diagnostic(
        code(detect::unknown_operator),
        help("Valid operators include: ==, !=, >, <, contains, matches, etc.")
    )]
    UnknownOperator {
        operator: String,
        #[label("unknown operator")]
        span: SourceSpan,
        #[source_code]
        src: String,
    },

    #[error("Unknown alias: '{word}'")]
    #[diagnostic(code(detect::unknown_alias))]
    UnknownAlias {
        word: String,
        #[label("unknown alias")]
        span: SourceSpan,
        #[source_code]
        src: String,
        #[help]
        suggestions: Option<String>,
    },

    #[error("Operator '{operator}' is not compatible with selector '{selector}'")]
    #[diagnostic(
        code(detect::incompatible_operator),
        help("This selector requires a different type of operator")
    )]
    IncompatibleOperator {
        selector: String,
        operator: String,
        #[label("incompatible operator")]
        operator_span: SourceSpan,
        #[label("for this selector")]
        selector_span: SourceSpan,
        #[source_code]
        src: String,
    },

    #[error("Expected {expected} value, found: {found}")]
    #[diagnostic(
        code(detect::invalid_value),
        help("Check the value type for this selector")
    )]
    InvalidValue {
        expected: String,
        found: String,
        #[label("invalid value")]
        span: SourceSpan,
        #[source_code]
        src: String,
    },

    // Escape errors
    #[error("Invalid escape sequence '\\{char}'")]
    #[diagnostic(
        code(detect::invalid_escape),
        help("Valid escape sequences: \\n, \\t, \\\\, \\\", \\'")
    )]
    InvalidEscape {
        char: char,
        #[label("invalid escape")]
        span: SourceSpan,
        #[source_code]
        src: String,
    },

    #[error("Unterminated escape sequence")]
    #[diagnostic(code(detect::unterminated_escape))]
    UnterminatedEscape {
        #[label("escape sequence not completed")]
        span: SourceSpan,
        #[source_code]
        src: String,
    },

    // Quote errors
    #[error("Unterminated string literal")]
    #[diagnostic(code(detect::unterminated_string))]
    UnterminatedString {
        #[label("missing closing {quote} quote")]
        span: SourceSpan,
        quote: char,
        #[source_code]
        src: String,
    },

    #[error("Stray {quote} quote")]
    #[diagnostic(
        code(detect::stray_quote),
        help("Remove the quote or add matching opening quote")
    )]
    StrayQuote {
        #[label("unexpected quote")]
        span: SourceSpan,
        quote: char,
        #[source_code]
        src: String,
    },

    // Filesystem errors
    #[error("Directory not found: {path}")]
    #[diagnostic(
        code(detect::directory_not_found),
        help("Check that the directory path exists and is accessible")
    )]
    DirectoryNotFound { path: String },

    #[error("Path is not a directory: {path}")]
    #[diagnostic(
        code(detect::not_a_directory),
        help("The path must be a directory, not a file")
    )]
    NotADirectory { path: String },

    // I/O errors
    #[error("I/O error: {message}")]
    #[diagnostic(code(detect::io_error))]
    IoError { message: String },

    // Internal errors
    #[error("Internal parser error: {message}")]
    #[diagnostic(code(detect::internal))]
    Internal {
        message: String,
        #[source_code]
        src: String,
    },
}

// Extension trait for span location extraction
pub trait SpanExt {
    fn to_location(&self) -> (usize, usize);
    fn to_source_span(&self) -> SourceSpan;
}

impl SpanExt for pest::Span<'_> {
    #[inline]
    fn to_location(&self) -> (usize, usize) {
        self.start_pos().line_col()
    }

    #[inline]
    fn to_source_span(&self) -> SourceSpan {
        (self.start(), self.end() - self.start()).into()
    }
}

/// Convert pest Rule enum to user-friendly names
fn rule_to_friendly_name(rule: &Rule) -> &'static str {
    match rule {
        Rule::program => "program",
        Rule::expr => "expression",
        Rule::infix => "operator (AND/OR)",
        Rule::and => "AND",
        Rule::or => "OR",
        Rule::prefix => "prefix operator (NOT)",
        Rule::neg => "NOT",
        Rule::primary => "predicate or expression",
        Rule::predicate => "predicate",
        Rule::selector => "selector",
        Rule::operator => "operator",
        Rule::value => "value",
        Rule::value_content => "value",
        Rule::raw_token => "value",
        Rule::quoted_string => "quoted string",
        Rule::unterminated_string => "unterminated string",
        Rule::trailing_quote => "trailing quote",
        Rule::single_word => "single-word alias",
        Rule::set_contents => "set contents",
        Rule::set_items => "set items",
        Rule::set_item => "set item",
        Rule::bare_set_item => "item",
        Rule::inner_double => "string content",
        Rule::inner_single => "string content",
        Rule::escaped => "escape sequence",
        Rule::raw_char => "character",
        Rule::balanced_paren => "balanced parentheses",
        Rule::balanced_bracket => "balanced brackets",
        Rule::balanced_curly => "balanced braces",
        Rule::WHITESPACE => "whitespace",
        Rule::EOI => "end of input",
    }
}

/// Generate contextual help text based on error patterns
fn generate_help_text(positives: &[Rule], found_eoi: bool) -> Option<String> {
    if positives.is_empty() {
        return None;
    }

    // Check for common patterns
    if positives.contains(&Rule::value) {
        if found_eoi {
            return Some("Try adding a value after the operator, like: ext == rs".to_string());
        }
        return Some("Expected a value here (e.g., a string, number, or [set])".to_string());
    }

    if (positives.contains(&Rule::expr) || positives.contains(&Rule::predicate)) && found_eoi {
        return Some("Expression is incomplete. Add a predicate after the operator.".to_string());
    }

    if positives.contains(&Rule::EOI) {
        return Some("Unexpected input. Check for unbalanced parentheses or quotes.".to_string());
    }

    None
}

impl DetectError {
    /// Create a syntax error from pest error with diagnostic information
    pub fn from_pest(pest_err: Box<pest::error::Error<Rule>>, src: String) -> Self {
        use pest::error::{ErrorVariant, InputLocation};

        // Extract position information with non-zero width for miette arrow rendering
        let (span, _pos) = match pest_err.location {
            InputLocation::Pos(pos) => {
                // For point locations, ensure non-zero width for miette arrow
                // If at/past EOI, point backwards at last char; otherwise point at current position
                if pos >= src.len() && pos > 0 {
                    ((pos - 1, 1).into(), pos)
                } else if pos < src.len() {
                    ((pos, 1).into(), pos)
                } else {
                    // Empty input
                    ((0, 0).into(), pos)
                }
            }
            InputLocation::Span((start, end)) => {
                let width = end.saturating_sub(start).max(1); // Ensure at least width 1
                ((start, width).into(), start)
            }
        };

        // Get line and column
        let (line, col) = match pest_err.line_col {
            pest::error::LineColLocation::Pos((line, col)) => (line, col),
            pest::error::LineColLocation::Span((line, col), _) => (line, col),
        };

        // Extract expected tokens and generate user-friendly message
        let (expected_msg, help) = match &pest_err.variant {
            ErrorVariant::ParsingError {
                positives,
                negatives: _,
            } => {
                let found_eoi = match pest_err.location {
                    InputLocation::Pos(p) => p >= src.len(),
                    InputLocation::Span((_, end)) => end >= src.len(),
                };

                let expected_msg = if positives.is_empty() {
                    "Unexpected input".to_string()
                } else if positives.len() == 1 {
                    format!("Expected {}", rule_to_friendly_name(&positives[0]))
                } else {
                    let names: Vec<&str> = positives.iter().map(rule_to_friendly_name).collect();
                    if names.len() <= 3 {
                        format!("Expected one of: {}", names.join(", "))
                    } else {
                        format!("Expected one of: {}, ...", names[..3].join(", "))
                    }
                };

                let help = generate_help_text(positives, found_eoi);
                (expected_msg, help)
            }
            ErrorVariant::CustomError { message } => (message.clone(), None),
        };

        DetectError::Syntax {
            src,
            span,
            help,
            expected_msg,
            line,
            col,
        }
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        DetectError::Internal {
            message: msg.into(),
            src: String::new(),
        }
    }

    /// Add source code to the error

    pub fn with_source(mut self, src: String) -> Self {
        match &mut self {
            DetectError::Syntax { src: s, .. }
            | DetectError::UnknownSelector { src: s, .. }
            | DetectError::InvalidStructuredPath { src: s, .. }
            | DetectError::UnknownStructuredFormat { src: s, .. }
            | DetectError::UnknownOperator { src: s, .. }
            | DetectError::UnknownAlias { src: s, .. }
            | DetectError::IncompatibleOperator { src: s, .. }
            | DetectError::InvalidValue { src: s, .. }
            | DetectError::InvalidEscape { src: s, .. }
            | DetectError::UnterminatedEscape { src: s, .. }
            | DetectError::UnterminatedString { src: s, .. }
            | DetectError::StrayQuote { src: s, .. }
            | DetectError::Internal { src: s, .. } => {
                *s = src;
            }
            // Filesystem and I/O errors don't have source code
            DetectError::DirectoryNotFound { .. }
            | DetectError::NotADirectory { .. }
            | DetectError::IoError { .. } => {}
        }
        self
    }
}
