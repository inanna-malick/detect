use detect::expr::Expr;
use detect::predicate::{
    parse_time_value, Bound, MetadataPredicate, NamePredicate, NumberMatcher, Predicate,
    StreamingCompiledContentPredicate, StringMatcher, TimeMatcher,
};
use detect::v2_parser::{RawParser, TypecheckError, Typechecker};
use globset;
use std::collections::HashSet;

#[test]
fn test_selector_recognition() {
    // Test path.full selector
    let result = RawParser::parse_raw_expr("path == foo").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
    let expected = Expr::Predicate(Predicate::name(NamePredicate::FullPath(
        StringMatcher::Equals("foo".to_string()),
    )));
    assert_eq!(typed, expected);

    // Test filename selector
    let result = RawParser::parse_raw_expr("filename == test.rs").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
    let expected = Expr::Predicate(Predicate::name(NamePredicate::FileName(
        StringMatcher::Equals("test.rs".to_string()),
    )));
    assert_eq!(typed, expected);

    // Test extension selector
    let result = RawParser::parse_raw_expr("ext == rs").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
    let expected = Expr::Predicate(Predicate::name(NamePredicate::Extension(
        StringMatcher::Equals("rs".to_string()),
    )));
    assert_eq!(typed, expected);
}

#[test]
fn test_unknown_selector() {
    let result = RawParser::parse_raw_expr("unknown_selector == foo").unwrap();
    let error = Typechecker::typecheck(result).unwrap_err();
    assert!(matches!(error, TypecheckError::UnknownSelector(s) if s == "unknown_selector"));
}

#[test]
fn test_operator_validation() {
    // Valid string operator
    let result = RawParser::parse_raw_expr("name == foo").unwrap();
    assert!(Typechecker::typecheck(result).is_ok());

    // Invalid operator for string selector (numeric operator on string type)
    let result = RawParser::parse_raw_expr("name > foo").unwrap();
    let error = Typechecker::typecheck(result).unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));

    // Valid numeric operator
    let result = RawParser::parse_raw_expr("size > 1000").unwrap();
    assert!(Typechecker::typecheck(result).is_ok());

    // Invalid operator for numeric selector (string operator on numeric type)
    let result = RawParser::parse_raw_expr("size contains foo").unwrap();
    let error = Typechecker::typecheck(result).unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));
}

#[test]
fn test_string_value_parsing() {
    // Equals - note: "name" maps to FileName (complete filename)
    let result = RawParser::parse_raw_expr("name == test").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
    let expected = Expr::Predicate(Predicate::name(NamePredicate::FileName(
        StringMatcher::Equals("test".to_string()),
    )));
    assert_eq!(typed, expected);

    // Not equals
    let result = RawParser::parse_raw_expr("filename != test.rs").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
    let expected = Expr::Predicate(Predicate::name(NamePredicate::FileName(
        StringMatcher::NotEquals("test.rs".to_string()),
    )));
    assert_eq!(typed, expected);

    // Contains
    let result = RawParser::parse_raw_expr("path contains src").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
    let expected = Expr::Predicate(Predicate::name(NamePredicate::FullPath(
        StringMatcher::Contains("src".to_string()),
    )));
    assert_eq!(typed, expected);
}

#[test]
fn test_set_value_parsing() {
    let result = RawParser::parse_raw_expr("ext in [rs, js, ts]").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
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
    let result = RawParser::parse_raw_expr("content ~= TODO.*").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
    let expected_content = StreamingCompiledContentPredicate::new("TODO.*".to_string()).unwrap();
    let expected = Expr::Predicate(Predicate::contents(expected_content));
    assert_eq!(typed, expected);
}

#[test]
fn test_size_value_parsing() {
    // Plain number (size > 1000 becomes range 1001..)
    let result = RawParser::parse_raw_expr("size > 1000").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
    let expected = Expr::Predicate(Predicate::meta(MetadataPredicate::Filesize(
        NumberMatcher::In(Bound::Left(1001..)),
    )));
    assert_eq!(typed, expected);

    // Size with unit (1mb = 1048576 bytes, so > 1mb becomes range 1048577..)
    let result = RawParser::parse_raw_expr("size > 1mb").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
    let expected = Expr::Predicate(Predicate::meta(MetadataPredicate::Filesize(
        NumberMatcher::In(Bound::Left(1048577..)),
    )));
    assert_eq!(typed, expected);
}

#[test]
fn test_temporal_value_parsing() {
    // Relative time - just verify it's the right structure, times will differ slightly
    let result = RawParser::parse_raw_expr("modified > -7d").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
    assert!(
        matches!(typed, Expr::Predicate(ref p) if matches!(p, Predicate::Metadata(ref mp) if matches!(&**mp, MetadataPredicate::Modified(TimeMatcher::After(_)))))
    );

    // Absolute date - this should be exactly comparable
    let result = RawParser::parse_raw_expr("created == 2024-01-01").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
    let expected_time = parse_time_value("2024-01-01").unwrap();
    let expected = Expr::Predicate(Predicate::meta(MetadataPredicate::Created(
        TimeMatcher::Equals(expected_time),
    )));
    assert_eq!(typed, expected);
}

#[test]
fn test_boolean_logic_preservation() {
    // AND
    let result = RawParser::parse_raw_expr("name == foo AND size > 1000").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
    assert!(matches!(typed, Expr::And(_, _)));

    // OR
    let result = RawParser::parse_raw_expr("name == foo OR ext == rs").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
    assert!(matches!(typed, Expr::Or(_, _)));

    // NOT
    let result = RawParser::parse_raw_expr("NOT name == foo").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
    assert!(matches!(typed, Expr::Not(_)));
}

#[test]
fn test_complex_expression() {
    let result =
        RawParser::parse_raw_expr("(filename == test.rs OR ext in [js, ts]) AND NOT size > 1mb")
            .unwrap();
    let typed = Typechecker::typecheck(result).unwrap();

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
    let result = RawParser::parse_raw_expr("*.rs").unwrap();
    let typed = Typechecker::typecheck(result).unwrap();
    let expected_glob = globset::Glob::new("*.rs").unwrap();
    let expected = Expr::Predicate(Predicate::name(NamePredicate::GlobPattern(expected_glob)));
    assert_eq!(typed, expected);
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
        let typed1 = Typechecker::typecheck(result1).unwrap();

        let result2 = RawParser::parse_raw_expr(canonical).unwrap();
        let typed2 = Typechecker::typecheck(result2).unwrap();

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
        let typed1 = Typechecker::typecheck(result1).unwrap();

        let result2 = RawParser::parse_raw_expr(canonical).unwrap();
        let typed2 = Typechecker::typecheck(result2).unwrap();

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
        let typed1 = Typechecker::typecheck(result1).unwrap();

        let result2 = RawParser::parse_raw_expr(canonical).unwrap();
        let typed2 = Typechecker::typecheck(result2).unwrap();

        assert_eq!(typed1, typed2, "Failed for {} vs {}", alias, canonical);
    }
}

#[test]
fn test_selector_aliases() {
    // Test that all selector aliases resolve correctly
    let cases = vec![
        // Path aliases
        ("path.full == x", "full == x", "path == x"),
        ("path.filename == x", "file == x", "filename == x"),
        ("path.stem == x", "base == x", "basename == x"),
        ("path.extension == x", "suffix == x", "ext == x"),
        ("path.parent == x", "dir == x", "directory == x"),
        // Content aliases
        (
            "contents contains x",
            "content contains x",
            "text contains x",
        ),
        // Type aliases
        ("type == file", "filetype == file", "kind == file"),
        // Size aliases
        ("size > 100", "filesize > 100", "bytes > 100"),
        // Depth aliases
        ("depth < 3", "level < 3", "depth < 3"),
        // Temporal aliases (skip relative time tests - they'll differ by microseconds)
        (
            "created == 2024-01-01",
            "birth == 2024-01-01",
            "birthtime == 2024-01-01",
        ),
        (
            "accessed < 2024-01-01",
            "atime < 2024-01-01",
            "access < 2024-01-01",
        ),
    ];

    for (a, b, c) in cases {
        let result_a = RawParser::parse_raw_expr(a).unwrap();
        let typed_a = Typechecker::typecheck(result_a).unwrap();

        let result_b = RawParser::parse_raw_expr(b).unwrap();
        let typed_b = Typechecker::typecheck(result_b).unwrap();

        let result_c = RawParser::parse_raw_expr(c).unwrap();
        let typed_c = Typechecker::typecheck(result_c).unwrap();

        assert_eq!(typed_a, typed_b, "Failed for {} vs {}", a, b);
        assert_eq!(typed_b, typed_c, "Failed for {} vs {}", b, c);
    }
}

#[test]
fn test_invalid_values() {
    // Set value for non-in operator
    let result = RawParser::parse_raw_expr("name == [foo, bar]").unwrap();
    let error = Typechecker::typecheck(result).unwrap_err();
    // With the new typechecker, using a set with == returns InvalidValue
    assert!(matches!(error, TypecheckError::InvalidValue { .. }));

    // Non-numeric value for size
    let result = RawParser::parse_raw_expr("size > foo").unwrap();
    let error = Typechecker::typecheck(result).unwrap_err();
    assert!(matches!(error, TypecheckError::InvalidValue { .. }));

    // Set value for contents - contents doesn't support 'in' operator
    let result = RawParser::parse_raw_expr("contents in [foo, bar]").unwrap();
    let error = Typechecker::typecheck(result).unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));
}

#[test]
fn test_contents_operators() {
    // Contents supports limited operators
    let valid_ops = vec!["==", "~=", "contains"];
    for op in valid_ops {
        let expr = format!("contents {} pattern", op);
        let result = RawParser::parse_raw_expr(&expr).unwrap();
        assert!(
            Typechecker::typecheck(result).is_ok(),
            "Failed for operator: {}",
            op
        );
    }

    // Contents does not support 'in'
    let result = RawParser::parse_raw_expr("contents in [foo, bar]").unwrap();
    let error = Typechecker::typecheck(result).unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));

    // Contents does not support '!='
    let result = RawParser::parse_raw_expr("contents != pattern").unwrap();
    let error = Typechecker::typecheck(result).unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));
}

#[test]
fn test_type_safety_enforcement() {
    // String selector with numeric operator should fail
    let result = RawParser::parse_raw_expr("name > foo").unwrap();
    let error = Typechecker::typecheck(result).unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));

    // Numeric selector with string operator should fail
    let result = RawParser::parse_raw_expr("size contains foo").unwrap();
    let error = Typechecker::typecheck(result).unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));

    // Temporal selector with invalid operator should fail
    let result = RawParser::parse_raw_expr("modified contains 2024").unwrap();
    let error = Typechecker::typecheck(result).unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));

    // Contents with 'in' operator should fail (special case)
    let result = RawParser::parse_raw_expr("contents in [foo, bar]").unwrap();
    let error = Typechecker::typecheck(result).unwrap_err();
    assert!(matches!(error, TypecheckError::IncompatibleOperator { .. }));

    // Contents with '!=' operator should fail (special case)
    let result = RawParser::parse_raw_expr("contents != pattern").unwrap();
    let error = Typechecker::typecheck(result).unwrap_err();
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
        let typecheck_result = Typechecker::typecheck(parse_result.unwrap());
        assert!(typecheck_result.is_ok(), "Failed to typecheck: {}", expr);
    }
}

#[test]
fn test_all_selector_aliases_work() {
    // Test cases that use various selector aliases
    let test_cases = vec![
        // Path aliases
        "path.full == test",
        "full == test",
        "file == test.rs",
        "base == test",
        "suffix == rs",
        "dir == src",
        "directory == src",
        // Content aliases
        "text contains TODO",
        // Type aliases
        "kind == file",
        "filetype == file",
        // Size aliases
        "bytes > 100",
        // Depth aliases
        "level < 3",
        // Temporal aliases
        "mod > 2024-01-01",
        "birth == 2024-01-01",
        "birthtime == 2024-01-01",
        "access < 2024-01-01",
        "atime < 2024-01-01",
    ];

    for expr in test_cases {
        let parse_result = RawParser::parse_raw_expr(expr);
        assert!(parse_result.is_ok(), "Failed to parse: {}", expr);
        let typecheck_result = Typechecker::typecheck(parse_result.unwrap());
        assert!(typecheck_result.is_ok(), "Failed to typecheck: {}", expr);
    }
}

#[test]
fn test_complex_expressions_with_aliases() {
    let test_cases = vec![
        "(file eq test.rs OR suffix in [js, ts]) AND NOT bytes gt 1mb",
        "dir == src AND (content matches TODO OR content regex FIXME)",
        "kind == file AND mod after 2024-01-01 AND bytes lte 10mb",
    ];

    for expr in test_cases {
        let parse_result = RawParser::parse_raw_expr(expr);
        assert!(parse_result.is_ok(), "Failed to parse: {}", expr);
        let typecheck_result = Typechecker::typecheck(parse_result.unwrap());
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
        let typed1 = Typechecker::typecheck(result1).unwrap();

        let result2 = RawParser::parse_raw_expr(lower_case).unwrap();
        let typed2 = Typechecker::typecheck(result2).unwrap();

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
        let result = RawParser::parse_raw_expr(expr).unwrap(); // Should parse now
        let error = Typechecker::typecheck(result).unwrap_err();
        assert!(
            matches!(error, TypecheckError::UnknownOperator(ref o) if o == op),
            "Expected UnknownOperator({}) for expression: {}",
            op,
            expr
        );
    }
}
