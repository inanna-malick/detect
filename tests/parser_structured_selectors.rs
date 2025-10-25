//! Tests for structured data selector parsing (yaml:, json:, toml:)
//!
//! Validates that selectors like `yaml:.spec.replicas == 5` parse correctly
//! into StructuredData predicates with proper format, path, operator, and native typed values.

use detect::expr::Expr;
use detect::parser::structured_path::PathComponent;
use detect::parser::typed::{DataFormat, StructuredOperator};
use detect::parser::{RawParser, Typechecker};
use detect::predicate::{Predicate, StructuredDataPredicate};

// ============================================================================
// Type Inference Tests
// ============================================================================

#[test]
fn parse_unquoted_number() {
    let input = "yaml:.port == 8080";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { path, operator, value }
        )) => {
            assert_eq!(path, vec![PathComponent::Key("port".to_string())]);
            assert_eq!(operator, StructuredOperator::Equals);
            // Verify it parsed as integer
            assert!(matches!(value, yaml_rust::Yaml::Integer(8080)));
        }
        _ => panic!("Expected YamlValue predicate, got: {:?}", typed_expr),
    }
}

#[test]
fn parse_quoted_number_as_string() {
    let input = r#"yaml:.port == "8080""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { value, .. }
        )) => {
            // Quotes are transparent - "8080" parses as integer via YAML
            assert!(matches!(value, yaml_rust::Yaml::Integer(8080)));
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_unquoted_bool_true() {
    let input = "yaml:.enabled == true";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { value, .. }
        )) => {
            assert!(matches!(value, yaml_rust::Yaml::Boolean(true)));
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_unquoted_bool_false() {
    let input = "yaml:.enabled == false";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { value, .. }
        )) => {
            assert!(matches!(value, yaml_rust::Yaml::Boolean(false)));
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_quoted_bool_as_string() {
    let input = r#"yaml:.enabled == "true""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { value, .. }
        )) => {
            // Quotes are transparent - "true" parses as boolean via YAML
            assert!(matches!(value, yaml_rust::Yaml::Boolean(true)));
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_unquoted_string_fallback() {
    let input = "yaml:.name == api";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { value, .. }
        )) => {
            // Bareword "api" doesn't parse as YAML, falls back to string
            assert!(matches!(value, yaml_rust::Yaml::String(s) if s == "api"));
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

// ============================================================================
// Format Detection
// ============================================================================

#[test]
fn parse_yaml_format() {
    let input = "yaml:.name == test";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(StructuredDataPredicate::YamlValue { .. })) => {
            // Success - it's a YamlValue variant
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_json_format() {
    let input = r#"json:.version == "1.0.0""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(StructuredDataPredicate::JsonValue { .. })) => {
            // Success - it's a JsonValue variant
        }
        _ => panic!("Expected JsonValue predicate"),
    }
}

#[test]
fn parse_toml_format() {
    let input = "toml:.server.port == 8080";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(StructuredDataPredicate::TomlValue { .. })) => {
            // Success - it's a TomlValue variant
        }
        _ => panic!("Expected TomlValue predicate"),
    }
}

// ============================================================================
// Path Parsing
// ============================================================================

#[test]
fn parse_single_key_path() {
    let input = "yaml:.name == test";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { path, .. }
        )) => {
            assert_eq!(path, vec![PathComponent::Key("name".to_string())]);
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_nested_keys() {
    let input = "yaml:.spec.replicas == 5";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { path, .. }
        )) => {
            assert_eq!(
                path,
                vec![
                    PathComponent::Key("spec".to_string()),
                    PathComponent::Key("replicas".to_string()),
                ]
            );
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_array_index_path() {
    let input = r#"json:[0].name == "first""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::JsonValue { path, .. }
        )) => {
            assert_eq!(
                path,
                vec![
                    PathComponent::Index(0),
                    PathComponent::Key("name".to_string()),
                ]
            );
        }
        _ => panic!("Expected JsonValue predicate"),
    }
}

#[test]
fn parse_wildcard_array_path() {
    let input = "yaml:.items[*].id == 42";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { path, .. }
        )) => {
            assert_eq!(
                path,
                vec![
                    PathComponent::Key("items".to_string()),
                    PathComponent::WildcardIndex,
                    PathComponent::Key("id".to_string()),
                ]
            );
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_complex_path() {
    let input = r#"yaml:.spec.containers[0].image == "nginx""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { path, .. }
        )) => {
            assert_eq!(
                path,
                vec![
                    PathComponent::Key("spec".to_string()),
                    PathComponent::Key("containers".to_string()),
                    PathComponent::Index(0),
                    PathComponent::Key("image".to_string()),
                ]
            );
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

// ============================================================================
// Operator Parsing
// ============================================================================

#[test]
fn parse_comparison_greater() {
    let input = "yaml:.count > 5";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { operator, value, .. }
        )) => {
            assert_eq!(operator, StructuredOperator::Greater);
            assert!(matches!(value, yaml_rust::Yaml::Integer(5)));
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_comparison_greater_equal() {
    let input = "yaml:.count >= 10";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { operator, value, .. }
        )) => {
            assert_eq!(operator, StructuredOperator::GreaterOrEqual);
            assert!(matches!(value, yaml_rust::Yaml::Integer(10)));
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_comparison_less() {
    let input = "yaml:.count < 100";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { operator, .. }
        )) => {
            assert_eq!(operator, StructuredOperator::Less);
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_regex_operator() {
    let input = r#"yaml:.name ~= "test.*""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlString { matcher, .. }
        )) => {
            // Verify it's a regex matcher (StringMatcher::Regex)
            assert!(matches!(matcher, detect::predicate::StringMatcher::Regex(_)));
        }
        _ => panic!("Expected YamlString predicate with regex"),
    }
}

// ============================================================================
// Error Cases
// ============================================================================

#[test]
fn error_comparison_with_quoted_string() {
    let input = r#"yaml:.count > "foo""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let result = Typechecker::typecheck(raw_expr, input);

    assert!(result.is_err(), "Comparison with quoted string should fail");
}

#[test]
fn error_comparison_with_invalid_number() {
    let input = "yaml:.count > notanumber";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let result = Typechecker::typecheck(raw_expr, input);

    assert!(result.is_err(), "Comparison with non-numeric value should fail");
}

#[test]
fn error_invalid_path_triple_dot() {
    let input = "yaml:...invalid == 5";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let result = Typechecker::typecheck(raw_expr, input);

    assert!(result.is_err(), "Triple dots in path should fail");
}

// ============================================================================
// Boolean Logic
// ============================================================================

#[test]
fn parse_and_expression() {
    let input = r#"yaml:.kind == "Deployment" AND yaml:.spec.replicas > 5"#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    // Verify it's an AND expression with two StructuredData predicates
    match typed_expr {
        Expr::And(left, right) => {
            assert!(matches!(*left, Expr::Predicate(Predicate::StructuredData(_))));
            assert!(matches!(*right, Expr::Predicate(Predicate::StructuredData(_))));
        }
        _ => panic!("Expected AND expression"),
    }
}

#[test]
fn parse_mixed_selectors() {
    let input = "name == config.yaml AND yaml:.debug == true";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    // Verify mix of Name and StructuredData predicates
    match typed_expr {
        Expr::And(left, right) => {
            assert!(matches!(*left, Expr::Predicate(Predicate::Name(_))));
            assert!(matches!(*right, Expr::Predicate(Predicate::StructuredData(_))));
        }
        _ => panic!("Expected AND expression with mixed predicates"),
    }
}

// ============================================================================
// Real-World Examples
// ============================================================================

#[test]
fn parse_k8s_deployment_query() {
    let input = r#"yaml:.kind == "Deployment" AND yaml:.spec.replicas > 5"#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let _typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();
    // Success if it parses without error
}

#[test]
fn parse_package_json_node_version() {
    let input = r#"json:.engines.node == "18""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::JsonValue { path, value, .. }
        )) => {
            assert_eq!(
                path,
                vec![
                    PathComponent::Key("engines".to_string()),
                    PathComponent::Key("node".to_string()),
                ]
            );
            // Quotes are transparent - "18" parses as number via JSON
            assert!(matches!(value, serde_json::Value::Number(n) if n.as_i64() == Some(18)));
        }
        _ => panic!("Expected JsonValue predicate"),
    }
}

#[test]
fn parse_toml_privileged_port() {
    let input = "toml:.server.port < 1024";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::TomlValue { operator, value, .. }
        )) => {
            assert_eq!(operator, StructuredOperator::Less);
            assert!(matches!(value, toml::Value::Integer(1024)));
        }
        _ => panic!("Expected TomlValue predicate"),
    }
}

#[test]
fn parse_negative_number() {
    let input = "yaml:.offset == -10";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { value, .. }
        )) => {
            assert!(matches!(value, yaml_rust::Yaml::Integer(-10)));
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_zero() {
    let input = "yaml:.count == 0";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { value, .. }
        )) => {
            assert!(matches!(value, yaml_rust::Yaml::Integer(0)));
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

// ============================================================================
// Recursive Descent Tests
// ============================================================================

#[test]
fn parse_simple_recursive_descent() {
    let input = "yaml:..password == secret";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { path, value, .. }
        )) => {
            assert_eq!(
                path,
                vec![PathComponent::RecursiveKey("password".to_string())]
            );
            assert!(matches!(value, yaml_rust::Yaml::String(s) if s == "secret"));
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_recursive_debug_flag() {
    let input = "yaml:..debug == true";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { path, value, .. }
        )) => {
            assert_eq!(
                path,
                vec![PathComponent::RecursiveKey("debug".to_string())]
            );
            assert!(matches!(value, yaml_rust::Yaml::Boolean(true)));
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_key_then_recursive() {
    let input = "yaml:.user..password == admin";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { path, .. }
        )) => {
            assert_eq!(
                path,
                vec![
                    PathComponent::Key("user".to_string()),
                    PathComponent::RecursiveKey("password".to_string()),
                ]
            );
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_recursive_then_path() {
    let input = r#"yaml:..metadata.labels.app == "web""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { path, value, .. }
        )) => {
            assert_eq!(
                path,
                vec![
                    PathComponent::RecursiveKey("metadata".to_string()),
                    PathComponent::Key("labels".to_string()),
                    PathComponent::Key("app".to_string()),
                ]
            );
            assert!(matches!(value, yaml_rust::Yaml::String(s) if s == "web"));
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_recursive_with_comparison() {
    let input = "toml:..port < 1024";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::TomlValue { path, operator, value, .. }
        )) => {
            assert_eq!(
                path,
                vec![PathComponent::RecursiveKey("port".to_string())]
            );
            assert_eq!(operator, StructuredOperator::Less);
            assert!(matches!(value, toml::Value::Integer(1024)));
        }
        _ => panic!("Expected TomlValue predicate"),
    }
}

#[test]
fn parse_recursive_with_regex() {
    let input = r#"json:..email ~= ".*@example\.com""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::JsonString { path, matcher }
        )) => {
            assert_eq!(
                path,
                vec![PathComponent::RecursiveKey("email".to_string())]
            );
            assert!(matches!(matcher, detect::predicate::StringMatcher::Regex(_)));
        }
        _ => panic!("Expected JsonString predicate with regex"),
    }
}

#[test]
fn parse_multiple_recursive() {
    let input = "yaml:..config..database == postgres";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { path, .. }
        )) => {
            assert_eq!(
                path,
                vec![
                    PathComponent::RecursiveKey("config".to_string()),
                    PathComponent::RecursiveKey("database".to_string()),
                ]
            );
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_recursive_with_array_index() {
    let input = "yaml:..users[0].name == alice";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { path, .. }
        )) => {
            assert_eq!(
                path,
                vec![
                    PathComponent::RecursiveKey("users".to_string()),
                    PathComponent::Index(0),
                    PathComponent::Key("name".to_string()),
                ]
            );
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_recursive_with_wildcard() {
    let input = "yaml:..items[*].id == 42";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(
            StructuredDataPredicate::YamlValue { path, .. }
        )) => {
            assert_eq!(
                path,
                vec![
                    PathComponent::RecursiveKey("items".to_string()),
                    PathComponent::WildcardIndex,
                    PathComponent::Key("id".to_string()),
                ]
            );
        }
        _ => panic!("Expected YamlValue predicate"),
    }
}

#[test]
fn parse_complex_recursive_query() {
    let input = r#"name == config.yaml AND yaml:..database.password ~= "secure.*""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    // Verify AND expression with mixed predicates
    match typed_expr {
        Expr::And(left, right) => {
            assert!(matches!(*left, Expr::Predicate(Predicate::Name(_))));
            if let Expr::Predicate(Predicate::StructuredData(
                StructuredDataPredicate::YamlString { path, .. }
            )) = *right {
                assert_eq!(
                    path,
                    vec![
                        PathComponent::RecursiveKey("database".to_string()),
                        PathComponent::Key("password".to_string()),
                    ]
                );
            } else {
                panic!("Expected YamlString predicate on right");
            }
        }
        _ => panic!("Expected AND expression"),
    }
}
