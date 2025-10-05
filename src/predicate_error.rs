use thiserror::Error;

/// Error type for predicate parsing operations
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

    #[error("Unknown selector: {0}")]
    UnknownSelector(String),
}
