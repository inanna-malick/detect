use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use super::parser::Rule;

/// Main error type for detect expressions, using miette for beautiful diagnostics
#[derive(Debug, Clone, Diagnostic, Error)]
pub enum DetectError {
    // Syntax errors from pest
    #[error("Syntax error in expression")]
    #[diagnostic(code(detect::syntax), help("Check your expression syntax"))]
    Syntax {
        #[label("error occurred here")]
        span: Option<SourceSpan>,
        message: String,
        #[source_code]
        src: String,
    },

    // Typechecker errors with spans
    #[error("Unknown selector: {selector}")]
    #[diagnostic(code(detect::unknown_selector), help("Valid selectors include: name, path, ext, size, modified, etc."))]
    UnknownSelector {
        selector: String,
        #[label("unknown selector")]
        span: SourceSpan,
        #[source_code]
        src: String,
    },
    
    #[error("Unknown operator: {operator}")]
    #[diagnostic(code(detect::unknown_operator), help("Valid operators include: ==, !=, >, <, contains, matches, etc."))]
    UnknownOperator {
        operator: String,
        #[label("unknown operator")]
        span: SourceSpan,
        #[source_code]
        src: String,
    },
    
    #[error("Operator '{operator}' is not compatible with selector '{selector}'")]
    #[diagnostic(code(detect::incompatible_operator), help("This selector requires a different type of operator"))]
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
    #[diagnostic(code(detect::invalid_value), help("Check the value type for this selector"))]
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
    #[diagnostic(code(detect::invalid_escape), help("Valid escape sequences: \\n, \\t, \\\\, \\\", \\'"))]
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

    // Internal errors
    #[error("Internal parser error: {message}")]
    #[diagnostic(code(detect::internal))]
    Internal {
        message: String,
        #[source_code]
        src: String,
    },
}

// Legacy type alias for compatibility during migration
pub type RawParseError = DetectError;

// Implement conversion from anyhow errors for compatibility
impl From<anyhow::Error> for DetectError {
    fn from(err: anyhow::Error) -> Self {
        DetectError::Internal {
            message: err.to_string(),
            src: String::new(),
        }
    }
}

// Extension trait for cleaner span location extraction
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

impl DetectError {
    /// Create a syntax error from pest error
    pub fn from_pest(pest_err: Box<pest::error::Error<Rule>>, src: String) -> Self {
        let span = match pest_err.location {
            pest::error::InputLocation::Pos(pos) => Some((pos, 0).into()),
            pest::error::InputLocation::Span((start, end)) => Some((start, end - start).into()),
        };
        
        DetectError::Syntax {
            span,
            message: pest_err.to_string(),
            src,
        }
    }

    /// Create an invalid escape error
    pub fn invalid_escape(char: char, position: usize) -> Self {
        // This is a legacy constructor, we'll need the source to create proper error
        // For now, create without source
        DetectError::InvalidEscape {
            char,
            span: (position, 1).into(),
            src: String::new(),
        }
    }

    /// Create an unterminated escape error
    pub fn unterminated_escape() -> Self {
        DetectError::UnterminatedEscape {
            span: (0, 0).into(),
            src: String::new(),
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
            DetectError::Syntax { src: s, .. } |
            DetectError::UnknownSelector { src: s, .. } |
            DetectError::UnknownOperator { src: s, .. } |
            DetectError::IncompatibleOperator { src: s, .. } |
            DetectError::InvalidValue { src: s, .. } |
            DetectError::InvalidEscape { src: s, .. } |
            DetectError::UnterminatedEscape { src: s, .. } |
            DetectError::Internal { src: s, .. } => {
                *s = src;
            }
        }
        self
    }
}

