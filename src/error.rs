use std::fmt;
use std::sync::Arc;

/// Structured error type for detect operations
#[derive(Debug)]
pub enum DetectError {
    /// Parse error with the input expression and optional source
    ParseError {
        /// The original parse error
        error: crate::parse_error::ParseError,
        /// The original source text for diagnostic display
        source: Option<Arc<str>>,
    },
    /// Any other error (gradual migration path)
    Other(anyhow::Error),
}

impl fmt::Display for DetectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DetectError::ParseError { error, source } => {
                // Check if we should use Miette formatting
                if let Some(src) = source {
                    // Try to create a diagnostic and display it
                    let _diagnostic = crate::diagnostics::parse_error_to_diagnostic(
                        error,
                        src,
                        None,
                    );
                    // For now, fall back to regular display
                    // In main.rs we'll use miette::Report for proper rendering
                    write!(f, "{}", error)?;
                    if let Some(hint) = error.hint() {
                        write!(f, "\n\n{}", hint)?;
                    }
                } else {
                    // Regular display without source
                    write!(f, "{}", error)?;
                    if let Some(hint) = error.hint() {
                        write!(f, "\n\n{}", hint)?;
                    }
                }
                Ok(())
            }
            DetectError::Other(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for DetectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DetectError::ParseError { error, .. } => Some(error),
            DetectError::Other(e) => e.source(),
        }
    }
}

impl From<anyhow::Error> for DetectError {
    fn from(err: anyhow::Error) -> Self {
        DetectError::Other(err)
    }
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
        if let DetectError::ParseError { source: ref mut src, .. } = self {
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
    
    /// Convert to a Miette diagnostic if this is a parse error with source
    pub fn to_diagnostic(&self) -> Option<crate::diagnostics::DetectDiagnostic> {
        match self {
            DetectError::ParseError { error, source: Some(src) } => {
                Some(crate::diagnostics::parse_error_to_diagnostic(
                    error,
                    src,
                    None,
                ))
            }
            _ => None,
        }
    }
}
