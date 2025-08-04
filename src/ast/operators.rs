//! Operator types for AST expressions

// ============================================================================
// Operator Enums
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringOp {
    Equals,
    NotEquals,
    Contains,
    Regex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericOp {
    Equals,
    NotEquals,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalOp {
    Equals,
    NotEquals,
    Before,
    After,
}