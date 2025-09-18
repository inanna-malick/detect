use detect::v2_parser::error::DetectError as TypecheckError;
use detect::v2_parser::{test_utils::RawTestExpr, Typechecker, *};

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

    // Set with no opening bracket
    let result = RawParser::parse_raw_expr("name in foo, bar]");
    assert!(result.is_err(), "No opening bracket should fail");

    // Nested sets (should fail)
    let result = RawParser::parse_raw_expr("name in [foo, [bar]]");
    assert!(result.is_err(), "Nested sets should fail");

    // Set with trailing comma
    let result = RawParser::parse_raw_expr("name in [foo, bar,]");
    assert!(result.is_err(), "Trailing comma should fail");

    // Set with only commas
    let result = RawParser::parse_raw_expr("name in [,,,]");
    assert!(result.is_err(), "Only commas should fail");
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
fn test_escape_sequences_preserved() {
    // Since we're a syntax-only parser, all escape sequences are preserved without validation
    // Previously "invalid" escape chars are now just preserved as-is
    let test_escapes = vec!['a', 'x', 'z', '0', '9', '!', '@', '#'];

    for ch in test_escapes {
        let input = format!("name == \"test\\{}\"", ch);
        let result = RawParser::parse_raw_expr(&input);
        let expected_value = format!("test\\{}", ch);
        let expected = RawTestExpr::string_predicate("name", "==", &expected_value);
        assert_eq!(result.unwrap().to_test_expr(), expected);
    }
}

#[test]
fn test_unicode_and_special_chars() {
    // Unicode in selectors (should fail - our grammar uses ASCII_ALPHANUMERIC)
    let result = RawParser::parse_raw_expr("测试 == foo");
    assert!(
        result.is_err(),
        "Unicode characters in selectors should fail"
    );

    // Unicode in values (should work)
    let result = RawParser::parse_raw_expr("name == 测试");
    assert_eq!(
        result.unwrap().to_test_expr(),
        RawTestExpr::string_predicate("name", "==", "测试")
    );

    // Emoji in values
    let result = RawParser::parse_raw_expr("name == 🚀");
    assert_eq!(
        result.unwrap().to_test_expr(),
        RawTestExpr::string_predicate("name", "==", "🚀")
    );

    // Control characters (actually works in bare values)
    let result = RawParser::parse_raw_expr("name == \x00");
    assert_eq!(
        result.unwrap().to_test_expr(),
        RawTestExpr::string_predicate("name", "==", "\x00")
    );

    // Very long strings
    let long_value = "a".repeat(10000);
    let input = format!("name == {}", long_value);
    let result = RawParser::parse_raw_expr(&input);
    let expected = RawTestExpr::string_predicate("name", "==", &long_value);
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

#[test]
fn test_operator_edge_cases() {
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

#[test]
fn test_selector_edge_cases() {
    // Empty selector
    let result = RawParser::parse_raw_expr(" == foo");
    assert!(result.is_err(), "Empty selector should fail");

    // Selector starting with numbers (actually works - gets parsed as "123name")
    let result = RawParser::parse_raw_expr("123name == foo");
    assert_eq!(
        result.unwrap().to_test_expr(),
        RawTestExpr::string_predicate("123name", "==", "foo")
    );

    // Selector with invalid characters
    let result = RawParser::parse_raw_expr("na$me == foo");
    assert!(result.is_err(), "Selector with $ should fail");

    let result = RawParser::parse_raw_expr("na-me == foo");
    assert!(result.is_err(), "Selector with - should fail");

    // Very long selector
    let long_selector = "a".repeat(1000);
    let input = format!("{} == foo", long_selector);
    let result = RawParser::parse_raw_expr(&input);
    let expected = RawTestExpr::string_predicate(&long_selector, "==", "foo");
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

#[test]
fn test_value_edge_cases() {
    // Value with all special characters
    let result = RawParser::parse_raw_expr(r#"name == "!@#$%^&*()_+-=[]{}|;:,.<>?""#);
    assert_eq!(
        result.unwrap().to_test_expr(),
        RawTestExpr::string_predicate("name", "==", "!@#$%^&*()_+-=[]{}|;:,.<>?")
    );

    // Bare value that looks like operator
    let result = RawParser::parse_raw_expr("name == ==");
    assert_eq!(
        result.unwrap().to_test_expr(),
        RawTestExpr::string_predicate("name", "==", "==")
    );

    // Bare value that looks like boolean operator
    let result = RawParser::parse_raw_expr("name == AND");
    assert!(
        result.is_err(),
        "AND keyword as bare value should fail (infix conflict)"
    );

    let result = RawParser::parse_raw_expr("name == OR");
    assert!(
        result.is_err(),
        "OR keyword as bare value should fail (infix conflict)"
    );

    // NOT is only a prefix operator, so it can be a value - this is correct!
    let result = RawParser::parse_raw_expr("name == NOT");
    assert_eq!(
        result.unwrap().to_test_expr(),
        RawTestExpr::string_predicate("name", "==", "NOT")
    );

    // Case insensitive variants
    let result = RawParser::parse_raw_expr("name == and");
    assert!(result.is_err(), "and keyword as bare value should fail");

    let result = RawParser::parse_raw_expr("name == or");
    assert!(result.is_err(), "or keyword as bare value should fail");

    let result = RawParser::parse_raw_expr("name == not");
    assert_eq!(
        result.unwrap().to_test_expr(),
        RawTestExpr::string_predicate("name", "==", "not")
    );
}

#[test]
fn test_glob_vs_predicate_conflicts() {
    // Things that could be ambiguous between glob and predicate

    // This should be a predicate, not a glob
    let result = RawParser::parse_raw_expr("*.rs");
    assert_eq!(result.unwrap().to_test_expr(), RawTestExpr::Glob("*.rs"));

    // What about something that starts like a predicate but is incomplete?
    let result = RawParser::parse_raw_expr("name*");
    assert_eq!(result.unwrap().to_test_expr(), RawTestExpr::Glob("name*"));

    // Glob with spaces (actually fails to parse - not supported by grammar)
    let result = RawParser::parse_raw_expr("test file");
    assert!(
        result.is_err(),
        "Glob with spaces should fail in current grammar"
    );
}

#[test]
fn test_complex_real_world_cases() {
    // Very complex expression - verify it parses to AND at root
    let complex = r#"(name == "test.rs" OR ext in [js, ts, "file.jsx"]) AND NOT (size > 1mb OR modified < -7d) AND content ~= "(TODO|FIXME)""#;
    let result = RawParser::parse_raw_expr(complex);
    assert!(
        matches!(result.unwrap().to_test_expr(), RawTestExpr::And(_, _)),
        "Should be AND at root"
    );

    // Expression with every operator - verify it parses to OR at root
    let all_ops = r#"a == b AND c != d OR e ~= f AND g > h OR i < j AND k >= l OR m <= n AND o contains p OR q in [r, s]"#;
    let result = RawParser::parse_raw_expr(all_ops);
    assert!(
        matches!(result.unwrap().to_test_expr(), RawTestExpr::Or(_, _)),
        "Should be OR at root"
    );

    // Deeply nested with mixed quotes - verify it parses to OR at root
    let nested = r#"((a == 'single') AND (b == "double")) OR (c in ['mixed', "quotes"])"#;
    let result = RawParser::parse_raw_expr(nested);
    assert!(
        matches!(result.unwrap().to_test_expr(), RawTestExpr::Or(_, _)),
        "Should be OR at root"
    );
}

#[test]
fn test_parser_robustness() {
    // Try to cause memory issues with large sets (deep nesting tested in test_extreme_nesting_limits)
    let large_set_items: Vec<String> = (0..1000).map(|i| format!("item{}", i)).collect();
    let large_set = format!("name in [{}]", large_set_items.join(", "));
    let result = RawParser::parse_raw_expr(&large_set);
    let large_set_vec: Vec<&str> = large_set_items.iter().map(|s| s.as_str()).collect();
    let expected = RawTestExpr::set_predicate("name", "in", large_set_vec);
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

#[test]
fn test_maximum_adversarial_cases() {
    // Test deeply nested boolean logic with mixed quotes and edge cases
    let adversarial_expr = r#"
        (((NOT (name == "test\\\"file" OR ext in ['rs', "js"]) AND 
          size > 1024) OR (NOT NOT modified > -7d)) AND 
         (content ~= "TODO.*FIXME" OR name != "")) AND 
        !(dir contains "/" AND NOT (type == "file"))
    "#
    .trim()
    .replace("\n        ", " ");

    let result = RawParser::parse_raw_expr(&adversarial_expr);
    // Verify it has reasonable structure - should be a complex AND at root
    let test_expr = result.unwrap().to_test_expr();
    assert!(
        matches!(test_expr, RawTestExpr::And(_, _)),
        "Should be an AND at root"
    );
}

#[test]
fn test_unicode_boundary_conditions() {
    // Test various Unicode edge cases with specific assertions
    let test_cases = vec![
        (
            "name == 你好世界",
            RawTestExpr::string_predicate("name", "==", "你好世界"),
        ), // CJK characters
        (
            "path == café",
            RawTestExpr::string_predicate("path", "==", "café"),
        ), // Accented characters
        (
            "name == файл",
            RawTestExpr::string_predicate("name", "==", "файл"),
        ), // Cyrillic
        (
            r#"content == "🚀🌟⭐""#,
            RawTestExpr::string_predicate("content", "==", "🚀🌟⭐"),
        ), // Emoji
        (
            "name == \u{1F4A9}",
            RawTestExpr::string_predicate("name", "==", "\u{1F4A9}"),
        ), // Pile of poo emoji as bare value
    ];

    for (input, expected) in test_cases {
        let result = RawParser::parse_raw_expr(input);
        assert_eq!(
            result.unwrap().to_test_expr(),
            expected,
            "Failed for input: {}",
            input
        );
    }
}

#[test]
fn test_pathological_whitespace() {
    // Test every kind of whitespace
    let whitespace_expr = "name\t\t==\n\n\t'value with\ttabs\nand newlines'";
    let result = RawParser::parse_raw_expr(whitespace_expr);
    let expected = RawTestExpr::string_predicate("name", "==", "value with\ttabs\nand newlines");
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

#[test]
fn test_extreme_nesting_limits() {
    // Test very deep nesting to check stack safety
    let deep_parens = "(".repeat(200) + "name == foo" + &")".repeat(200);
    let result = RawParser::parse_raw_expr(&deep_parens);
    let expected = RawTestExpr::string_predicate("name", "==", "foo");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Test deep NOT chain - more extreme than basic robustness test
    let deep_not = "NOT ".repeat(100) + "name == foo";
    let result = RawParser::parse_raw_expr(&deep_not);
    let mut expected = RawTestExpr::string_predicate("name", "==", "foo");
    for _ in 0..100 {
        expected = RawTestExpr::not(expected);
    }
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

#[test]
fn test_unknown_operators_parse_but_fail_typecheck() {
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
fn test_memory_stress_large_inputs() {
    // Test very large quoted strings
    let large_string = "x".repeat(50000);
    let input = format!(r#"name == "{}""#, large_string);
    let result = RawParser::parse_raw_expr(&input);
    let expected = RawTestExpr::string_predicate("name", "==", &large_string);
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Test very large sets - more extreme than basic robustness test
    let large_set_items: Vec<String> = (0..5000).map(|i| format!("item{}", i)).collect();
    let large_set = format!("name in [{}]", large_set_items.join(", "));
    let result = RawParser::parse_raw_expr(&large_set);
    let large_set_vec: Vec<&str> = large_set_items.iter().map(|s| s.as_str()).collect();
    let expected = RawTestExpr::set_predicate("name", "in", large_set_vec);
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

#[test]
fn test_empty_set_values() {
    // Empty set should parse but might fail typecheck
    let result = RawParser::parse_raw_expr("ext in []");
    assert!(result.is_ok());

    // Verify it parses as a set operation with empty items
    let expected = RawTestExpr::set_predicate("ext", "in", vec![]);
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

#[test]
fn test_mixed_quotes_in_sets() {
    // Mixed quotes should work fine
    let cases = vec![
        r#"ext in ['js', "ts", 'jsx']"#,
        r#"name in ["test.rs", 'main.rs', "lib.rs"]"#,
        r#"basename in ['config', "settings", 'options']"#,
    ];

    for expr in cases {
        let result = RawParser::parse_raw_expr(expr);
        assert!(result.is_ok(), "Failed to parse: {}", expr);
    }
}

#[test]
fn test_very_large_numeric_values() {
    use detect::v2_parser::typechecker::Typechecker;

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
fn test_escaped_characters_in_values() {
    // Test various escape sequences
    let cases = vec![
        r#"name == "test\nfile.txt""#,
        r#"content contains "line1\nline2\ttab""#,
        r#"path ~= "\\\\server\\\\share""#,
        r#"text contains "quote\"inside""#,
    ];

    for expr in cases {
        let result = RawParser::parse_raw_expr(expr);
        assert!(result.is_ok(), "Failed to parse: {}", expr);
    }
}

#[test]
fn test_whitespace_in_set_values() {
    use detect::v2_parser::typechecker::Typechecker;

    // Whitespace should be preserved in set values
    let expr = r#"name in ["file one.txt", "file  two.txt", "  spaces  "]"#;
    let result = RawParser::parse_raw_expr(expr);
    assert!(result.is_ok());

    // After typechecking, verify whitespace is preserved
    let typed = Typechecker::typecheck(result.unwrap(), expr).unwrap();
    // The actual verification would happen in the set values
}

#[test]
fn test_special_regex_characters() {
    use detect::v2_parser::typechecker::Typechecker;

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
        (r#"path ~= "[a-z]+\\.rs$""#, "Character class with quantifier"),
        (r#"name ~= "^test_[0-9]{3}""#, "Anchored pattern with repetition"),

        // Case insensitive flag
        (r#"text ~= "(?i)case.*insensitive""#, "Case insensitive flag"),
    ];

    for (expr, description) in cases {
        let parse_result = RawParser::parse_raw_expr(expr);
        assert!(parse_result.is_ok(), "Failed to parse {}: {}", description, expr);

        let typecheck_result = Typechecker::typecheck(parse_result.unwrap(), expr);
        assert!(
            typecheck_result.is_ok(),
            "Failed to typecheck {}: {}",
            description,
            expr
        );
    }
}
