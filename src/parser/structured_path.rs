//! Parser for structured data path expressions
//!
//! Handles paths like:
//! - `.spec.replicas` → [Key("spec"), Key("replicas")]
//! - `[0].name` → [Index(0), Key("name")]
//! - `.items[*].id` → [Key("items"), WildcardIndex, Key("id")]

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use thiserror::Error;

#[derive(Parser)]
#[grammar = "parser/structured_path.pest"]
pub struct PathParser;

/// A single component in a path expression
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathComponent {
    /// Object field access: .fieldname
    Key(String),
    /// Recursive descent: ..fieldname (matches key at any depth)
    RecursiveKey(String),
    /// Array index access: [42]
    Index(usize),
    /// Array wildcard access: [*]
    WildcardIndex,
}

/// Errors that can occur during path parsing
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PathParseError {
    /// Syntax error from Pest parser
    #[error("Path syntax error: {0}")]
    Syntax(String),

    /// Invalid numeric index value
    #[error("Invalid array index '{value}': {reason}")]
    InvalidIndex { value: String, reason: String },

    /// Empty path (no components)
    #[error("Path cannot be empty")]
    EmptyPath,
}

/// Parse a path expression into a vector of components
///
/// # Examples
/// ```
/// use detect::parser::structured_path::{parse_path, PathComponent};
///
/// let components = parse_path(".spec.replicas").unwrap();
/// assert_eq!(components, vec![
///     PathComponent::Key("spec".to_string()),
///     PathComponent::Key("replicas".to_string()),
/// ]);
///
/// let components = parse_path("[0].name").unwrap();
/// assert_eq!(components, vec![
///     PathComponent::Index(0),
///     PathComponent::Key("name".to_string()),
/// ]);
/// ```
pub fn parse_path(input: &str) -> Result<Vec<PathComponent>, PathParseError> {
    if input.is_empty() {
        return Err(PathParseError::EmptyPath);
    }

    let pairs = PathParser::parse(Rule::path, input)
        .map_err(|e| PathParseError::Syntax(format!("Failed to parse path '{}': {}", input, e)))?;

    let mut components = Vec::new();

    for pair in pairs {
        match pair.as_rule() {
            Rule::path => {
                // Recurse into path components
                for component_pair in pair.into_inner() {
                    if let Some(component) = parse_component(component_pair)? {
                        components.push(component);
                    }
                }
            }
            Rule::EOI => {} // End of input, ignore
            _ => {
                return Err(PathParseError::Syntax(format!(
                    "Unexpected rule: {:?}",
                    pair.as_rule()
                )))
            }
        }
    }

    if components.is_empty() {
        return Err(PathParseError::EmptyPath);
    }

    Ok(components)
}

fn parse_component(pair: Pair<'_, Rule>) -> Result<Option<PathComponent>, PathParseError> {
    match pair.as_rule() {
        Rule::recursive_key => {
            // recursive_key -> identifier
            let identifier = pair
                .into_inner()
                .next()
                .ok_or_else(|| PathParseError::Syntax("Missing identifier".to_string()))?;
            Ok(Some(PathComponent::RecursiveKey(
                identifier.as_str().to_string(),
            )))
        }
        Rule::key_access => {
            // key_access -> identifier
            let identifier = pair
                .into_inner()
                .next()
                .ok_or_else(|| PathParseError::Syntax("Missing identifier".to_string()))?;
            Ok(Some(PathComponent::Key(identifier.as_str().to_string())))
        }
        Rule::index_access => {
            // index_access -> number
            let number_pair = pair
                .into_inner()
                .next()
                .ok_or_else(|| PathParseError::Syntax("Missing number".to_string()))?;
            let number_str = number_pair.as_str();

            let index = number_str
                .parse::<usize>()
                .map_err(|e| PathParseError::InvalidIndex {
                    value: number_str.to_string(),
                    reason: e.to_string(),
                })?;

            Ok(Some(PathComponent::Index(index)))
        }
        Rule::wildcard_access => Ok(Some(PathComponent::WildcardIndex)),
        _ => Ok(None), // Skip unknown rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_key() {
        let result = parse_path(".name").unwrap();
        assert_eq!(result, vec![PathComponent::Key("name".to_string())]);
    }

    #[test]
    fn test_nested_keys() {
        let result = parse_path(".spec.replicas").unwrap();
        assert_eq!(
            result,
            vec![
                PathComponent::Key("spec".to_string()),
                PathComponent::Key("replicas".to_string()),
            ]
        );
    }

    #[test]
    fn test_deep_nesting() {
        let result = parse_path(".a.b.c.d").unwrap();
        assert_eq!(
            result,
            vec![
                PathComponent::Key("a".to_string()),
                PathComponent::Key("b".to_string()),
                PathComponent::Key("c".to_string()),
                PathComponent::Key("d".to_string()),
            ]
        );
    }

    #[test]
    fn test_single_index() {
        let result = parse_path("[0]").unwrap();
        assert_eq!(result, vec![PathComponent::Index(0)]);
    }

    #[test]
    fn test_index_then_key() {
        let result = parse_path("[0].name").unwrap();
        assert_eq!(
            result,
            vec![
                PathComponent::Index(0),
                PathComponent::Key("name".to_string()),
            ]
        );
    }

    #[test]
    fn test_key_then_index() {
        let result = parse_path(".items[0]").unwrap();
        assert_eq!(
            result,
            vec![
                PathComponent::Key("items".to_string()),
                PathComponent::Index(0),
            ]
        );
    }

    #[test]
    fn test_wildcard() {
        let result = parse_path("[*]").unwrap();
        assert_eq!(result, vec![PathComponent::WildcardIndex]);
    }

    #[test]
    fn test_wildcard_with_keys() {
        let result = parse_path(".items[*].id").unwrap();
        assert_eq!(
            result,
            vec![
                PathComponent::Key("items".to_string()),
                PathComponent::WildcardIndex,
                PathComponent::Key("id".to_string()),
            ]
        );
    }

    #[test]
    fn test_multiple_indices() {
        let result = parse_path("[0][1][2]").unwrap();
        assert_eq!(
            result,
            vec![
                PathComponent::Index(0),
                PathComponent::Index(1),
                PathComponent::Index(2),
            ]
        );
    }

    #[test]
    fn test_complex_path() {
        let result = parse_path(".spec.containers[0].image").unwrap();
        assert_eq!(
            result,
            vec![
                PathComponent::Key("spec".to_string()),
                PathComponent::Key("containers".to_string()),
                PathComponent::Index(0),
                PathComponent::Key("image".to_string()),
            ]
        );
    }

    #[test]
    fn test_underscore_in_key() {
        let result = parse_path(".my_field").unwrap();
        assert_eq!(result, vec![PathComponent::Key("my_field".to_string())]);
    }

    #[test]
    fn test_mixed_case_key() {
        let result = parse_path(".camelCase").unwrap();
        assert_eq!(result, vec![PathComponent::Key("camelCase".to_string())]);
    }

    #[test]
    fn test_large_index() {
        let result = parse_path("[999]").unwrap();
        assert_eq!(result, vec![PathComponent::Index(999)]);
    }

    #[test]
    fn test_error_empty_path() {
        let result = parse_path("");
        assert!(matches!(result, Err(PathParseError::EmptyPath)));
    }

    #[test]
    fn test_error_no_dot_before_key() {
        let result = parse_path("name");
        assert!(matches!(result, Err(PathParseError::Syntax(_))));
    }

    #[test]
    fn test_error_missing_bracket() {
        let result = parse_path("[0");
        assert!(matches!(result, Err(PathParseError::Syntax(_))));
    }

    #[test]
    fn test_error_missing_closing_bracket() {
        let result = parse_path(".items[0");
        assert!(matches!(result, Err(PathParseError::Syntax(_))));
    }

    #[test]
    fn test_error_empty_brackets() {
        let result = parse_path("[]");
        assert!(matches!(result, Err(PathParseError::Syntax(_))));
    }

    #[test]
    fn test_error_invalid_character_in_key() {
        let result = parse_path(".field-name");
        assert!(matches!(result, Err(PathParseError::Syntax(_))));
    }

    #[test]
    fn test_error_triple_dot() {
        // Triple dots are invalid (recursive descent is only double dots)
        let result = parse_path("...field");
        assert!(matches!(result, Err(PathParseError::Syntax(_))));
    }

    #[test]
    fn test_error_space_in_key() {
        let result = parse_path(".my field");
        assert!(matches!(result, Err(PathParseError::Syntax(_))));
    }
}
