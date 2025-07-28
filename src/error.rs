use std::fmt;

/// Structured error type for detect operations
#[derive(Debug)]
pub enum DetectError {
    /// Parse error with the input expression
    ParseError {
        message: String,
        hint: Option<String>,
        location: Option<(usize, usize)>,
    },
    /// Any other error (gradual migration path)
    Other(anyhow::Error),
}

impl fmt::Display for DetectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DetectError::ParseError { message, hint, location } => {
                write!(f, "{}", message)?;
                if let Some((line, col)) = location {
                    write!(f, " at line {}, column {}", line, col)?;
                }
                if let Some(hint) = hint {
                    write!(f, "\n\n{}", hint)?;
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
            DetectError::ParseError { .. } => None,
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
            message: err.to_string(),
            hint: err.hint(),
            location: err.location(),
        }
    }
}