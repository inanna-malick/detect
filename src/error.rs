use std::sync::Arc;
use thiserror::Error;

/// Structured error type for detect operations
#[derive(Debug, Error)]
pub enum DetectError {
    /// Parse error with the input expression and optional source
    #[error("{}", .error)]
    ParseError {
        /// The original parse error
        #[source]
        error: crate::parse_error::ParseError,
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

use crate::parse_error::ParseError;

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
