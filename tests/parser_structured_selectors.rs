//! Tests for structured data selector parsing (yaml:, json:, toml:)
//!
//! Validates that selectors like `yaml:.spec.replicas == 5` parse correctly
//! into StructuredData predicates with proper format, path, operator, and native typed values.

use detect::expr::Expr;
use detect::parser::structured_path::PathComponent;
use detect::parser::typed::StructuredOperator;
use detect::parser::{RawParser, Typechecker};
use detect::predicate::{
    Bound, MetadataPredicate, NamePredicate, NumberMatcher, Predicate, StringMatcher,
    StructuredDataPredicate,
};
use std::collections::HashSet;

// ============================================================================
// Test Helpers
// ============================================================================

/// Build expected synthetic predicate wrapper for structured selectors
/// Returns: (ext in [exts]) AND (size < 10MB) AND structured_predicate
fn build_expected_synthetic(
    format: &str, // "yaml", "json", or "toml"
    structured_pred: StructuredDataPredicate,
) -> Expr<Predicate> {
    let extensions: Vec<&str> = match format {
        "yaml" => vec!["yaml", "yml"],
        "json" => vec!["json"],
        "toml" => vec!["toml"],
        _ => panic!("Invalid format: {}", format),
    };

    let ext_set: HashSet<String> = extensions.iter().map(|s| s.to_string()).collect();
    let ext_pred = Expr::Predicate(Predicate::name(NamePredicate::Extension(
        StringMatcher::In(ext_set),
    )));

    let size_pred = Expr::Predicate(Predicate::meta(MetadataPredicate::Filesize(
        NumberMatcher::In(Bound::Right(..10 * 1024 * 1024)),
    )));

    let structured = Expr::Predicate(Predicate::structured(structured_pred));

    Expr::and(Expr::and(ext_pred, size_pred), structured)
}

// ============================================================================
// Type Inference Tests
// ============================================================================

#[test]
fn parse_unquoted_number() {
    let input = "yaml:.port == 8080";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::Key("port".to_string())],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::Integer(8080),
            raw_string: "8080".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_quoted_number_as_string() {
    let input = r#"yaml:.port == "8080""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::Key("port".to_string())],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::Integer(8080),
            raw_string: "8080".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_unquoted_bool_true() {
    let input = "yaml:.enabled == true";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::Key("enabled".to_string())],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::Boolean(true),
            raw_string: "true".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_unquoted_bool_false() {
    let input = "yaml:.enabled == false";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::Key("enabled".to_string())],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::Boolean(false),
            raw_string: "false".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_quoted_bool_as_string() {
    let input = r#"yaml:.enabled == "true""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::Key("enabled".to_string())],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::Boolean(true),
            raw_string: "true".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_unquoted_string_fallback() {
    let input = "yaml:.name == api";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::Key("name".to_string())],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::String("api".to_string()),
            raw_string: "api".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

// ============================================================================
// Format Detection
// ============================================================================

#[test]
fn parse_yaml_format() {
    let input = "yaml:.name == test";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::Key("name".to_string())],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::String("test".to_string()),
            raw_string: "test".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_json_format() {
    let input = r#"json:.version == "1.0.0""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "json",
        StructuredDataPredicate::JsonValue {
            path: vec![PathComponent::Key("version".to_string())],
            operator: StructuredOperator::Equals,
            value: serde_json::Value::String("1.0.0".to_string()),
            raw_string: "1.0.0".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_toml_format() {
    let input = "toml:.server.port == 8080";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "toml",
        StructuredDataPredicate::TomlValue {
            path: vec![
                PathComponent::Key("server".to_string()),
                PathComponent::Key("port".to_string()),
            ],
            operator: StructuredOperator::Equals,
            value: toml::Value::Integer(8080),
            raw_string: "8080".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

// ============================================================================
// Path Parsing
// ============================================================================

#[test]
fn parse_single_key_path() {
    let input = "yaml:.name == test";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::Key("name".to_string())],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::String("test".to_string()),
            raw_string: "test".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_nested_keys() {
    let input = "yaml:.spec.replicas == 5";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![
                PathComponent::Key("spec".to_string()),
                PathComponent::Key("replicas".to_string()),
            ],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::Integer(5),
            raw_string: "5".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_array_index_path() {
    let input = r#"json:[0].name == "first""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "json",
        StructuredDataPredicate::JsonValue {
            path: vec![
                PathComponent::Index(0),
                PathComponent::Key("name".to_string()),
            ],
            operator: StructuredOperator::Equals,
            value: serde_json::Value::String("first".to_string()),
            raw_string: "first".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_wildcard_array_path() {
    let input = "yaml:.items[*].id == 42";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![
                PathComponent::Key("items".to_string()),
                PathComponent::WildcardIndex,
                PathComponent::Key("id".to_string()),
            ],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::Integer(42),
            raw_string: "42".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_complex_path() {
    let input = r#"yaml:.spec.containers[0].image == "nginx""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![
                PathComponent::Key("spec".to_string()),
                PathComponent::Key("containers".to_string()),
                PathComponent::Index(0),
                PathComponent::Key("image".to_string()),
            ],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::String("nginx".to_string()),
            raw_string: "nginx".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

// ============================================================================
// Operator Parsing
// ============================================================================

#[test]
fn parse_comparison_greater() {
    let input = "yaml:.count > 5";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::Key("count".to_string())],
            operator: StructuredOperator::Greater,
            value: yaml_rust2::Yaml::Integer(5),
            raw_string: "5".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_comparison_greater_equal() {
    let input = "yaml:.count >= 10";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::Key("count".to_string())],
            operator: StructuredOperator::GreaterOrEqual,
            value: yaml_rust2::Yaml::Integer(10),
            raw_string: "10".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_comparison_less() {
    let input = "yaml:.count < 100";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::Key("count".to_string())],
            operator: StructuredOperator::Less,
            value: yaml_rust2::Yaml::Integer(100),
            raw_string: "100".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_regex_operator() {
    let input = r#"yaml:.name ~= "test.*""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let matcher = StringMatcher::Regex(regex::Regex::new("test.*").unwrap());

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlString {
            path: vec![PathComponent::Key("name".to_string())],
            matcher,
        },
    );

    assert_eq!(typed_expr, expected);
}

// ============================================================================
// Error Cases
// ============================================================================

#[test]
fn error_comparison_with_quoted_string() {
    let input = r#"yaml:.count > "foo""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let result = Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default());

    assert!(result.is_err(), "Comparison with quoted string should fail");
}

#[test]
fn error_comparison_with_invalid_number() {
    let input = "yaml:.count > notanumber";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let result = Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default());

    assert!(
        result.is_err(),
        "Comparison with non-numeric value should fail"
    );
}

#[test]
fn error_invalid_path_triple_dot() {
    let input = "yaml:...invalid == 5";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let result = Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default());

    assert!(result.is_err(), "Triple dots in path should fail");
}

// ============================================================================
// Boolean Logic
// ============================================================================

#[test]
fn parse_and_expression() {
    let input = r#"yaml:.kind == "Deployment" AND yaml:.spec.replicas > 5"#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    // Left: yaml:.kind == "Deployment" with synthetic wrapper
    let left_synthetic = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::Key("kind".to_string())],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::String("Deployment".to_string()),
            raw_string: "Deployment".to_string(),
        },
    );

    // Right: yaml:.spec.replicas > 5 with synthetic wrapper
    let right_synthetic = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![
                PathComponent::Key("spec".to_string()),
                PathComponent::Key("replicas".to_string()),
            ],
            operator: StructuredOperator::Greater,
            value: yaml_rust2::Yaml::Integer(5),
            raw_string: "5".to_string(),
        },
    );

    let expected = Expr::and(left_synthetic, right_synthetic);

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_mixed_selectors() {
    let input = "name == config.yaml AND yaml:.debug == true";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    // Left: Name predicate
    let name_pred = Expr::Predicate(Predicate::name(NamePredicate::FileName(
        StringMatcher::Equals("config.yaml".to_string()),
    )));

    // Right: yaml:.debug == true with synthetic wrapper
    let yaml_synthetic = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::Key("debug".to_string())],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::Boolean(true),
            raw_string: "true".to_string(),
        },
    );

    let expected = Expr::and(name_pred, yaml_synthetic);

    assert_eq!(typed_expr, expected);
}

// ============================================================================
// Real-World Examples
// ============================================================================

#[test]
fn parse_k8s_deployment_query() {
    let input = r#"yaml:.kind == "Deployment" AND yaml:.spec.replicas > 5"#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let _typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();
    // Success if it parses without error
}

#[test]
fn parse_package_json_node_version() {
    let input = r#"json:.engines.node == "18""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "json",
        StructuredDataPredicate::JsonValue {
            path: vec![
                PathComponent::Key("engines".to_string()),
                PathComponent::Key("node".to_string()),
            ],
            operator: StructuredOperator::Equals,
            value: serde_json::Value::Number(serde_json::Number::from(18)),
            raw_string: "18".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_toml_privileged_port() {
    let input = "toml:.server.port < 1024";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "toml",
        StructuredDataPredicate::TomlValue {
            path: vec![
                PathComponent::Key("server".to_string()),
                PathComponent::Key("port".to_string()),
            ],
            operator: StructuredOperator::Less,
            value: toml::Value::Integer(1024),
            raw_string: "1024".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_negative_number() {
    let input = "yaml:.offset == -10";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::Key("offset".to_string())],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::Integer(-10),
            raw_string: "-10".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_zero() {
    let input = "yaml:.count == 0";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::Key("count".to_string())],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::Integer(0),
            raw_string: "0".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

// ============================================================================
// Recursive Descent Tests
// ============================================================================

#[test]
fn parse_simple_recursive_descent() {
    let input = "yaml:..password == secret";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::RecursiveKey("password".to_string())],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::String("secret".to_string()),
            raw_string: "secret".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_recursive_debug_flag() {
    let input = "yaml:..debug == true";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![PathComponent::RecursiveKey("debug".to_string())],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::Boolean(true),
            raw_string: "true".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_key_then_recursive() {
    let input = "yaml:.user..password == admin";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![
                PathComponent::Key("user".to_string()),
                PathComponent::RecursiveKey("password".to_string()),
            ],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::String("admin".to_string()),
            raw_string: "admin".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_recursive_then_path() {
    let input = r#"yaml:..metadata.labels.app == "web""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![
                PathComponent::RecursiveKey("metadata".to_string()),
                PathComponent::Key("labels".to_string()),
                PathComponent::Key("app".to_string()),
            ],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::String("web".to_string()),
            raw_string: "web".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_recursive_with_comparison() {
    let input = "toml:..port < 1024";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "toml",
        StructuredDataPredicate::TomlValue {
            path: vec![PathComponent::RecursiveKey("port".to_string())],
            operator: StructuredOperator::Less,
            value: toml::Value::Integer(1024),
            raw_string: "1024".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_recursive_with_regex() {
    let input = r#"json:..email ~= ".*@example\.com""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let matcher = StringMatcher::Regex(regex::Regex::new(r".*@example\.com").unwrap());

    let expected = build_expected_synthetic(
        "json",
        StructuredDataPredicate::JsonString {
            path: vec![PathComponent::RecursiveKey("email".to_string())],
            matcher,
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_multiple_recursive() {
    let input = "yaml:..config..database == postgres";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![
                PathComponent::RecursiveKey("config".to_string()),
                PathComponent::RecursiveKey("database".to_string()),
            ],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::String("postgres".to_string()),
            raw_string: "postgres".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_recursive_with_array_index() {
    let input = "yaml:..users[0].name == alice";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![
                PathComponent::RecursiveKey("users".to_string()),
                PathComponent::Index(0),
                PathComponent::Key("name".to_string()),
            ],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::String("alice".to_string()),
            raw_string: "alice".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_recursive_with_wildcard() {
    let input = "yaml:..items[*].id == 42";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    let expected = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlValue {
            path: vec![
                PathComponent::RecursiveKey("items".to_string()),
                PathComponent::WildcardIndex,
                PathComponent::Key("id".to_string()),
            ],
            operator: StructuredOperator::Equals,
            value: yaml_rust2::Yaml::Integer(42),
            raw_string: "42".to_string(),
        },
    );

    assert_eq!(typed_expr, expected);
}

#[test]
fn parse_complex_recursive_query() {
    let input = r#"name == config.yaml AND yaml:..database.password ~= "secure.*""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr =
        Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

    // Left: Name predicate
    let name_pred = Expr::Predicate(Predicate::name(NamePredicate::FileName(
        StringMatcher::Equals("config.yaml".to_string()),
    )));

    // Right: Synthetic wrapper with YamlString predicate
    let matcher = StringMatcher::Regex(regex::Regex::new(r"secure.*").unwrap());
    let yaml_synthetic = build_expected_synthetic(
        "yaml",
        StructuredDataPredicate::YamlString {
            path: vec![
                PathComponent::RecursiveKey("database".to_string()),
                PathComponent::Key("password".to_string()),
            ],
            matcher,
        },
    );

    let expected = Expr::and(name_pred, yaml_synthetic);

    assert_eq!(typed_expr, expected);
}

#[test]
fn test_raw_string_preservation_yaml() {
    // Test that raw_string field preserves original input exactly
    let test_cases = vec![
        ("yaml:.count == 1_000_000", "1_000_000"),
        ("yaml:.permissions == 0755", "0755"),
        ("yaml:.enabled == yes", "yes"),
        ("yaml:.version == 1.0", "1.0"),
        ("yaml:.name == test", "test"),
    ];

    for (input, expected_raw) in test_cases {
        let raw_expr = RawParser::parse_raw_expr(input).unwrap();
        let typed_expr =
            Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

        // Extract the structured predicate from the synthetic wrapper
        // Structure: And(And(ext, size), Structured(...))
        match typed_expr {
            Expr::And(_, right) => match *right {
                Expr::Predicate(Predicate::Structured(StructuredDataPredicate::YamlValue {
                    raw_string,
                    ..
                })) => {
                    assert_eq!(raw_string, expected_raw, "Input: {}", input);
                }
                _ => panic!("Expected YamlValue predicate for input: {}", input),
            },
            _ => panic!("Expected synthetic wrapper for input: {}", input),
        }
    }
}

#[test]
fn test_raw_string_preservation_json() {
    // Test that raw_string field preserves original input for JSON
    let test_cases = vec![
        ("json:.version == 1.0", "1.0"),
        ("json:.count == 42", "42"),
        ("json:.name == test", "test"),
    ];

    for (input, expected_raw) in test_cases {
        let raw_expr = RawParser::parse_raw_expr(input).unwrap();
        let typed_expr =
            Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

        // Extract the structured predicate from the synthetic wrapper
        // Structure: And(And(ext, size), Structured(...))
        match typed_expr {
            Expr::And(_, right) => match *right {
                Expr::Predicate(Predicate::Structured(StructuredDataPredicate::JsonValue {
                    raw_string,
                    ..
                })) => {
                    assert_eq!(raw_string, expected_raw, "Input: {}", input);
                }
                _ => panic!("Expected JsonValue predicate for input: {}", input),
            },
            _ => panic!("Expected synthetic wrapper for input: {}", input),
        }
    }
}

#[test]
fn test_raw_string_preservation_toml() {
    // Test that raw_string field preserves original input for TOML
    let test_cases = vec![
        ("toml:.max_connections == 10_000", "10_000"),
        ("toml:.port == 8080", "8080"),
        ("toml:.enabled == true", "true"),
    ];

    for (input, expected_raw) in test_cases {
        let raw_expr = RawParser::parse_raw_expr(input).unwrap();
        let typed_expr =
            Typechecker::typecheck(raw_expr, input, &detect::RuntimeConfig::default()).unwrap();

        // Extract the structured predicate from the synthetic wrapper
        // Structure: And(And(ext, size), Structured(...))
        match typed_expr {
            Expr::And(_, right) => match *right {
                Expr::Predicate(Predicate::Structured(StructuredDataPredicate::TomlValue {
                    raw_string,
                    ..
                })) => {
                    assert_eq!(raw_string, expected_raw, "Input: {}", input);
                }
                _ => panic!("Expected TomlValue predicate for input: {}", input),
            },
            _ => panic!("Expected synthetic wrapper for input: {}", input),
        }
    }
}
