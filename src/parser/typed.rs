/// Type-safe selector and operator system for the v2 parser
///
/// This module provides strongly-typed enums for selectors and operators,
/// ensuring that only valid combinations can be constructed at compile time.
use super::typechecker::TypecheckError;

/// Error type for parsing selectors and operators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Unknown selector name
    UnknownSelector(String),
    /// Unknown operator name
    UnknownOperator(String),
}

// ============================================================================
// Operator Types
// ============================================================================

/// Operators that can be applied to string-type selectors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringOperator {
    Equals,    // ==, =, eq
    NotEquals, // !=, <>, ne
    Matches,   // ~=, =~, ~, matches
    Contains,  // contains, has
    In,        // in
}

/// Operators that can be applied to numeric-type selectors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericOperator {
    Equals,         // ==, =, eq
    NotEquals,      // !=, <>, ne
    Greater,        // >, gt
    GreaterOrEqual, // >=, =>, gte, ge
    Less,           // <, lt
    LessOrEqual,    // <=, =<, lte, le
}

/// Operators that can be applied to temporal-type selectors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalOperator {
    Equals,        // ==, =, eq
    NotEquals,     // !=, <>, ne
    Before,        // <, before, lt
    After,         // >, after, gt
    BeforeOrEqual, // <=, =<, lte, le
    AfterOrEqual,  // >=, =>, gte, ge
}

/// Operators that can be applied to enum-type selectors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumOperator {
    Equals,    // ==, =, eq
    NotEquals, // !=, <>, ne
    In,        // in
}

// ============================================================================
// Selector Types
// ============================================================================

/// Path component selectors - all return strings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathComponent {
    Full,      // path - absolute filesystem path
    Name,      // name - complete filename with extension
    Stem,      // basename - filename without extension
    Extension, // ext - file extension without dot
    Parent,    // dir - parent directory path
}

/// String-type selectors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringSelector {
    Path(PathComponent),
    Contents, // contents, content, text
}

impl StringSelector {
    /// Get canonical name for error messages
    pub fn canonical_name(&self) -> &'static str {
        match self {
            StringSelector::Path(PathComponent::Full) => "path",
            StringSelector::Path(PathComponent::Name) => "name",
            StringSelector::Path(PathComponent::Stem) => "basename",
            StringSelector::Path(PathComponent::Extension) => "ext",
            StringSelector::Path(PathComponent::Parent) => "dir",
            StringSelector::Contents => "content",
        }
    }
}

/// Numeric-type selectors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericSelector {
    Size,  // size, filesize, bytes
    Depth, // depth, level
}

/// Temporal-type selectors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalSelector {
    Modified, // modified, mtime, mod, modification
    Created,  // created, ctime, birth, birthtime
    Accessed, // accessed, atime, access
}

/// Enum-type selectors (validated at parse time)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnumSelector {
    Type, // type, filetype - file type enum
}

/// Selector categories - groups selectors by their value type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorCategory {
    String(StringSelector),
    Numeric(NumericSelector),
    Temporal(TemporalSelector),
    Enum(EnumSelector),
}

/// A typed selector paired with its compatible operator
/// This ensures type safety - you can't create invalid combinations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypedSelector {
    String(StringSelector, StringOperator),
    Numeric(NumericSelector, NumericOperator),
    Enum(EnumSelector, EnumOperator),
    Temporal(TemporalSelector, TemporalOperator),
}

// ============================================================================
// Parsing Functions
// ============================================================================

/// Parse a selector string into a typed selector category
///
/// # Errors
/// Returns `ParseError::UnknownSelector` if the selector name is not recognized.
pub fn recognize_selector(s: &str) -> Result<SelectorCategory, ParseError> {
    match s {
        // File Identity (5) + aliases
        "name" | "filename" => Ok(SelectorCategory::String(StringSelector::Path(
            PathComponent::Name,
        ))),
        "basename" | "stem" => Ok(SelectorCategory::String(StringSelector::Path(
            PathComponent::Stem,
        ))),
        "ext" | "extension" => Ok(SelectorCategory::String(StringSelector::Path(
            PathComponent::Extension,
        ))),
        "path" => Ok(SelectorCategory::String(StringSelector::Path(
            PathComponent::Full,
        ))),
        "dir" | "parent" | "directory" => Ok(SelectorCategory::String(StringSelector::Path(
            PathComponent::Parent,
        ))),

        // File Properties (3) + aliases
        "size" | "filesize" | "bytes" => Ok(SelectorCategory::Numeric(NumericSelector::Size)),
        "type" | "filetype" => Ok(SelectorCategory::Enum(EnumSelector::Type)),
        "depth" => Ok(SelectorCategory::Numeric(NumericSelector::Depth)),

        // Time (3) + common Unix aliases
        "modified" | "mtime" => Ok(SelectorCategory::Temporal(TemporalSelector::Modified)),
        "created" | "ctime" => Ok(SelectorCategory::Temporal(TemporalSelector::Created)),
        "accessed" | "atime" => Ok(SelectorCategory::Temporal(TemporalSelector::Accessed)),

        // Content (1) + aliases
        "content" | "contents" | "text" => Ok(SelectorCategory::String(StringSelector::Contents)),

        // Everything else is unknown
        _ => Err(ParseError::UnknownSelector(s.to_string())),
    }
}

/// Parse a string operator with aliases
///
/// # Errors
/// Returns `ParseError::UnknownOperator` if the operator is not recognized.
pub fn parse_string_operator(s: &str) -> Result<StringOperator, ParseError> {
    let s_lower = s.to_lowercase();
    match s_lower.as_str() {
        "==" | "=" | "eq" => Ok(StringOperator::Equals),
        "!=" | "<>" | "ne" | "neq" => Ok(StringOperator::NotEquals),
        "~=" | "=~" | "~" | "matches" | "regex" => Ok(StringOperator::Matches),
        "contains" | "has" | "includes" => Ok(StringOperator::Contains),
        "in" => Ok(StringOperator::In),
        _ => Err(ParseError::UnknownOperator(s.to_string())),
    }
}

/// Parse a numeric operator with aliases
///
/// # Errors
/// Returns `ParseError::UnknownOperator` if the operator is not recognized.
pub fn parse_numeric_operator(s: &str) -> Result<NumericOperator, ParseError> {
    let s_lower = s.to_lowercase();
    match s_lower.as_str() {
        "==" | "=" | "eq" => Ok(NumericOperator::Equals),
        "!=" | "<>" | "ne" | "neq" => Ok(NumericOperator::NotEquals),
        ">" | "gt" => Ok(NumericOperator::Greater),
        ">=" | "=>" | "gte" | "ge" => Ok(NumericOperator::GreaterOrEqual),
        "<" | "lt" => Ok(NumericOperator::Less),
        "<=" | "=<" | "lte" | "le" => Ok(NumericOperator::LessOrEqual),
        _ => Err(ParseError::UnknownOperator(s.to_string())),
    }
}

/// Parse a temporal operator with aliases
///
/// # Errors
/// Returns `ParseError::UnknownOperator` if the operator is not recognized.
pub fn parse_temporal_operator(s: &str) -> Result<TemporalOperator, ParseError> {
    let s_lower = s.to_lowercase();
    match s_lower.as_str() {
        "==" | "=" | "eq" | "on" => Ok(TemporalOperator::Equals),
        "!=" | "<>" | "ne" | "neq" => Ok(TemporalOperator::NotEquals),
        "<" | "before" | "lt" => Ok(TemporalOperator::Before),
        ">" | "after" | "gt" => Ok(TemporalOperator::After),
        "<=" | "=<" | "lte" | "le" => Ok(TemporalOperator::BeforeOrEqual),
        ">=" | "=>" | "gte" | "ge" => Ok(TemporalOperator::AfterOrEqual),
        _ => Err(ParseError::UnknownOperator(s.to_string())),
    }
}

/// Parse an enum operator with aliases
///
/// # Errors
/// Returns `ParseError::UnknownOperator` if the operator is not recognized.
pub fn parse_enum_operator(s: &str) -> Result<EnumOperator, ParseError> {
    let s_lower = s.to_lowercase();
    match s_lower.as_str() {
        "==" | "=" | "eq" => Ok(EnumOperator::Equals),
        "!=" | "<>" | "ne" | "neq" => Ok(EnumOperator::NotEquals),
        "in" => Ok(EnumOperator::In),
        _ => Err(ParseError::UnknownOperator(s.to_string())),
    }
}

/// Parse selector and operator together, ensuring type compatibility
///
/// # Errors
/// Returns `TypecheckError` for unknown selectors, unknown operators, or incompatible selector-operator combinations.
pub fn parse_selector_operator(
    selector_str: &str,
    selector_span: pest::Span,
    operator_str: &str,
    operator_span: pest::Span,
    source: &str,
) -> Result<TypedSelector, TypecheckError> {
    use crate::parser::error::SpanExt;
    let selector_category =
        recognize_selector(selector_str).map_err(|_| TypecheckError::UnknownSelector {
            selector: selector_str.to_string(),
            span: selector_span.to_source_span(),
            src: source.to_string(),
        })?;

    // Check if operator exists for ANY type to determine error type
    let operator_lower = operator_str.to_lowercase();
    let is_known_operator = parse_string_operator(&operator_lower).is_ok()
        || parse_numeric_operator(&operator_lower).is_ok()
        || parse_temporal_operator(&operator_lower).is_ok();

    match selector_category {
        SelectorCategory::Enum(selector) => {
            let operator = parse_enum_operator(operator_str).map_err(|_| {
                if is_known_operator {
                    TypecheckError::IncompatibleOperator {
                        selector: selector_str.to_string(),
                        operator: operator_str.to_string(),
                        selector_span: selector_span.to_source_span(),
                        operator_span: operator_span.to_source_span(),
                        src: source.to_string(),
                    }
                } else {
                    TypecheckError::UnknownOperator {
                        operator: operator_str.to_string(),
                        span: operator_span.to_source_span(),
                        src: source.to_string(),
                    }
                }
            })?;
            Ok(TypedSelector::Enum(selector, operator))
        }

        SelectorCategory::String(selector) => {
            let operator = parse_string_operator(operator_str).map_err(|_| {
                if is_known_operator {
                    TypecheckError::IncompatibleOperator {
                        selector: selector_str.to_string(),
                        operator: operator_str.to_string(),
                        selector_span: selector_span.to_source_span(),
                        operator_span: operator_span.to_source_span(),
                        src: source.to_string(),
                    }
                } else {
                    TypecheckError::UnknownOperator {
                        operator: operator_str.to_string(),
                        span: operator_span.to_source_span(),
                        src: source.to_string(),
                    }
                }
            })?;

            // Special validation: Contents doesn't support 'in' or '!='
            if matches!(selector, StringSelector::Contents) {
                match operator {
                    StringOperator::In | StringOperator::NotEquals => {
                        return Err(TypecheckError::IncompatibleOperator {
                            selector: selector_str.to_string(),
                            operator: operator_str.to_string(),
                            selector_span: selector_span.to_source_span(),
                            operator_span: operator_span.to_source_span(),
                            src: source.to_string(),
                        });
                    }
                    _ => {}
                }
            }

            Ok(TypedSelector::String(selector, operator))
        }

        SelectorCategory::Numeric(selector) => {
            let operator = parse_numeric_operator(operator_str).map_err(|_| {
                if is_known_operator {
                    TypecheckError::IncompatibleOperator {
                        selector: selector_str.to_string(),
                        operator: operator_str.to_string(),
                        selector_span: selector_span.to_source_span(),
                        operator_span: operator_span.to_source_span(),
                        src: source.to_string(),
                    }
                } else {
                    TypecheckError::UnknownOperator {
                        operator: operator_str.to_string(),
                        span: operator_span.to_source_span(),
                        src: source.to_string(),
                    }
                }
            })?;
            Ok(TypedSelector::Numeric(selector, operator))
        }

        SelectorCategory::Temporal(selector) => {
            let operator = parse_temporal_operator(operator_str).map_err(|_| {
                if is_known_operator {
                    TypecheckError::IncompatibleOperator {
                        selector: selector_str.to_string(),
                        operator: operator_str.to_string(),
                        selector_span: selector_span.to_source_span(),
                        operator_span: operator_span.to_source_span(),
                        src: source.to_string(),
                    }
                } else {
                    TypecheckError::UnknownOperator {
                        operator: operator_str.to_string(),
                        span: operator_span.to_source_span(),
                        src: source.to_string(),
                    }
                }
            })?;
            Ok(TypedSelector::Temporal(selector, operator))
        }
    }
}

// ============================================================================
// Display implementations for debugging
// ============================================================================

impl NumericSelector {
    /// Get canonical name for error messages
    pub fn canonical_name(&self) -> &'static str {
        match self {
            NumericSelector::Size => "size",
            NumericSelector::Depth => "depth",
        }
    }
}

impl TemporalSelector {
    /// Get canonical name for error messages
    pub fn canonical_name(&self) -> &'static str {
        match self {
            TemporalSelector::Modified => "modified",
            TemporalSelector::Created => "created",
            TemporalSelector::Accessed => "accessed",
        }
    }
}
