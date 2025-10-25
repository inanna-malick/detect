//! Tests for structured data path parsing
//!
//! Validates the parsing of path expressions like:
//! - `.spec.replicas`
//! - `[0].name`
//! - `.items[*].id`

use detect::parser::structured_path::{parse_path, PathComponent, PathParseError};

// ============================================================================
// Simple Paths
// ============================================================================

#[test]
fn parse_single_key() {
    let result = parse_path(".name").unwrap();
    assert_eq!(result, vec![PathComponent::Key("name".to_string())]);
}

#[test]
fn parse_two_keys() {
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
fn parse_three_keys() {
    let result = parse_path(".metadata.labels.app").unwrap();
    assert_eq!(
        result,
        vec![
            PathComponent::Key("metadata".to_string()),
            PathComponent::Key("labels".to_string()),
            PathComponent::Key("app".to_string()),
        ]
    );
}

#[test]
fn parse_deeply_nested() {
    let result = parse_path(".a.b.c.d.e.f").unwrap();
    assert_eq!(result.len(), 6);
    assert!(matches!(result[0], PathComponent::Key(_)));
    assert!(matches!(result[5], PathComponent::Key(_)));
}

// ============================================================================
// Array Indices
// ============================================================================

#[test]
fn parse_single_index() {
    let result = parse_path("[0]").unwrap();
    assert_eq!(result, vec![PathComponent::Index(0)]);
}

#[test]
fn parse_index_then_key() {
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
fn parse_key_then_index() {
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
fn parse_multiple_indices() {
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
fn parse_large_index() {
    let result = parse_path("[12345]").unwrap();
    assert_eq!(result, vec![PathComponent::Index(12345)]);
}

#[test]
fn parse_zero_index() {
    let result = parse_path("[0]").unwrap();
    assert_eq!(result, vec![PathComponent::Index(0)]);
}

// ============================================================================
// Wildcards
// ============================================================================

#[test]
fn parse_single_wildcard() {
    let result = parse_path("[*]").unwrap();
    assert_eq!(result, vec![PathComponent::WildcardIndex]);
}

#[test]
fn parse_wildcard_with_key() {
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
fn parse_multiple_wildcards() {
    let result = parse_path(".grid[*][*]").unwrap();
    assert_eq!(
        result,
        vec![
            PathComponent::Key("grid".to_string()),
            PathComponent::WildcardIndex,
            PathComponent::WildcardIndex,
        ]
    );
}

#[test]
fn parse_wildcard_at_start() {
    let result = parse_path("[*].name").unwrap();
    assert_eq!(
        result,
        vec![
            PathComponent::WildcardIndex,
            PathComponent::Key("name".to_string()),
        ]
    );
}

// ============================================================================
// Mixed Patterns
// ============================================================================

#[test]
fn parse_kubernetes_manifest_path() {
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
fn parse_package_json_path() {
    let result = parse_path(".dependencies.react").unwrap();
    assert_eq!(
        result,
        vec![
            PathComponent::Key("dependencies".to_string()),
            PathComponent::Key("react".to_string()),
        ]
    );
}

#[test]
fn parse_nested_array_access() {
    let result = parse_path(".data[0].items[1].value").unwrap();
    assert_eq!(
        result,
        vec![
            PathComponent::Key("data".to_string()),
            PathComponent::Index(0),
            PathComponent::Key("items".to_string()),
            PathComponent::Index(1),
            PathComponent::Key("value".to_string()),
        ]
    );
}

#[test]
fn parse_wildcard_in_middle() {
    let result = parse_path(".users[*].posts[0].title").unwrap();
    assert_eq!(
        result,
        vec![
            PathComponent::Key("users".to_string()),
            PathComponent::WildcardIndex,
            PathComponent::Key("posts".to_string()),
            PathComponent::Index(0),
            PathComponent::Key("title".to_string()),
        ]
    );
}

// ============================================================================
// Key Naming Variations
// ============================================================================

#[test]
fn parse_underscore_in_key() {
    let result = parse_path(".my_field").unwrap();
    assert_eq!(result, vec![PathComponent::Key("my_field".to_string())]);
}

#[test]
fn parse_camel_case_key() {
    let result = parse_path(".camelCase").unwrap();
    assert_eq!(result, vec![PathComponent::Key("camelCase".to_string())]);
}

#[test]
fn parse_pascal_case_key() {
    let result = parse_path(".PascalCase").unwrap();
    assert_eq!(result, vec![PathComponent::Key("PascalCase".to_string())]);
}

#[test]
fn parse_snake_case_key() {
    let result = parse_path(".snake_case_field").unwrap();
    assert_eq!(
        result,
        vec![PathComponent::Key("snake_case_field".to_string())]
    );
}

#[test]
fn parse_numeric_suffix_key() {
    let result = parse_path(".field123").unwrap();
    assert_eq!(result, vec![PathComponent::Key("field123".to_string())]);
}

#[test]
fn parse_key_starting_with_underscore() {
    let result = parse_path("._private").unwrap();
    assert_eq!(result, vec![PathComponent::Key("_private".to_string())]);
}

#[test]
fn parse_multiple_underscores() {
    let result = parse_path(".__proto__").unwrap();
    assert_eq!(result, vec![PathComponent::Key("__proto__".to_string())]);
}

// ============================================================================
// Error Cases
// ============================================================================

#[test]
fn error_empty_path() {
    let result = parse_path("");
    assert!(matches!(result, Err(PathParseError::EmptyPath)));
}

#[test]
fn error_bare_word_no_dot() {
    let result = parse_path("name");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_missing_opening_bracket() {
    let result = parse_path("0]");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_missing_closing_bracket() {
    let result = parse_path("[0");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_empty_brackets() {
    let result = parse_path("[]");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_double_dot() {
    let result = parse_path("..field");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_dot_only() {
    let result = parse_path(".");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_space_in_key() {
    let result = parse_path(".my field");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_hyphen_in_key() {
    let result = parse_path(".my-field");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_special_char_in_key() {
    let result = parse_path(".field@name");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_key_starting_with_number() {
    let result = parse_path(".123field");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_negative_index() {
    let result = parse_path("[-1]");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_float_index() {
    let result = parse_path("[3.14]");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_wildcard_with_number() {
    let result = parse_path("[*1]");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_multiple_wildcards_in_bracket() {
    let result = parse_path("[**]");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_text_in_brackets() {
    let result = parse_path("[abc]");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_missing_key_after_dot() {
    let result = parse_path(".field.");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

#[test]
fn error_consecutive_indices_no_separation() {
    // This should actually parse fine as two separate indices
    let result = parse_path("[0][1]");
    assert!(result.is_ok());
}

#[test]
fn error_bracket_after_dot() {
    // .[] is invalid - missing key
    let result = parse_path(".[]");
    assert!(matches!(result, Err(PathParseError::Syntax(_))));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn edge_case_very_long_key() {
    let long_key = "a".repeat(100);
    let path = format!(".{}", long_key);
    let result = parse_path(&path).unwrap();
    assert_eq!(result, vec![PathComponent::Key(long_key)]);
}

#[test]
fn edge_case_many_nested_keys() {
    let mut path = String::new();
    for i in 0..50 {
        path.push_str(&format!(".k{}", i));
    }
    let result = parse_path(&path).unwrap();
    assert_eq!(result.len(), 50);
}

#[test]
fn edge_case_alternating_keys_and_indices() {
    let result = parse_path(".a[0].b[1].c[2]").unwrap();
    assert_eq!(result.len(), 6);
    assert!(matches!(result[0], PathComponent::Key(_)));
    assert!(matches!(result[1], PathComponent::Index(0)));
    assert!(matches!(result[2], PathComponent::Key(_)));
    assert!(matches!(result[3], PathComponent::Index(1)));
    assert!(matches!(result[4], PathComponent::Key(_)));
    assert!(matches!(result[5], PathComponent::Index(2)));
}

#[test]
fn edge_case_single_letter_keys() {
    let result = parse_path(".a.b.c").unwrap();
    assert_eq!(
        result,
        vec![
            PathComponent::Key("a".to_string()),
            PathComponent::Key("b".to_string()),
            PathComponent::Key("c".to_string()),
        ]
    );
}

// ============================================================================
// Real-World Examples
// ============================================================================

#[test]
fn real_world_k8s_deployment() {
    let result = parse_path(".spec.template.spec.containers[0].resources.limits.memory").unwrap();
    assert_eq!(result.len(), 8);
}

#[test]
fn real_world_package_json_scripts() {
    let result = parse_path(".scripts.test").unwrap();
    assert_eq!(
        result,
        vec![
            PathComponent::Key("scripts".to_string()),
            PathComponent::Key("test".to_string()),
        ]
    );
}

#[test]
fn real_world_docker_compose() {
    let result = parse_path(".services.web.ports[0]").unwrap();
    assert_eq!(
        result,
        vec![
            PathComponent::Key("services".to_string()),
            PathComponent::Key("web".to_string()),
            PathComponent::Key("ports".to_string()),
            PathComponent::Index(0),
        ]
    );
}

#[test]
fn real_world_github_actions() {
    let result = parse_path(".jobs.build.steps[*].name").unwrap();
    assert_eq!(
        result,
        vec![
            PathComponent::Key("jobs".to_string()),
            PathComponent::Key("build".to_string()),
            PathComponent::Key("steps".to_string()),
            PathComponent::WildcardIndex,
            PathComponent::Key("name".to_string()),
        ]
    );
}

#[test]
fn real_world_terraform_state() {
    let result = parse_path(".resources[*].instances[0].attributes.id").unwrap();
    assert_eq!(result.len(), 6);
    assert!(matches!(result[1], PathComponent::WildcardIndex));
    assert!(matches!(result[3], PathComponent::Index(0)));
}
