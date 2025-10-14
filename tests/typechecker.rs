use detect::expr::Expr;
use detect::parser::error::DetectError as TypecheckError;
use detect::parser::{RawParser, Typechecker};
use detect::predicate::{
    parse_time_value, Bound, MetadataPredicate, NamePredicate, NumberMatcher, Predicate,
    StreamingCompiledContentPredicate, StringMatcher, TimeMatcher,
};

/// Helper function to parse and typecheck an expression
fn parse_and_typecheck(expr: &str) -> Result<Expr<Predicate>, TypecheckError> {
    let raw_expr = RawParser::parse_raw_expr(expr).unwrap();
    Typechecker::typecheck(raw_expr, expr)
}

use std::collections::HashSet;

#[test]
fn test_selector_recognition() {
    // Test path selector (full absolute path)
    let typed = parse_and_typecheck("path == foo").unwrap();
    let expected = Expr::Predicate(Predicate::name(NamePredicate::FullPath(
        StringMatcher::Equals("foo".to_string()),
    )));
    assert_eq!(typed, expected);

    // Test name selector (full filename with extension)
    let typed = parse_and_typecheck("name == test.rs").unwrap();
    let expected = Expr::Predicate(Predicate::name(NamePredicate::FileName(
        StringMatcher::Equals("test.rs".to_string()),
    )));
    assert_eq!(typed, expected);

    // Test extension selector
    let typed = parse_and_typecheck("ext == rs").unwrap();
    let expected = Expr::Predicate(Predicate::name(NamePredicate::Extension(
        StringMatcher::Equals("rs".to_string()),
    )));
    assert_eq!(typed, expected);
}

#[test]
fn test_unknown_selector() {
    let error = parse_and_typecheck("unknown_selector == foo").unwrap_err();
    assert!(
        matches!(error, TypecheckError::UnknownSelector { selector, .. } if selector == "unknown_selector")
    );
}

#[test]
fn test_operator_validation() {
    // Valid string operator
    assert!(parse_and_typecheck("name == foo").is_ok());

    // Invalid operator for string selector (numeric operator on string type)
    let error = parse_and_typecheck("name > foo").unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));

    // Valid numeric operator
    assert!(parse_and_typecheck("size > 1000").is_ok());

    // Invalid operator for numeric selector (string operator on numeric type)
    let error = parse_and_typecheck("size contains foo").unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));
}

#[test]
fn test_string_value_parsing() {
    // Equals - "name" now means full filename with extension
    let typed = parse_and_typecheck("name == test.rs").unwrap();
    let expected = Expr::Predicate(Predicate::name(NamePredicate::FileName(
        StringMatcher::Equals("test.rs".to_string()),
    )));
    assert_eq!(typed, expected);

    // Not equals
    let typed = parse_and_typecheck("name != test.rs").unwrap();
    let expected = Expr::Predicate(Predicate::name(NamePredicate::FileName(
        StringMatcher::NotEquals("test.rs".to_string()),
    )));
    assert_eq!(typed, expected);

    // Contains
    let typed = parse_and_typecheck("path contains src").unwrap();
    let expected = Expr::Predicate(Predicate::name(NamePredicate::FullPath(
        StringMatcher::Contains("src".to_string()),
    )));
    assert_eq!(typed, expected);
}

#[test]
fn test_set_value_parsing() {
    let typed = parse_and_typecheck("ext in [rs, js, ts]").unwrap();
    let expected_set: HashSet<String> = vec!["rs", "js", "ts"]
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    let expected = Expr::Predicate(Predicate::name(NamePredicate::Extension(
        StringMatcher::In(expected_set),
    )));
    assert_eq!(typed, expected);
}

#[test]
fn test_regex_parsing() {
    let typed = parse_and_typecheck("content ~= TODO.*").unwrap();
    let expected_content = StreamingCompiledContentPredicate::new("TODO.*".to_string()).unwrap();
    let expected = Expr::Predicate(Predicate::contents(expected_content));
    assert_eq!(typed, expected);
}

#[test]
fn test_simple_unquoted_quantifiers_typecheck() {
    // Test that unquoted patterns with quantifiers compile to valid regex predicates
    let test_cases = vec![
        ("content ~= [0-9]+", "[0-9]+"),
        ("content ~= [a-z]*", "[a-z]*"),
        ("content ~= [A-Z]?", "[A-Z]?"),
        ("content ~= [0-9]{2,4}", "[0-9]{2,4}"),
        ("content ~= (foo|bar)+", "(foo|bar)+"),
        ("content ~= (test)*", "(test)*"),
        ("content ~= (a|b){2,3}", "(a|b){2,3}"),
    ];

    for (expr, expected_pattern) in test_cases {
        let typed = parse_and_typecheck(expr).unwrap();
        let expected = Expr::Predicate(Predicate::contents(
            StreamingCompiledContentPredicate::new(expected_pattern.to_string()).unwrap(),
        ));
        assert_eq!(typed, expected, "Mismatch for: {}", expr);
    }
}

#[test]
fn test_complex_unquoted_patterns_typecheck() {
    // Test complex multi-bracket patterns and patterns with trailing chars
    let test_cases = vec![
        ("content ~= [a-z]+[A-Z]+", "[a-z]+[A-Z]+"),
        ("content ~= [0-9]+\\.[0-9]+", "[0-9]+\\.[0-9]+"),
        ("content ~= [a-z]+[A-Z]+[0-9]+", "[a-z]+[A-Z]+[0-9]+"),
        ("content ~= [0-9]+@domain", "[0-9]+@domain"),
        ("content ~= [0-9]+-[0-9]+", "[0-9]+-[0-9]+"),
        ("content ~= [a-z]+_suffix", "[a-z]+_suffix"),
        ("content ~= [0-9]+:port", "[0-9]+:port"),
        (
            "content ~= [a-z]+@[a-z]+\\.[a-z]+",
            "[a-z]+@[a-z]+\\.[a-z]+",
        ),
        (
            "content ~= \\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}",
            "\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}",
        ),
    ];

    for (expr, expected_pattern) in test_cases {
        let typed = parse_and_typecheck(expr).unwrap();
        let expected = Expr::Predicate(Predicate::contents(
            StreamingCompiledContentPredicate::new(expected_pattern.to_string()).unwrap(),
        ));
        assert_eq!(typed, expected, "Mismatch for: {}", expr);
    }
}

#[test]
fn test_quoted_unquoted_equivalence() {
    // Verify that quoted and unquoted patterns produce identical results
    let test_cases = vec![
        ("content ~= [0-9]+", r#"content ~= "[0-9]+""#),
        ("content ~= [a-z]+[A-Z]+", r#"content ~= "[a-z]+[A-Z]+""#),
        ("content ~= (foo|bar)+", r#"content ~= "(foo|bar)+""#),
        ("content ~= [0-9]+@domain", r#"content ~= "[0-9]+@domain""#),
    ];

    for (unquoted_expr, quoted_expr) in test_cases {
        let unquoted = parse_and_typecheck(unquoted_expr).unwrap();
        let quoted = parse_and_typecheck(quoted_expr).unwrap();
        assert_eq!(
            unquoted, quoted,
            "Quoted/unquoted mismatch for: {}",
            unquoted_expr
        );
    }
}

#[test]
fn test_unquoted_quantifiers_in_boolean_expressions() {
    // Test that unquoted quantifiers work correctly in boolean context

    // Simple AND expression
    let typed = parse_and_typecheck("content ~= [0-9]+ AND size > 1kb").unwrap();
    let left = Expr::Predicate(Predicate::contents(
        StreamingCompiledContentPredicate::new("[0-9]+".to_string()).unwrap(),
    ));
    let right = Expr::Predicate(Predicate::meta(MetadataPredicate::Filesize(
        NumberMatcher::In(Bound::Left(1025..)), // 1kb + 1
    )));
    let expected = Expr::and(left, right);
    assert_eq!(typed, expected);

    // OR expression with complex pattern
    let typed = parse_and_typecheck("content ~= [a-z]+[A-Z]+ OR name == test").unwrap();
    let left = Expr::Predicate(Predicate::contents(
        StreamingCompiledContentPredicate::new("[a-z]+[A-Z]+".to_string()).unwrap(),
    ));
    let right = Expr::Predicate(Predicate::name(NamePredicate::FileName(
        StringMatcher::Equals("test".to_string()),
    )));
    let expected = Expr::or(left, right);
    assert_eq!(typed, expected);

    // NOT expression
    let typed = parse_and_typecheck("NOT content ~= [0-9]+").unwrap();
    let inner = Expr::Predicate(Predicate::contents(
        StreamingCompiledContentPredicate::new("[0-9]+".to_string()).unwrap(),
    ));
    let expected = Expr::negate(inner);
    assert_eq!(typed, expected);

    // Complex nested expression
    let typed = parse_and_typecheck("(content ~= [0-9]+ OR content ~= [a-z]+) AND NOT type == dir")
        .unwrap();
    let lhs_left = Expr::Predicate(Predicate::contents(
        StreamingCompiledContentPredicate::new("[0-9]+".to_string()).unwrap(),
    ));
    let lhs_right = Expr::Predicate(Predicate::contents(
        StreamingCompiledContentPredicate::new("[a-z]+".to_string()).unwrap(),
    ));
    let lhs = Expr::or(lhs_left, lhs_right);
    let rhs_inner = Expr::Predicate(Predicate::meta(MetadataPredicate::Type(
        StringMatcher::Equals("dir".to_string()),
    )));
    let rhs = Expr::negate(rhs_inner);
    let expected = Expr::and(lhs, rhs);
    assert_eq!(typed, expected);
}

#[test]
fn test_invalid_regex_patterns_fail_typecheck() {
    // Parser accepts these, but typechecker should reject when regex compilation fails
    // Note: Many "malformed" patterns like [0-9]++ are actually valid - the second +
    // is interpreted as a literal character. Only truly invalid regex syntax fails.
    let invalid_patterns = vec![
        "content ~= [z-a]",      // Invalid character class range (reversed)
        "content ~= (?P<>test)", // Empty capture group name
        "content ~= (?#",        // Unclosed comment
    ];

    for expr in invalid_patterns {
        let result = parse_and_typecheck(expr);
        assert!(
            matches!(result, Err(TypecheckError::InvalidValue { .. })),
            "Should fail with InvalidValue error for: {}, got: {:?}",
            expr,
            result
        );
    }
}

#[test]
fn test_size_value_parsing() {
    // Plain number (size > 1000 becomes range 1001..)
    let typed = parse_and_typecheck("size > 1000").unwrap();
    let expected = Expr::Predicate(Predicate::meta(MetadataPredicate::Filesize(
        NumberMatcher::In(Bound::Left(1001..)),
    )));
    assert_eq!(typed, expected);

    // Size with unit (1mb = 1048576 bytes, so > 1mb becomes range 1048577..)
    let typed = parse_and_typecheck("size > 1mb").unwrap();
    let expected = Expr::Predicate(Predicate::meta(MetadataPredicate::Filesize(
        NumberMatcher::In(Bound::Left(1048577..)),
    )));
    assert_eq!(typed, expected);
}

#[test]
fn test_temporal_value_parsing() {
    // Relative time - just verify it's the right structure, times will differ slightly
    let typed = parse_and_typecheck("modified > -7d").unwrap();
    assert!(
        matches!(typed, Expr::Predicate(ref p) if matches!(p, Predicate::Metadata(ref mp) if matches!(&**mp, MetadataPredicate::Modified(TimeMatcher::After(_)))))
    );

    // Absolute date - this should be exactly comparable
    let typed = parse_and_typecheck("created == 2024-01-01").unwrap();
    let expected_time = parse_time_value("2024-01-01").unwrap();
    let expected = Expr::Predicate(Predicate::meta(MetadataPredicate::Created(
        TimeMatcher::Equals(expected_time),
    )));
    assert_eq!(typed, expected);
}

#[test]
fn test_boolean_logic_preservation() {
    // AND
    let typed = parse_and_typecheck("name == foo AND size > 1000").unwrap();
    assert!(matches!(typed, Expr::And(_, _)));

    // OR
    let typed = parse_and_typecheck("name == foo OR ext == rs").unwrap();
    assert!(matches!(typed, Expr::Or(_, _)));

    // NOT
    let typed = parse_and_typecheck("NOT name == foo").unwrap();
    assert!(matches!(typed, Expr::Not(_)));
}

#[test]
fn test_complex_expression() {
    let typed =
        parse_and_typecheck("(name == test.rs OR ext in [js, ts]) AND NOT size > 1mb").unwrap();

    // Construct expected complex expression
    let lhs_left = Expr::Predicate(Predicate::name(NamePredicate::FileName(
        StringMatcher::Equals("test.rs".to_string()),
    )));
    let lhs_right_set: HashSet<String> = vec!["js", "ts"]
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    let lhs_right = Expr::Predicate(Predicate::name(NamePredicate::Extension(
        StringMatcher::In(lhs_right_set),
    )));
    let lhs = Expr::or(lhs_left, lhs_right);

    let rhs_inner = Expr::Predicate(Predicate::meta(MetadataPredicate::Filesize(
        NumberMatcher::In(Bound::Left(1048577..)),
    )));
    let rhs = Expr::negate(rhs_inner);

    let expected = Expr::and(lhs, rhs);
    assert_eq!(typed, expected);
}

#[test]
fn test_glob_pattern() {
    let typed = parse_and_typecheck("*.rs").unwrap();
    let expected_glob = globset::Glob::new("*.rs").unwrap();
    let expected = Expr::Predicate(Predicate::name(NamePredicate::GlobPattern(expected_glob)));
    assert_eq!(typed, expected);
}

#[test]
fn test_selector_aliases() {
    // Test that all selector aliases resolve to the same predicate

    // File identity aliases
    assert_eq!(
        parse_and_typecheck("filename == test.rs").unwrap(),
        parse_and_typecheck("name == test.rs").unwrap()
    );

    assert_eq!(
        parse_and_typecheck("stem == test").unwrap(),
        parse_and_typecheck("basename == test").unwrap()
    );

    assert_eq!(
        parse_and_typecheck("extension == rs").unwrap(),
        parse_and_typecheck("ext == rs").unwrap()
    );

    assert_eq!(
        parse_and_typecheck("parent contains src").unwrap(),
        parse_and_typecheck("dir contains src").unwrap()
    );

    assert_eq!(
        parse_and_typecheck("directory contains src").unwrap(),
        parse_and_typecheck("dir contains src").unwrap()
    );

    // File property aliases
    assert_eq!(
        parse_and_typecheck("filesize > 1mb").unwrap(),
        parse_and_typecheck("size > 1mb").unwrap()
    );

    assert_eq!(
        parse_and_typecheck("bytes > 1024").unwrap(),
        parse_and_typecheck("size > 1024").unwrap()
    );

    assert_eq!(
        parse_and_typecheck("filetype == file").unwrap(),
        parse_and_typecheck("type == file").unwrap()
    );

    // Time aliases - use absolute timestamps to avoid timing issues
    assert_eq!(
        parse_and_typecheck("mtime > 2024-01-01").unwrap(),
        parse_and_typecheck("modified > 2024-01-01").unwrap()
    );

    assert_eq!(
        parse_and_typecheck("ctime < 2024-12-31").unwrap(),
        parse_and_typecheck("created < 2024-12-31").unwrap()
    );

    assert_eq!(
        parse_and_typecheck("atime == 2024-06-15").unwrap(),
        parse_and_typecheck("accessed == 2024-06-15").unwrap()
    );

    // Content aliases
    assert_eq!(
        parse_and_typecheck("contents contains TODO").unwrap(),
        parse_and_typecheck("content contains TODO").unwrap()
    );

    assert_eq!(
        parse_and_typecheck("text ~= pattern").unwrap(),
        parse_and_typecheck("content ~= pattern").unwrap()
    );
}

#[test]
fn test_operator_aliases() {
    // Test string operator aliases
    let cases = vec![
        ("name = foo", "name == foo"),
        ("name eq foo", "name == foo"),
        ("path != bar", "path <> bar"),
        ("content matches pattern", "content ~= pattern"),
        ("content regex pattern", "content ~= pattern"),
        ("path has src", "path contains src"),
        ("path includes src", "path contains src"),
    ];

    for (alias, canonical) in cases {
        let result1 = RawParser::parse_raw_expr(alias).unwrap();
        let typed1 = Typechecker::typecheck(result1, alias).unwrap();

        let result2 = RawParser::parse_raw_expr(canonical).unwrap();
        let typed2 = Typechecker::typecheck(result2, canonical).unwrap();

        assert_eq!(typed1, typed2, "Failed for {} vs {}", alias, canonical);
    }

    // Test numeric operator aliases
    let num_cases = vec![
        ("size = 100", "size == 100"),
        ("size gt 100", "size > 100"),
        ("size gte 100", "size >= 100"),
        ("size lt 100", "size < 100"),
        ("size lte 100", "size <= 100"),
        ("size => 100", "size >= 100"),
        ("size =< 100", "size <= 100"),
    ];

    for (alias, canonical) in num_cases {
        let result1 = RawParser::parse_raw_expr(alias).unwrap();
        let typed1 = Typechecker::typecheck(result1, alias).unwrap();

        let result2 = RawParser::parse_raw_expr(canonical).unwrap();
        let typed2 = Typechecker::typecheck(result2, canonical).unwrap();

        assert_eq!(typed1, typed2, "Failed for {} vs {}", alias, canonical);
    }

    // Test temporal operator aliases
    let time_cases = vec![
        ("modified on 2024-01-01", "modified == 2024-01-01"),
        ("modified before 2024-01-01", "modified < 2024-01-01"),
        ("modified after 2024-01-01", "modified > 2024-01-01"),
    ];

    for (alias, canonical) in time_cases {
        let result1 = RawParser::parse_raw_expr(alias).unwrap();
        let typed1 = Typechecker::typecheck(result1, alias).unwrap();

        let result2 = RawParser::parse_raw_expr(canonical).unwrap();
        let typed2 = Typechecker::typecheck(result2, canonical).unwrap();

        assert_eq!(typed1, typed2, "Failed for {} vs {}", alias, canonical);
    }
}

#[test]
fn test_invalid_values() {
    // Bracketed value with == now treated as literal string (operator determines intent)
    let result = parse_and_typecheck("name == [foo, bar]");
    assert!(
        result.is_ok(),
        "With == operator, [foo, bar] is a literal string value"
    );

    // Non-numeric value for size
    let error = parse_and_typecheck("size > foo").unwrap_err();
    assert!(matches!(error, TypecheckError::InvalidValue { .. }));

    // Set value for content - content doesn't support 'in' operator
    let error = parse_and_typecheck("content in [foo, bar]").unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));
}

#[test]
fn test_content_operators() {
    // Contents supports limited operators
    let valid_ops = vec!["==", "~=", "contains"];
    for op in valid_ops {
        let expr = format!("content {} pattern", op);
        assert!(
            parse_and_typecheck(&expr).is_ok(),
            "Failed for operator: {}",
            op
        );
    }

    // Contents does not support 'in'
    let error = parse_and_typecheck("content in [foo, bar]").unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));

    // Contents does not support '!='
    let error = parse_and_typecheck("content != pattern").unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));
}

#[test]
fn test_type_safety_enforcement() {
    // String selector with numeric operator should fail
    let error = parse_and_typecheck("name > foo").unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));

    // Numeric selector with string operator should fail
    let error = parse_and_typecheck("size contains foo").unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));

    // Temporal selector with invalid operator should fail
    let error = parse_and_typecheck("modified contains 2024").unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));

    // Contents with 'in' operator should fail (special case)
    let error = parse_and_typecheck("content in [foo, bar]").unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));

    // Contents with '!=' operator should fail (special case)
    let error = parse_and_typecheck("content != pattern").unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));
}

#[test]
fn test_all_operator_aliases_work() {
    // Test cases that use various operator aliases
    let test_cases = vec![
        // String operators
        "name = foo",
        "name eq foo",
        "path <> bar",
        "content matches pattern",
        "content regex pattern",
        "path has src",
        "path includes src",
        // Numeric operators
        "size = 100",
        "size gt 100",
        "size gte 100",
        "size lt 100",
        "size lte 100",
        "size => 100",
        "size =< 100",
        // Temporal operators
        "modified on 2024-01-01",
        "modified before 2024-01-01",
        "modified after 2024-01-01",
    ];

    for expr in test_cases {
        let parse_result = RawParser::parse_raw_expr(expr);
        assert!(parse_result.is_ok(), "Failed to parse: {}", expr);
        let typecheck_result = Typechecker::typecheck(parse_result.unwrap(), expr);
        assert!(typecheck_result.is_ok(), "Failed to typecheck: {}", expr);
    }
}

#[test]
fn test_case_insensitive_operators() {
    // Test that operators work in any case
    let test_cases = vec![
        ("name CONTAINS foo", "name contains foo"),
        ("size GT 100", "size gt 100"),
        ("modified BEFORE 2024-01-01", "modified before 2024-01-01"),
        ("content MATCHES pattern", "content matches pattern"),
        ("ext IN [rs, js]", "ext in [rs, js]"),
        ("name EQ test", "name eq test"),
    ];

    for (upper_case, lower_case) in test_cases {
        let result1 = RawParser::parse_raw_expr(upper_case).unwrap();
        let typed1 = Typechecker::typecheck(result1, upper_case).unwrap();

        let result2 = RawParser::parse_raw_expr(lower_case).unwrap();
        let typed2 = Typechecker::typecheck(result2, lower_case).unwrap();

        assert_eq!(
            typed1, typed2,
            "Case sensitivity failed for: {} vs {}",
            upper_case, lower_case
        );
    }
}

#[test]
fn test_truly_unknown_operators() {
    // Test that genuinely unknown operators fail at typecheck with UnknownOperator error
    let test_cases = vec![
        ("name === foo", "==="),
        ("name ! foo", "!"),
        ("name <=> foo", "<=>"),
        ("name ~~ foo", "~~"),
        ("name >>> foo", ">>>"),
        ("name like foo", "like"),       // SQL-style LIKE not supported
        ("name between foo", "between"), // SQL BETWEEN not supported
    ];

    for (expr, op) in test_cases {
        // Should parse now, but fail at typecheck
        let error = parse_and_typecheck(expr).unwrap_err();
        assert!(
            matches!(error, TypecheckError::UnknownOperator { operator: ref o, .. } if o == op),
            "Expected UnknownOperator({}) for expression: {}",
            op,
            expr
        );
    }
}

// ==============================================================================
// Cross-layer edge case tests (formerly in stress_test_parser.rs)
// These tests verify parse→typecheck pipeline behavior
// ==============================================================================

#[test]
fn test_operator_edge_cases() {
    use detect::parser::test_utils::RawTestExpr;

    // Single = is now valid (alias for ==)
    let expr = "name = foo";
    let result = RawParser::parse_raw_expr(expr).unwrap();
    let expected = RawTestExpr::string_predicate("name", "=", "foo");
    assert_eq!(result.to_test_expr(), expected);
    // Verify it typechecks successfully as an alias for ==
    let typecheck_result = Typechecker::typecheck(result, expr);
    assert!(
        typecheck_result.is_ok(),
        "Single = should typecheck as valid alias for =="
    );

    // Single ! now parses but will fail at typecheck (needs to be != or NOT)
    let expr = "name ! foo";
    let result = RawParser::parse_raw_expr(expr).unwrap();
    let expected = RawTestExpr::string_predicate("name", "!", "foo");
    assert_eq!(result.to_test_expr(), expected);
    // Verify it fails at typecheck with UnknownOperator
    let typecheck_result = Typechecker::typecheck(result, expr);
    assert!(
        matches!(typecheck_result, Err(TypecheckError::UnknownOperator { operator: ref o, .. }) if o == "!"),
        "Single ! should fail typecheck with UnknownOperator"
    );

    // Single ~ is now valid (alias for ~=)
    let expr = "name ~ foo";
    let result = RawParser::parse_raw_expr(expr).unwrap();
    let expected = RawTestExpr::string_predicate("name", "~", "foo");
    assert_eq!(result.to_test_expr(), expected);
    // Verify it typechecks successfully as an alias for ~=
    let typecheck_result = Typechecker::typecheck(result, expr);
    assert!(
        typecheck_result.is_ok(),
        "Single ~ should typecheck as valid alias"
    );

    // Spaced operators will parse as separate tokens and fail
    let result = RawParser::parse_raw_expr("name < = foo");
    assert!(
        result.is_err(),
        "Spaced <= should still fail due to grammar structure"
    );

    // Non-existent operators now parse but will fail at typecheck
    let expr = "name === foo";
    let result = RawParser::parse_raw_expr(expr).unwrap();
    let expected = RawTestExpr::string_predicate("name", "===", "foo");
    assert_eq!(result.to_test_expr(), expected);
    // Verify it fails at typecheck with UnknownOperator
    let typecheck_result = Typechecker::typecheck(result, expr);
    assert!(
        matches!(typecheck_result, Err(TypecheckError::UnknownOperator { operator: ref o, .. }) if o == "==="),
        "Triple equals should fail typecheck with UnknownOperator"
    );

    // <> is now valid (alias for !=)
    let expr = "name <> foo";
    let result = RawParser::parse_raw_expr(expr).unwrap();
    let expected = RawTestExpr::string_predicate("name", "<>", "foo");
    assert_eq!(result.to_test_expr(), expected);
    // Verify it typechecks successfully as an alias for !=
    let typecheck_result = Typechecker::typecheck(result, expr);
    assert!(
        typecheck_result.is_ok(),
        "SQL-style <> should typecheck as valid alias for !="
    );
}

#[test]
fn test_unknown_operators_parse_but_fail_typecheck() {
    use detect::parser::test_utils::RawTestExpr;

    // Test that various unknown operators parse successfully but fail at typecheck
    let test_cases = vec![
        ("name ! foo", "!", "name", "foo"),
        ("size === 100", "===", "size", "100"),
        ("path <=> test", "<=>", "path", "test"),
        ("content ~~ pattern", "~~", "content", "pattern"),
        ("depth >>> 3", ">>>", "depth", "3"),
        ("type !! file", "!!", "type", "file"),
        ("ext !== rs", "!==", "ext", "rs"),
    ];

    for (expr, expected_op, expected_selector, expected_value) in test_cases {
        // Parse should succeed
        let result = RawParser::parse_raw_expr(expr).unwrap_or_else(|e| {
            panic!("Failed to parse '{}': {:?}", expr, e);
        });

        // Verify parsed structure
        let expected =
            RawTestExpr::string_predicate(expected_selector, expected_op, expected_value);
        assert_eq!(
            result.to_test_expr(),
            expected,
            "Parsed structure mismatch for '{}'",
            expr
        );

        // Typecheck should fail with UnknownOperator
        let typecheck_result = Typechecker::typecheck(result, expr);
        assert!(
            matches!(typecheck_result, Err(TypecheckError::UnknownOperator { operator: ref o, .. }) if o == expected_op),
            "Expected UnknownOperator({}) for '{}', got {:?}",
            expected_op,
            expr,
            typecheck_result
        );
    }
}

#[test]
fn test_very_large_numeric_values() {
    use detect::parser::typechecker::Typechecker;

    // Test numbers at the edge of u64
    let cases = vec![
        "size == 18446744073709551615", // u64::MAX
        "size > 9999999999999999999",
        "filesize < 1000000000000000000",
    ];

    for expr in cases {
        let result = RawParser::parse_raw_expr(expr);
        assert!(result.is_ok(), "Failed to parse: {}", expr);
    }

    // Test numbers beyond u64 range (should parse but fail typecheck)
    let overflow = "size > 99999999999999999999999999999";
    let parse_result = RawParser::parse_raw_expr(overflow);
    assert!(parse_result.is_ok());

    // Should fail during typecheck
    let typecheck_result = Typechecker::typecheck(parse_result.unwrap(), overflow);
    assert!(typecheck_result.is_err());
}

#[test]
fn test_special_regex_characters() {
    use detect::parser::typechecker::Typechecker;

    // Test regex patterns with special characters that require proper handling
    // These patterns test the hybrid regex engine's ability to handle both
    // Rust regex and PCRE2 patterns correctly
    let cases = vec![
        // Escaped literal characters
        (r#"name ~= "test\\.rs""#, "Escaped dot in regex"),
        (r#"path ~= "src/main\\.rs""#, "Path with escaped dot"),
        // Word boundaries (PCRE2 feature, should fallback)
        (r#"content ~= "\\bword\\b""#, "Word boundary anchors"),
        // Character classes and quantifiers
        (
            r#"path ~= "[a-z]+\\.rs$""#,
            "Character class with quantifier",
        ),
        (
            r#"name ~= "^test_[0-9]{3}""#,
            "Anchored pattern with repetition",
        ),
        // Case insensitive flag
        (
            r#"text ~= "(?i)case.*insensitive""#,
            "Case insensitive flag",
        ),
    ];

    for (expr, description) in cases {
        let parse_result = RawParser::parse_raw_expr(expr);
        assert!(
            parse_result.is_ok(),
            "Failed to parse {}: {}",
            description,
            expr
        );

        let typecheck_result = Typechecker::typecheck(parse_result.unwrap(), expr);
        assert!(
            typecheck_result.is_ok(),
            "Failed to typecheck {}: {}",
            description,
            expr
        );
    }
}
