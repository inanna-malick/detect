use detect::parser::test_utils::RawTestExpr;
use detect::parser::*;

// ==============================================================================

#[test]
fn test_unterminated_double_quote() {
    // Unterminated double quote at various positions
    let result = RawParser::parse_raw_expr(r#"contents ~= "a"#);
    assert!(result.is_err(), "Unterminated double quote should fail");

    let result = RawParser::parse_raw_expr(r#"name == "test"#);
    assert!(result.is_err(), "Unterminated double quote should fail");

    let result = RawParser::parse_raw_expr(r#"ext == "some long string"#);
    assert!(result.is_err(), "Unterminated double quote should fail");
}

#[test]
fn test_unterminated_single_quote() {
    // Unterminated single quote at various positions
    let result = RawParser::parse_raw_expr("contents ~= 'a");
    assert!(result.is_err(), "Unterminated single quote should fail");

    let result = RawParser::parse_raw_expr("name == 'test");
    assert!(result.is_err(), "Unterminated single quote should fail");

    let result = RawParser::parse_raw_expr("ext == 'foo bar baz");
    assert!(result.is_err(), "Unterminated single quote should fail");
}

#[test]
fn test_stray_double_quote_after_value() {
    // Stray double quote immediately after valid bare token
    let result = RawParser::parse_raw_expr(r#"contents ~= a""#);
    assert!(result.is_err(), "Stray double quote should fail");

    let result = RawParser::parse_raw_expr(r#"name == foo""#);
    assert!(result.is_err(), "Stray double quote should fail");

    let result = RawParser::parse_raw_expr(r#"ext == test.rs""#);
    assert!(result.is_err(), "Stray double quote should fail");
}

#[test]
fn test_stray_single_quote_after_value() {
    // Stray single quote immediately after valid bare token
    let result = RawParser::parse_raw_expr("contents ~= a'");
    assert!(result.is_err(), "Stray single quote should fail");

    let result = RawParser::parse_raw_expr("name == foo'");
    assert!(result.is_err(), "Stray single quote should fail");

    let result = RawParser::parse_raw_expr("ext == test.rs'");
    assert!(result.is_err(), "Stray single quote should fail");
}

#[test]
fn test_quote_errors_in_complex_expressions() {
    // Unterminated quote in boolean expressions
    let result = RawParser::parse_raw_expr(r#"ext == rs AND name == "unterminated"#);
    assert!(
        result.is_err(),
        "Unterminated quote in AND expression should fail"
    );

    let result = RawParser::parse_raw_expr(r#"ext == rs OR name == 'foo"#);
    assert!(
        result.is_err(),
        "Unterminated quote in OR expression should fail"
    );

    // Stray quote in boolean expressions
    let result = RawParser::parse_raw_expr(r#"ext == rs AND name == foo""#);
    assert!(result.is_err(), "Stray quote in AND expression should fail");

    let result = RawParser::parse_raw_expr(r#"ext == rs OR name == bar'"#);
    assert!(result.is_err(), "Stray quote in OR expression should fail");
}

#[test]
fn test_lone_quote_as_value() {
    // Single quote character alone should be unterminated
    let result = RawParser::parse_raw_expr(r#"contents ~= ""#);
    assert!(result.is_err(), "Lone double quote should fail");

    let result = RawParser::parse_raw_expr("contents ~= '");
    assert!(result.is_err(), "Lone single quote should fail");
}

#[test]
fn test_properly_quoted_strings_still_work() {
    // Verify that proper quotes continue to work after adding error detection
    let result = RawParser::parse_raw_expr(r#"name == "properly quoted""#);
    assert!(result.is_ok(), "Properly quoted double quotes should work");
    let expected = RawTestExpr::quoted_predicate("name", "==", "properly quoted");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    let result = RawParser::parse_raw_expr("name == 'properly quoted'");
    assert!(result.is_ok(), "Properly quoted single quotes should work");
    let expected = RawTestExpr::quoted_predicate("name", "==", "properly quoted");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // With spaces
    let result = RawParser::parse_raw_expr(r#"content ~= "test string with spaces""#);
    assert!(result.is_ok(), "Quoted string with spaces should work");
    let expected = RawTestExpr::quoted_predicate("content", "~=", "test string with spaces");
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

#[test]
fn test_edge_case_empty_inputs() {
    // Empty string
    let result = RawParser::parse_raw_expr("");
    assert!(result.is_err(), "Empty string should fail");

    // Just whitespace
    let result = RawParser::parse_raw_expr("   ");
    assert!(result.is_err(), "Whitespace only should fail");

    // Just operators
    let result = RawParser::parse_raw_expr("==");
    assert!(result.is_err(), "Operator only should fail");
}

#[test]
fn test_malformed_sets() {
    // Set with no closing bracket
    let result = RawParser::parse_raw_expr("name in [foo, bar");
    assert!(result.is_err(), "Unclosed set should fail");

    // Set with no opening bracket - actually just parses as "foo" bare token
    let result = RawParser::parse_raw_expr("name in foo, bar]");
    assert!(result.is_err(), "Malformed syntax should fail");

    // Nested sets - simplified grammar now allows this to parse (will fail at typecheck)
    let result = RawParser::parse_raw_expr("name in [foo, [bar]]");
    assert!(
        result.is_ok(),
        "Simplified grammar allows nested brackets (typecheck will handle validity)"
    );

    // Set with trailing comma - now allowed, typechecker filters empty items
    let result = RawParser::parse_raw_expr("name in [foo, bar,]");
    assert!(result.is_ok(), "Trailing comma is now allowed");

    // Set with only commas - parses as empty set after filtering
    let result = RawParser::parse_raw_expr("name in [,,,]");
    assert!(result.is_ok(), "Only commas parses as empty set");
}

#[test]
fn test_malformed_quotes() {
    // Mismatched quotes
    let result = RawParser::parse_raw_expr(r#"name == "foo'"#);
    assert!(result.is_err(), "Mismatched quotes should fail");

    let result = RawParser::parse_raw_expr(r#"name == 'foo""#);
    assert!(result.is_err(), "Mismatched quotes should fail");

    // Escaped quote at end without closing
    let result = RawParser::parse_raw_expr(r#"name == "foo\""#);
    assert!(result.is_err(), "Escaped quote at end should fail");

    // Multiple quotes
    let result = RawParser::parse_raw_expr(r#"name == ""foo""#);
    assert!(result.is_err(), "Double quotes should fail");

    // Quote in the middle of bare value
    let result = RawParser::parse_raw_expr(r#"name == fo"o"#);
    assert!(result.is_err(), "Quote in middle should fail");
}

#[test]
fn test_boolean_logic_edge_cases() {
    // Incomplete boolean expressions
    let result = RawParser::parse_raw_expr("name == foo AND");
    assert!(result.is_err(), "Incomplete AND should fail");

    let result = RawParser::parse_raw_expr("OR name == foo");
    assert!(result.is_err(), "Leading OR should fail");

    let result = RawParser::parse_raw_expr("NOT");
    assert!(result.is_err(), "Standalone NOT should fail");

    // Multiple consecutive operators
    let result = RawParser::parse_raw_expr("name == foo AND OR bar == baz");
    assert!(result.is_err(), "AND OR should fail");

    let result = RawParser::parse_raw_expr("name == foo NOT AND bar == baz");
    assert!(result.is_err(), "NOT AND should fail");

    // Multiple NOT
    let result = RawParser::parse_raw_expr("NOT NOT name == foo");
    let expected = RawTestExpr::not(RawTestExpr::not(RawTestExpr::string_predicate(
        "name", "==", "foo",
    )));
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Mixed NOT usage - prefix operator with NOT as value
    let result = RawParser::parse_raw_expr("NOT filename == NOT");
    let expected = RawTestExpr::not(RawTestExpr::string_predicate("filename", "==", "NOT"));
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

#[test]
fn test_parentheses_edge_cases() {
    // Unmatched parentheses
    let result = RawParser::parse_raw_expr("((name == foo)");
    assert!(result.is_err(), "Unmatched opening parens should fail");

    let result = RawParser::parse_raw_expr("(name == foo))");
    assert!(result.is_err(), "Unmatched closing parens should fail");

    // Empty parentheses
    let result = RawParser::parse_raw_expr("()");
    assert!(result.is_err(), "Empty parentheses should fail");

    // Parentheses around operators
    let result = RawParser::parse_raw_expr("name (==) foo");
    assert!(result.is_err(), "Parentheses around operators should fail");

    // Basic nested parentheses (deep nesting tested in test_extreme_nesting_limits)
    let result = RawParser::parse_raw_expr("((name == foo))");
    let expected = RawTestExpr::string_predicate("name", "==", "foo");
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

