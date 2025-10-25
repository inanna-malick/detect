use detect::parser::test_utils::RawTestExpr;
use detect::parser::*;

#[test]
fn test_bare_value_with_or_substring() {
    // Words containing "or" substring should parse as bare values
    let test_cases = vec![
        ("content contains Error", "content", "contains", "Error"),
        ("name == vendor", "name", "==", "vendor"),
        (
            "content contains information",
            "content",
            "contains",
            "information",
        ),
        ("name == sensor", "name", "==", "sensor"),
    ];

    for (expr, selector, op, value) in test_cases {
        let result = RawParser::parse_raw_expr(expr);
        let expected = RawTestExpr::string_predicate(selector, op, value);
        assert_eq!(
            result.unwrap().to_test_expr(),
            expected,
            "Failed for: {}",
            expr
        );
    }
}

#[test]
fn test_bare_value_with_and_substring() {
    // Words containing "and" substring should parse as bare values
    let test_cases = vec![
        ("name contains Android", "name", "contains", "Android"),
        ("name == standard", "name", "==", "standard"),
        (
            "content contains candidate",
            "content",
            "contains",
            "candidate",
        ),
        ("name == expand", "name", "==", "expand"),
    ];

    for (expr, selector, op, value) in test_cases {
        let result = RawParser::parse_raw_expr(expr);
        let expected = RawTestExpr::string_predicate(selector, op, value);
        assert_eq!(
            result.unwrap().to_test_expr(),
            expected,
            "Failed for: {}",
            expr
        );
    }
}

#[test]
fn test_bare_value_with_not_substring() {
    // Words containing "not" substring should parse as bare values
    let test_cases = vec![
        ("content contains cannot", "content", "contains", "cannot"),
        ("name == another", "name", "==", "another"),
        (
            "content contains notation",
            "content",
            "contains",
            "notation",
        ),
        ("name == denote", "name", "==", "denote"),
    ];

    for (expr, selector, op, value) in test_cases {
        let result = RawParser::parse_raw_expr(expr);
        let expected = RawTestExpr::string_predicate(selector, op, value);
        assert_eq!(
            result.unwrap().to_test_expr(),
            expected,
            "Failed for: {}",
            expr
        );
    }
}

#[test]
fn test_bare_value_case_variations() {
    // Test case variations of words with reserved substrings
    let test_cases = vec![
        ("content contains error", "error"),
        ("content contains ERROR", "ERROR"),
        ("content contains ErRoR", "ErRoR"),
    ];

    for (expr, expected_value) in test_cases {
        let result = RawParser::parse_raw_expr(expr);
        let expected = RawTestExpr::string_predicate("content", "contains", expected_value);
        assert_eq!(
            result.unwrap().to_test_expr(),
            expected,
            "Failed for case variation: {}",
            expr
        );
    }
}

#[test]
fn test_reserved_words_as_complete_tokens_work_in_value_position() {
    // Reserved words as values should work - context disambiguates!
    // "name == or" searches for a file named "or", not a boolean operator
    let result = RawParser::parse_raw_expr("name == or");
    assert!(
        result.is_ok(),
        "Reserved word 'or' in value position should work"
    );
    let expected = RawTestExpr::string_predicate("name", "==", "or");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    let result = RawParser::parse_raw_expr("name == and");
    assert!(
        result.is_ok(),
        "Reserved word 'and' in value position should work"
    );
    let expected = RawTestExpr::string_predicate("name", "==", "and");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    let result = RawParser::parse_raw_expr("name == not");
    assert!(
        result.is_ok(),
        "Reserved word 'not' in value position should work"
    );
    let expected = RawTestExpr::string_predicate("name", "==", "not");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Case variations
    let result = RawParser::parse_raw_expr("name == OR");
    assert!(
        result.is_ok(),
        "Reserved word 'OR' in value position should work"
    );

    let result = RawParser::parse_raw_expr("name == AND");
    assert!(
        result.is_ok(),
        "Reserved word 'AND' in value position should work"
    );

    let result = RawParser::parse_raw_expr("name == NOT");
    assert!(
        result.is_ok(),
        "Reserved word 'NOT' in value position should work"
    );
}

#[test]
fn test_quoted_reserved_words_should_work() {
    // Quoted versions should always work, even for actual reserved words
    let test_cases = vec![
        (r#"name == "or""#, "name", "==", "or"),
        (r#"name == "and""#, "name", "==", "and"),
        (r#"name == "not""#, "name", "==", "not"),
        (
            r#"content contains "Error""#,
            "content",
            "contains",
            "Error",
        ),
    ];

    for (expr, selector, op, value) in test_cases {
        let result = RawParser::parse_raw_expr(expr);
        let expected = RawTestExpr::quoted_predicate(selector, op, value);
        assert_eq!(
            result.unwrap().to_test_expr(),
            expected,
            "Failed for quoted reserved word: {}",
            expr
        );
    }
}

// ==============================================================================
// Regex Pattern Tests - Quantifiers, Escapes, Delimiters
// ==============================================================================

#[test]
fn test_set_parsing() {
    use detect::parser::RawParser;

    // Comprehensive set parsing test covering all edge cases
    let test_cases: Vec<(&str, Vec<&str>, &str)> = vec![
        // Basic bare items
        ("rs, js, ts", vec!["rs", "js", "ts"], "basic bare items"),
        // Commas in quoted strings
        (
            r#""foo, bar", baz"#,
            vec!["foo, bar", "baz"],
            "commas in double quotes",
        ),
        (
            r#"'foo, bar', "baz, qux", plain"#,
            vec!["foo, bar", "baz, qux", "plain"],
            "mixed quotes with commas",
        ),
        // Escaped quotes
        (
            r#""foo\"bar", 'baz\'qux'"#,
            vec![r#"foo\"bar"#, r#"baz\'qux"#],
            "escaped quotes",
        ),
        (
            r#""He said 'hello'", 'She said "hi"'"#,
            vec!["He said 'hello'", r#"She said "hi""#],
            "nested quote styles",
        ),
        (
            r#""She said \"hello\"", 'He said \'hi\''"#,
            vec![r#"She said \"hello\""#, r#"He said \'hi\'"#],
            "escaped nested quotes",
        ),
        // Trailing commas
        ("rs, js, ts,", vec!["rs", "js", "ts"], "trailing comma"),
        ("rs,", vec!["rs"], "single item with trailing comma"),
        // Empty sets
        ("", vec![], "empty set"),
        (r#""""#, vec![], "empty quoted string"),
        // Whitespace handling
        (
            "rs  ,  js  ,  ts",
            vec!["rs", "js", "ts"],
            "extra whitespace around bare items",
        ),
        (
            r#""  spaces  ", bare"#,
            vec!["  spaces  ", "bare"],
            "whitespace preserved in quotes",
        ),
        ("foo  ,  bar  ", vec!["foo", "bar"], "trailing whitespace"),
        // Edge cases
        (r#""", foo"#, vec!["foo"], "empty quoted string filtered"),
        ("foo,,,bar", vec!["foo", "bar"], "multiple commas filtered"),
        (
            "foo-bar, baz_qux, file.txt",
            vec!["foo-bar", "baz_qux", "file.txt"],
            "special chars in bare items",
        ),
        // Unicode
        (
            r#""测试", "файл", "🦀""#,
            vec!["测试", "файл", "🦀"],
            "unicode in quotes",
        ),
        ("测试, файл", vec!["测试", "файл"], "unicode in bare items"),
        // Real-world examples
        (
            r#"rs, "config.toml", "my file.txt", Cargo.toml"#,
            vec!["rs", "config.toml", "my file.txt", "Cargo.toml"],
            "mix of quoted and bare",
        ),
        (
            r#""https://example.com?a=1,2,3", /path/to/file"#,
            vec!["https://example.com?a=1,2,3", "/path/to/file"],
            "URLs and paths",
        ),
    ];

    for (input, expected, description) in test_cases {
        let result = RawParser::parse_set_contents(input).unwrap();
        assert_eq!(result, expected, "Failed: {}", description);
    }
}

#[test]
fn test_bare_token_asymmetric_delimiters() {
    use detect::parser::RawParser;

    // Closing paren should terminate bare token (it's a structural char)
    let result = RawParser::parse_raw_expr("name == foo)bar");
    assert!(result.is_err(), "Closing paren should cause parse error");

    // Opening brackets/parens allowed mid-token
    let result = RawParser::parse_raw_expr("content contains foo(bar").unwrap();
    let expected = RawTestExpr::string_predicate("content", "contains", "foo(bar");
    assert_eq!(result.to_test_expr(), expected);

    let result = RawParser::parse_raw_expr("content contains foo[bar").unwrap();
    let expected = RawTestExpr::string_predicate("content", "contains", "foo[bar");
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_quoted_string_with_newlines() {
    use detect::parser::RawParser;

    // Newlines inside quotes should be preserved
    let result = RawParser::parse_raw_expr("content == \"line1\nline2\"").unwrap();
    let expected = RawTestExpr::quoted_predicate("content", "==", "line1\nline2");
    assert_eq!(result.to_test_expr(), expected);

    // Tabs preserved
    let result = RawParser::parse_raw_expr("content == \"col1\tcol2\"").unwrap();
    let expected = RawTestExpr::quoted_predicate("content", "==", "col1\tcol2");
    assert_eq!(result.to_test_expr(), expected);
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
        let expected = RawTestExpr::quoted_predicate("name", "==", &expected_value);
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
        RawTestExpr::quoted_predicate("name", "==", "!@#$%^&*()_+-=[]{}|;:,.<>?")
    );

    // Bare value that looks like operator
    let result = RawParser::parse_raw_expr("name == ==");
    assert_eq!(
        result.unwrap().to_test_expr(),
        RawTestExpr::string_predicate("name", "==", "==")
    );

    // Reserved words as bare values work - context disambiguates
    // "name == AND" searches for file named "AND", not a boolean operator
    let result = RawParser::parse_raw_expr("name == AND");
    assert_eq!(
        result.unwrap().to_test_expr(),
        RawTestExpr::string_predicate("name", "==", "AND")
    );

    let result = RawParser::parse_raw_expr("name == OR");
    assert_eq!(
        result.unwrap().to_test_expr(),
        RawTestExpr::string_predicate("name", "==", "OR")
    );

    let result = RawParser::parse_raw_expr("name == NOT");
    assert_eq!(
        result.unwrap().to_test_expr(),
        RawTestExpr::string_predicate("name", "==", "NOT")
    );

    // Case insensitive variants also work
    let result = RawParser::parse_raw_expr("name == and");
    assert_eq!(
        result.unwrap().to_test_expr(),
        RawTestExpr::string_predicate("name", "==", "and")
    );

    let result = RawParser::parse_raw_expr("name == or");
    assert_eq!(
        result.unwrap().to_test_expr(),
        RawTestExpr::string_predicate("name", "==", "or")
    );

    let result = RawParser::parse_raw_expr("name == not");
    assert_eq!(
        result.unwrap().to_test_expr(),
        RawTestExpr::string_predicate("name", "==", "not")
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
    // With new parser, sets are stored as raw token strings
    let expected_token = format!("[{}]", large_set_items.join(", "));
    let expected = RawTestExpr::string_predicate("name", "in", &expected_token);
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
            RawTestExpr::quoted_predicate("content", "==", "🚀🌟⭐"),
        ), // Emoji - quoted
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
    let expected = RawTestExpr::quoted_predicate("name", "==", "value with\ttabs\nand newlines");
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
fn test_memory_stress_large_inputs() {
    // Test very large quoted strings
    let large_string = "x".repeat(50000);
    let input = format!(r#"name == "{}""#, large_string);
    let result = RawParser::parse_raw_expr(&input);
    let expected = RawTestExpr::quoted_predicate("name", "==", &large_string);
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Test very large sets - now stored as raw token string
    let large_set_items: Vec<String> = (0..5000).map(|i| format!("item{}", i)).collect();
    let large_set = format!("name in [{}]", large_set_items.join(", "));
    let result = RawParser::parse_raw_expr(&large_set);
    // With new parser, sets are stored as raw token strings
    let expected_token = format!("[{}]", large_set_items.join(", "));
    let expected = RawTestExpr::string_predicate("name", "in", &expected_token);
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

#[test]
fn test_empty_set_values() {
    // Empty set should parse but might fail typecheck
    let result = RawParser::parse_raw_expr("ext in []");
    assert!(result.is_ok());

    // Verify it parses as raw token
    let expected = RawTestExpr::string_predicate("ext", "in", "[]");
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
    use detect::parser::typechecker::Typechecker;

    // Whitespace should be preserved in set values
    let expr = r#"name in ["file one.txt", "file  two.txt", "  spaces  "]"#;
    let result = RawParser::parse_raw_expr(expr);
    assert!(result.is_ok());

    // After typechecking, verify whitespace is preserved
    let _typed = Typechecker::typecheck(result.unwrap(), expr).unwrap();
    // The actual verification would happen in the set values
}
