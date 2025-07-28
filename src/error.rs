use std::fmt;

/// Structured error type for detect operations
#[derive(Debug)]
pub enum DetectError {
    /// Parse error with the input expression
    ParseError {
        input: String,
        message: String,
    },
    /// Any other error (gradual migration path)
    Other(anyhow::Error),
}

impl fmt::Display for DetectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DetectError::ParseError { input, message } => {
                write!(f, "Failed to parse expression '{}': {}", input, message)
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