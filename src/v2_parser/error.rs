use thiserror::Error;

use super::parser::Rule;

#[derive(Debug, Error)]
pub enum RawParseError {
    #[error("Syntax error: {0}")]
    Syntax(#[from] Box<pest::error::Error<Rule>>),

    #[error("Invalid escape sequence '\\{char}' at position {position}")]
    InvalidEscape { char: char, position: usize },

    #[error("Unterminated escape sequence at end of string")]
    UnterminatedEscape,

    #[error("Internal parser error: {0}")]
    Internal(String),
}

// Extension trait for cleaner span location extraction
pub trait SpanExt {
    fn to_location(&self) -> (usize, usize);
}

impl SpanExt for pest::Span<'_> {
    #[inline]
    fn to_location(&self) -> (usize, usize) {
        self.start_pos().line_col()
    }
}

impl RawParseError {
    /// Create an invalid escape error
    pub fn invalid_escape(char: char, position: usize) -> Self {
        RawParseError::InvalidEscape { char, position }
    }

    /// Create an unterminated escape error
    pub fn unterminated_escape() -> Self {
        RawParseError::UnterminatedEscape
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        RawParseError::Internal(msg.into())
    }
}
