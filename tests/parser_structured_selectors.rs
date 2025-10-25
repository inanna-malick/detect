//! Tests for structured data selector parsing (yaml:, json:, toml:)
//!
//! Validates that selectors like `yaml:.spec.replicas == 5` parse correctly
//! into StructuredData predicates with proper format, path, operator, and typed values.

use detect::expr::Expr;
use detect::parser::structured_path::PathComponent;
use detect::parser::typed::{DataFormat, StructuredOperator};
use detect::parser::{RawParser, Typechecker};
use detect::predicate::{Predicate, StructuredValue};

// ============================================================================
// Type Inference Tests
// ============================================================================

#[test]
fn parse_unquoted_number() {
    let input = "yaml:.port == 8080";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.format, DataFormat::Yaml);
            assert_eq!(pred.path, vec![PathComponent::Key("port".to_string())]);
            assert_eq!(pred.operator, StructuredOperator::Equals);
            assert_eq!(pred.value, StructuredValue::Number(8080));
        }
        _ => panic!("Expected StructuredData predicate, got: {:?}", typed_expr),
    }
}

#[test]
fn parse_quoted_number_as_string() {
    let input = r#"yaml:.port == "8080""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.value, StructuredValue::String("8080".to_string()));
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_unquoted_bool_true() {
    let input = "yaml:.enabled == true";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.value, StructuredValue::Bool(true));
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_unquoted_bool_false() {
    let input = "yaml:.enabled == false";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.value, StructuredValue::Bool(false));
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_quoted_bool_as_string() {
    let input = r#"yaml:.enabled == "true""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.value, StructuredValue::String("true".to_string()));
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_unquoted_string_fallback() {
    let input = "yaml:.name == api";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.value, StructuredValue::String("api".to_string()));
        }
        _ => panic!("Expected StructuredData predicate"),
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
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.format, DataFormat::Yaml);
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_json_format() {
    let input = r#"json:.version == "1.0.0""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.format, DataFormat::Json);
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_toml_format() {
    let input = "toml:.server.port == 8080";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.format, DataFormat::Toml);
        }
        _ => panic!("Expected StructuredData predicate"),
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
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.path, vec![PathComponent::Key("name".to_string())]);
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_nested_keys() {
    let input = "yaml:.spec.replicas == 5";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(
                pred.path,
                vec![
                    PathComponent::Key("spec".to_string()),
                    PathComponent::Key("replicas".to_string()),
                ]
            );
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_array_index_path() {
    let input = r#"json:[0].name == "first""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(
                pred.path,
                vec![
                    PathComponent::Index(0),
                    PathComponent::Key("name".to_string()),
                ]
            );
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_wildcard_array_path() {
    let input = "yaml:.items[*].id == 42";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(
                pred.path,
                vec![
                    PathComponent::Key("items".to_string()),
                    PathComponent::WildcardIndex,
                    PathComponent::Key("id".to_string()),
                ]
            );
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_complex_path() {
    let input = r#"yaml:.spec.containers[0].image == "nginx""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(
                pred.path,
                vec![
                    PathComponent::Key("spec".to_string()),
                    PathComponent::Key("containers".to_string()),
                    PathComponent::Index(0),
                    PathComponent::Key("image".to_string()),
                ]
            );
        }
        _ => panic!("Expected StructuredData predicate"),
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
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.operator, StructuredOperator::Greater);
            assert_eq!(pred.value, StructuredValue::Number(5));
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_comparison_greater_equal() {
    let input = "yaml:.count >= 10";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.operator, StructuredOperator::GreaterOrEqual);
            assert_eq!(pred.value, StructuredValue::Number(10));
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_comparison_less() {
    let input = "yaml:.count < 100";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.operator, StructuredOperator::Less);
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_regex_operator() {
    let input = r#"yaml:.name ~= "test.*""#;
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.operator, StructuredOperator::Matches);
            assert_eq!(pred.value, StructuredValue::String("test.*".to_string()));
        }
        _ => panic!("Expected StructuredData predicate"),
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
fn error_invalid_path_double_dot() {
    let input = "yaml:..invalid == 5";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let result = Typechecker::typecheck(raw_expr, input);

    assert!(result.is_err(), "Double dots in path should fail");
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
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.format, DataFormat::Json);
            assert_eq!(
                pred.path,
                vec![
                    PathComponent::Key("engines".to_string()),
                    PathComponent::Key("node".to_string()),
                ]
            );
            // Note: Quoted "18" becomes String, not Number
            assert_eq!(pred.value, StructuredValue::String("18".to_string()));
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_toml_privileged_port() {
    let input = "toml:.server.port < 1024";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.format, DataFormat::Toml);
            assert_eq!(pred.operator, StructuredOperator::Less);
            assert_eq!(pred.value, StructuredValue::Number(1024));
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_negative_number() {
    let input = "yaml:.offset == -10";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.value, StructuredValue::Number(-10));
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}

#[test]
fn parse_zero() {
    let input = "yaml:.count == 0";
    let raw_expr = RawParser::parse_raw_expr(input).unwrap();
    let typed_expr = Typechecker::typecheck(raw_expr, input).unwrap();

    match typed_expr {
        Expr::Predicate(Predicate::StructuredData(pred)) => {
            assert_eq!(pred.value, StructuredValue::Number(0));
        }
        _ => panic!("Expected StructuredData predicate"),
    }
}
