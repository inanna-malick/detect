//! Selector types for AST expressions

// ============================================================================
// Selector Enums
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringSelectorType {
    PathFull,
    PathParent,
    PathParentDir,
    PathName,
    PathStem,
    PathSuffix,
    Contents,
    Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumericSelectorType {
    Size,
    Depth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalSelectorType {
    Modified,
    Created,
    Accessed,
}