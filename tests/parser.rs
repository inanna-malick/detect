use detect::parser::test_utils::RawTestExpr;
use detect::parser::*;

#[test]
fn test_simple_predicate() {
    let result = RawParser::parse_raw_expr("name == foo").unwrap();
    let expected = RawTestExpr::string_predicate("name", "==", "foo");
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_quoted_values() {
    let result = RawParser::parse_raw_expr(r#"filename == "my file.txt""#).unwrap();
    let expected = RawTestExpr::string_predicate("filename", "==", "my file.txt");
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_single_quoted_values() {
    let result = RawParser::parse_raw_expr("filename == 'my file.txt'").unwrap();
    let expected = RawTestExpr::string_predicate("filename", "==", "my file.txt");
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_escape_sequences() {
    // Test double quote escapes
    let result = RawParser::parse_raw_expr(r#"name == "file\"with\"quotes""#).unwrap();
    let expected = RawTestExpr::string_predicate("name", "==", r#"file\"with\"quotes"#);
    assert_eq!(result.to_test_expr(), expected);

    // Test various escape sequences
    let result = RawParser::parse_raw_expr(r#"content == "line1\nline2\ttab\\backslash""#).unwrap();
    let expected =
        RawTestExpr::string_predicate("content", "==", r#"line1\nline2\ttab\\backslash"#);
    assert_eq!(result.to_test_expr(), expected);

    // Test single quote escapes
    let result = RawParser::parse_raw_expr(r"name == 'file\'with\'quotes'").unwrap();
    let expected = RawTestExpr::string_predicate("name", "==", r"file\'with\'quotes");
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_set_values() {
    let result = RawParser::parse_raw_expr("ext in [rs, js, ts]").unwrap();
    let expected = RawTestExpr::set_predicate("ext", "in", vec!["rs", "js", "ts"]);
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_mixed_set() {
    let result = RawParser::parse_raw_expr(r#"name in [README, "my file", config]"#).unwrap();
    let expected = RawTestExpr::set_predicate("name", "in", vec!["README", "my file", "config"]);
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_set_with_quotes_and_escapes() {
    let result = RawParser::parse_raw_expr(r#"name in ["file\"1", 'file\'2', plain]"#).unwrap();
    let expected =
        RawTestExpr::set_predicate("name", "in", vec![r#"file\"1"#, r"file\'2", "plain"]);
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_empty_set() {
    let result = RawParser::parse_raw_expr("ext in []").unwrap();
    let expected = RawTestExpr::set_predicate("ext", "in", vec![]);
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_boolean_logic() {
    let result = RawParser::parse_raw_expr("name == foo AND size > 1000").unwrap();
    let expected = RawTestExpr::and(
        RawTestExpr::string_predicate("name", "==", "foo"),
        RawTestExpr::string_predicate("size", ">", "1000"),
    );
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_or_logic() {
    let result = RawParser::parse_raw_expr("name == foo OR name == bar").unwrap();
    let expected = RawTestExpr::or(
        RawTestExpr::string_predicate("name", "==", "foo"),
        RawTestExpr::string_predicate("name", "==", "bar"),
    );
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_negation_variants() {
    // Test NOT keyword
    let result = RawParser::parse_raw_expr("NOT name == foo").unwrap();
    let expected = RawTestExpr::not(RawTestExpr::string_predicate("name", "==", "foo"));
    assert_eq!(result.to_test_expr(), expected);

    // Test ! symbol
    let result = RawParser::parse_raw_expr("! name == foo").unwrap();
    let expected = RawTestExpr::not(RawTestExpr::string_predicate("name", "==", "foo"));
    assert_eq!(result.to_test_expr(), expected);

    // Test escaped ! symbol
    let result = RawParser::parse_raw_expr("\\! name == foo").unwrap();
    let expected = RawTestExpr::not(RawTestExpr::string_predicate("name", "==", "foo"));
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_glob_pattern() {
    let result = RawParser::parse_raw_expr("*.rs").unwrap();
    let expected = RawTestExpr::glob("*.rs");
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_complex_glob_pattern() {
    let result = RawParser::parse_raw_expr("**/*.js").unwrap();
    let expected = RawTestExpr::glob("**/*.js");
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_operator_precedence() {
    // AND should bind tighter than OR
    let result = RawParser::parse_raw_expr("a == b OR c == d AND e == f").unwrap();
    let expected = RawTestExpr::or(
        RawTestExpr::string_predicate("a", "==", "b"),
        RawTestExpr::and(
            RawTestExpr::string_predicate("c", "==", "d"),
            RawTestExpr::string_predicate("e", "==", "f"),
        ),
    );
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_parentheses() {
    let result = RawParser::parse_raw_expr("(a == b OR c == d) AND e == f").unwrap();
    let expected = RawTestExpr::and(
        RawTestExpr::or(
            RawTestExpr::string_predicate("a", "==", "b"),
            RawTestExpr::string_predicate("c", "==", "d"),
        ),
        RawTestExpr::string_predicate("e", "==", "f"),
    );
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_complex_expression() {
    let result = RawParser::parse_raw_expr(
        r#"(name == "test.rs" OR ext in [js, ts]) AND NOT size > 1mb"#,
    )
    .unwrap();

    let expected = RawTestExpr::and(
        RawTestExpr::or(
            RawTestExpr::string_predicate("name", "==", "test.rs"),
            RawTestExpr::set_predicate("ext", "in", vec!["js", "ts"]),
        ),
        RawTestExpr::not(RawTestExpr::string_predicate("size", ">", "1mb")),
    );
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_all_operators() {
    let test_cases = vec![
        ("name == foo", "=="),
        ("name != foo", "!="),
        ("name ~= foo", "~="),
        ("name > foo", ">"),
        ("name < foo", "<"),
        ("name >= foo", ">="),
        ("name <= foo", "<="),
        ("name contains foo", "contains"),
        ("name in [foo]", "in"),
    ];

    for (input, expected_op) in test_cases {
        let result = RawParser::parse_raw_expr(input).unwrap();
        match result.to_test_expr() {
            RawTestExpr::Predicate(pred) => {
                assert_eq!(pred.operator, expected_op, "Failed for input: {}", input);
            }
            _ => panic!("Expected predicate for input: {}", input),
        }
    }
}

#[test]
fn test_complex_selectors() {
    let result = RawParser::parse_raw_expr("name == test.rs").unwrap();
    let expected = RawTestExpr::string_predicate("name", "==", "test.rs");
    assert_eq!(result.to_test_expr(), expected);

    let result = RawParser::parse_raw_expr("meta.size > 1000").unwrap();
    let expected = RawTestExpr::string_predicate("meta.size", ">", "1000");
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_case_insensitive_keywords() {
    let result = RawParser::parse_raw_expr("name == foo AND name == bar").unwrap();
    let result_upper = RawParser::parse_raw_expr("name == foo AND name == bar").unwrap();
    assert_eq!(result.to_test_expr(), result_upper.to_test_expr());

    let result = RawParser::parse_raw_expr("NOT name == foo").unwrap();
    let result_lower = RawParser::parse_raw_expr("not name == foo").unwrap();
    assert_eq!(result.to_test_expr(), result_lower.to_test_expr());
}

#[test]
fn test_syntax_errors() {
    // Missing value
    let result = RawParser::parse_raw_expr("name ==");
    assert!(result.is_err());

    // Missing operator
    let result = RawParser::parse_raw_expr("name foo");
    assert!(result.is_err());

    // Unclosed parentheses
    let result = RawParser::parse_raw_expr("(name == foo");
    assert!(result.is_err());

    // Unclosed set
    let result = RawParser::parse_raw_expr("name in [foo");
    assert!(result.is_err());

    // Unclosed quote
    let result = RawParser::parse_raw_expr(r#"name == "unclosed"#);
    assert!(result.is_err());
}

#[test]
fn test_invalid_escape_sequences() {
    // Since we're a syntax-only parser, we preserve escape sequences without validating them
    // This previously "invalid" escape sequence is now just preserved as-is
    let result = RawParser::parse_raw_expr(r#"name == "invalid\x""#);
    let expected = RawTestExpr::string_predicate("name", "==", r"invalid\x");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Unterminated string (this is actually a syntax error, not escape error)
    let result = RawParser::parse_raw_expr("name == \"unterminated");
    assert!(result.is_err());
}

#[test]
fn test_whitespace_handling() {
    let result1 = RawParser::parse_raw_expr("name==foo").unwrap();
    let result2 = RawParser::parse_raw_expr("name == foo").unwrap();
    let result3 = RawParser::parse_raw_expr("  name   ==   foo  ").unwrap();

    assert_eq!(result1.to_test_expr(), result2.to_test_expr());
    assert_eq!(result2.to_test_expr(), result3.to_test_expr());
}

#[test]
fn test_set_with_whitespace() {
    let result = RawParser::parse_raw_expr("ext in [ rs , js , ts ]").unwrap();
    let expected = RawTestExpr::set_predicate("ext", "in", vec!["rs", "js", "ts"]);
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_edge_cases() {
    // Empty string value
    let result = RawParser::parse_raw_expr(r#"name == """#).unwrap();
    let expected = RawTestExpr::string_predicate("name", "==", "");
    assert_eq!(result.to_test_expr(), expected);

    // Value with special characters
    let result = RawParser::parse_raw_expr("name == foo-bar_baz.txt").unwrap();
    let expected = RawTestExpr::string_predicate("name", "==", "foo-bar_baz.txt");
    assert_eq!(result.to_test_expr(), expected);

    // Selector with dots and underscores
    let result = RawParser::parse_raw_expr("path.name_with_underscores == foo").unwrap();
    let expected = RawTestExpr::string_predicate("path.name_with_underscores", "==", "foo");
    assert_eq!(result.to_test_expr(), expected);
}

// Bug: Reserved word substrings in bare values
// The grammar excludes "or", "and", "not" as substrings, not just as complete words.
// This breaks common words like "Error", "vendor", "Android", "cannot", etc.

#[test]
fn test_bare_value_with_or_substring() {
    // "Error" contains "or" substring - should parse but currently fails
    let result = RawParser::parse_raw_expr("content contains Error");
    assert!(result.is_ok(), "Should parse 'Error' as bare value");
    let expected = RawTestExpr::string_predicate("content", "contains", "Error");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Other common words with "or" substring
    let result = RawParser::parse_raw_expr("name == vendor");
    assert!(result.is_ok(), "Should parse 'vendor' as bare value");

    let result = RawParser::parse_raw_expr("content contains information");
    assert!(result.is_ok(), "Should parse 'information' as bare value");

    let result = RawParser::parse_raw_expr("name == sensor");
    assert!(result.is_ok(), "Should parse 'sensor' as bare value");
}

#[test]
fn test_bare_value_with_and_substring() {
    // "Android" contains "and" substring - should parse but currently fails
    let result = RawParser::parse_raw_expr("name contains Android");
    assert!(result.is_ok(), "Should parse 'Android' as bare value");
    let expected = RawTestExpr::string_predicate("name", "contains", "Android");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Other common words with "and" substring
    let result = RawParser::parse_raw_expr("name == standard");
    assert!(result.is_ok(), "Should parse 'standard' as bare value");

    let result = RawParser::parse_raw_expr("content contains candidate");
    assert!(result.is_ok(), "Should parse 'candidate' as bare value");

    let result = RawParser::parse_raw_expr("name == expand");
    assert!(result.is_ok(), "Should parse 'expand' as bare value");
}

#[test]
fn test_bare_value_with_not_substring() {
    // "cannot" contains "not" substring - should parse but currently fails
    let result = RawParser::parse_raw_expr("content contains cannot");
    assert!(result.is_ok(), "Should parse 'cannot' as bare value");
    let expected = RawTestExpr::string_predicate("content", "contains", "cannot");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Other common words with "not" substring
    let result = RawParser::parse_raw_expr("name == another");
    assert!(result.is_ok(), "Should parse 'another' as bare value");

    let result = RawParser::parse_raw_expr("content contains notation");
    assert!(result.is_ok(), "Should parse 'notation' as bare value");

    let result = RawParser::parse_raw_expr("name == denote");
    assert!(result.is_ok(), "Should parse 'denote' as bare value");
}

#[test]
fn test_bare_value_case_variations() {
    // Test case variations of words with reserved substrings
    let result = RawParser::parse_raw_expr("content contains error");
    assert!(result.is_ok(), "Should parse lowercase 'error'");

    let result = RawParser::parse_raw_expr("content contains ERROR");
    assert!(result.is_ok(), "Should parse uppercase 'ERROR'");

    let result = RawParser::parse_raw_expr("content contains ErRoR");
    assert!(result.is_ok(), "Should parse mixed case 'ErRoR'");
}

#[test]
fn test_reserved_words_as_complete_tokens_work_in_value_position() {
    // Reserved words as values should work - context disambiguates!
    // "name == or" searches for a file named "or", not a boolean operator
    let result = RawParser::parse_raw_expr("name == or");
    assert!(result.is_ok(), "Reserved word 'or' in value position should work");
    let expected = RawTestExpr::string_predicate("name", "==", "or");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    let result = RawParser::parse_raw_expr("name == and");
    assert!(result.is_ok(), "Reserved word 'and' in value position should work");
    let expected = RawTestExpr::string_predicate("name", "==", "and");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    let result = RawParser::parse_raw_expr("name == not");
    assert!(result.is_ok(), "Reserved word 'not' in value position should work");
    let expected = RawTestExpr::string_predicate("name", "==", "not");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Case variations
    let result = RawParser::parse_raw_expr("name == OR");
    assert!(result.is_ok(), "Reserved word 'OR' in value position should work");

    let result = RawParser::parse_raw_expr("name == AND");
    assert!(result.is_ok(), "Reserved word 'AND' in value position should work");

    let result = RawParser::parse_raw_expr("name == NOT");
    assert!(result.is_ok(), "Reserved word 'NOT' in value position should work");
}

#[test]
fn test_quoted_reserved_words_should_work() {
    // Quoted versions should always work, even for actual reserved words
    let result = RawParser::parse_raw_expr(r#"name == "or""#);
    assert!(result.is_ok(), "Quoted 'or' should parse");

    let result = RawParser::parse_raw_expr(r#"name == "and""#);
    assert!(result.is_ok(), "Quoted 'and' should parse");

    let result = RawParser::parse_raw_expr(r#"name == "not""#);
    assert!(result.is_ok(), "Quoted 'not' should parse");

    let result = RawParser::parse_raw_expr(r#"content contains "Error""#);
    assert!(result.is_ok(), "Quoted 'Error' should parse");
}
