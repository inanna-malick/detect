/// Type-safe selector and operator system for the v2 parser
///
/// This module provides strongly-typed enums for selectors and operators,
/// ensuring that only valid combinations can be constructed at compile time.
use super::typechecker::TypecheckError;

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
    Equals,    // ==, =, eq
    NotEquals, // !=, <>, ne
    Before,    // <, before, lt
    After,     // >, after, gt
}

// ============================================================================
// Selector Types
// ============================================================================

/// Path component selectors - all return strings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathComponent {
    Full,      // path, path.full, full
    Name,      // filename, path.filename, name, file
    Stem,      // stem, path.stem, basename, base
    Extension, // ext, extension, path.extension, suffix
    Parent,    // parent, path.parent, dir, directory
}

/// String-type selectors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringSelector {
    Path(PathComponent),
    Contents, // contents, content, text
    Type,     // type, filetype, kind
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

/// Selector categories - groups selectors by their value type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorCategory {
    String(StringSelector),
    Numeric(NumericSelector),
    Temporal(TemporalSelector),
}

/// A typed selector paired with its compatible operator
/// This ensures type safety - you can't create invalid combinations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypedSelector {
    String(StringSelector, StringOperator),
    Numeric(NumericSelector, NumericOperator),
    Temporal(TemporalSelector, TemporalOperator),
}

// ============================================================================
// Parsing Functions
// ============================================================================

/// Parse a selector string into a typed selector category
pub fn recognize_selector(s: &str) -> Result<SelectorCategory, ()> {
    match s {
        // Path selectors - Full
        "path" | "path.full" | "full" => Ok(SelectorCategory::String(StringSelector::Path(
            PathComponent::Full,
        ))),

        // Path selectors - Name (full filename with extension)
        "filename" | "path.filename" | "file" => Ok(SelectorCategory::String(
            StringSelector::Path(PathComponent::Name),
        )),

        // Path selectors - Stem (filename without extension)
        // Note: path.name maps to stem for v1 compatibility
        "stem" | "path.stem" | "path.name" | "name" | "basename" | "base" => Ok(
            SelectorCategory::String(StringSelector::Path(PathComponent::Stem)),
        ),

        // Path selectors - Extension
        "ext" | "extension" | "path.extension" | "suffix" => Ok(SelectorCategory::String(
            StringSelector::Path(PathComponent::Extension),
        )),

        // Path selectors - Parent
        "parent" | "path.parent" | "dir" | "directory" => Ok(SelectorCategory::String(
            StringSelector::Path(PathComponent::Parent),
        )),

        // Contents selector
        "contents" | "content" | "text" => Ok(SelectorCategory::String(StringSelector::Contents)),

        // Type selector
        "type" | "filetype" | "kind" => Ok(SelectorCategory::String(StringSelector::Type)),

        // Size selector
        "size" | "filesize" | "bytes" => Ok(SelectorCategory::Numeric(NumericSelector::Size)),

        // Depth selector
        "depth" | "level" => Ok(SelectorCategory::Numeric(NumericSelector::Depth)),

        // Temporal selectors - Modified
        "modified" | "mtime" | "mod" | "modification" => {
            Ok(SelectorCategory::Temporal(TemporalSelector::Modified))
        }

        // Temporal selectors - Created
        "created" | "ctime" | "birth" | "birthtime" => {
            Ok(SelectorCategory::Temporal(TemporalSelector::Created))
        }

        // Temporal selectors - Accessed
        "accessed" | "atime" | "access" => {
            Ok(SelectorCategory::Temporal(TemporalSelector::Accessed))
        }

        _ => Err(()),
    }
}

/// Parse a string operator with aliases
pub fn parse_string_operator(s: &str) -> Result<StringOperator, ()> {
    let s_lower = s.to_lowercase();
    match s_lower.as_str() {
        "==" | "=" | "eq" => Ok(StringOperator::Equals),
        "!=" | "<>" | "ne" | "neq" => Ok(StringOperator::NotEquals),
        "~=" | "=~" | "~" | "matches" | "regex" => Ok(StringOperator::Matches),
        "contains" | "has" | "includes" => Ok(StringOperator::Contains),
        "in" => Ok(StringOperator::In),
        _ => Err(()),
    }
}

/// Parse a numeric operator with aliases
pub fn parse_numeric_operator(s: &str) -> Result<NumericOperator, ()> {
    let s_lower = s.to_lowercase();
    match s_lower.as_str() {
        "==" | "=" | "eq" => Ok(NumericOperator::Equals),
        "!=" | "<>" | "ne" | "neq" => Ok(NumericOperator::NotEquals),
        ">" | "gt" => Ok(NumericOperator::Greater),
        ">=" | "=>" | "gte" | "ge" => Ok(NumericOperator::GreaterOrEqual),
        "<" | "lt" => Ok(NumericOperator::Less),
        "<=" | "=<" | "lte" | "le" => Ok(NumericOperator::LessOrEqual),
        _ => Err(()),
    }
}

/// Parse a temporal operator with aliases
pub fn parse_temporal_operator(s: &str) -> Result<TemporalOperator, ()> {
    let s_lower = s.to_lowercase();
    match s_lower.as_str() {
        "==" | "=" | "eq" | "on" => Ok(TemporalOperator::Equals),
        "!=" | "<>" | "ne" | "neq" => Ok(TemporalOperator::NotEquals),
        "<" | "before" | "lt" => Ok(TemporalOperator::Before),
        ">" | "after" | "gt" => Ok(TemporalOperator::After),
        _ => Err(()),
    }
}

/// Parse selector and operator together, ensuring type compatibility
pub fn parse_selector_operator(
    selector_str: &str,
    selector_span: pest::Span,
    operator_str: &str,
    operator_span: pest::Span,
    source: &str,
) -> Result<TypedSelector, TypecheckError> {
    use crate::v2_parser::error::SpanExt;
    let selector_category = recognize_selector(selector_str).map_err(|_| {
        TypecheckError::UnknownSelector {
            selector: selector_str.to_string(),
            span: selector_span.to_source_span(),
            src: source.to_string(),
        }
    })?;

    // Check if operator exists for ANY type to determine error type
    let operator_lower = operator_str.to_lowercase();
    let is_known_operator = parse_string_operator(&operator_lower).is_ok()
        || parse_numeric_operator(&operator_lower).is_ok()
        || parse_temporal_operator(&operator_lower).is_ok();

    match selector_category {
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

impl StringSelector {
    /// Get canonical name for error messages
    pub fn canonical_name(&self) -> &'static str {
        match self {
            StringSelector::Path(PathComponent::Full) => "path.full",
            StringSelector::Path(PathComponent::Name) => "filename",
            StringSelector::Path(PathComponent::Stem) => "stem",
            StringSelector::Path(PathComponent::Extension) => "extension",
            StringSelector::Path(PathComponent::Parent) => "parent",
            StringSelector::Contents => "contents",
            StringSelector::Type => "type",
        }
    }
}

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
