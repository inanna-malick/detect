use detect::parser::test_utils::RawTestExpr;
use detect::parser::*;

// ==============================================================================

#[test]
fn test_unquoted_regex_patterns() {
    // Comprehensive test for unquoted regex patterns with various delimiters and quantifiers
    let test_cases = vec![
        // Basic bracket patterns
        ("content ~= [0-9]", "[0-9]"),
        ("content ~= [a-zA-Z_]", "[a-zA-Z_]"),
        // Brackets with quantifiers
        ("content ~= [0-9]+", "[0-9]+"),
        ("content ~= [a-z]*", "[a-z]*"),
        ("content ~= [A-Z]?", "[A-Z]?"),
        ("content ~= [0-9]{2,4}", "[0-9]{2,4}"),
        // Complex multi-bracket patterns
        ("content ~= [a-z]+[A-Z]+", "[a-z]+[A-Z]+"),
        ("content ~= [A-Z][a-z]+", "[A-Z][a-z]+"),
        ("content ~= [0-9]+\\.[0-9]+", "[0-9]+\\.[0-9]+"),
        // Basic paren patterns
        ("content ~= (Result|Option)", "(Result|Option)"),
        ("content ~= (test)", "(test)"),
        // Parentheses with quantifiers
        ("content ~= (foo|bar)+", "(foo|bar)+"),
        ("content ~= (test)*", "(test)*"),
        ("content ~= (a|b){2,3}", "(a|b){2,3}"),
        // Basic curly brace quantifiers
        ("content ~= \\d{1,3}", "\\d{1,3}"),
        ("content ~= a{2,5}", "a{2,5}"),
        // Mixed patterns
        (
            "content ~= [a-z]+@[a-z]+\\.[a-z]+",
            "[a-z]+@[a-z]+\\.[a-z]+",
        ),
        (
            "content ~= \\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}",
            "\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}",
        ),
    ];

    for (expr, expected_value) in test_cases {
        let result = RawParser::parse_raw_expr(expr);
        assert!(result.is_ok(), "Should parse: {}", expr);
        let expected = RawTestExpr::string_predicate("content", "~=", expected_value);
        assert_eq!(
            result.unwrap().to_test_expr(),
            expected,
            "Failed for: {}",
            expr
        );
    }
}

#[test]
fn test_escape_sequences_in_bare_tokens() {
    // Verify parser captures escape sequences literally without interpretation
    // Tests critical regex escape sequences reported as broken: \(, \w{n,m}, \b
    let test_cases = vec![
        // Basic escape sequences
        (r"content ~= \d+", r"\d+"),
        (r"content ~= \s+", r"\s+"),
        (r"content ~= \w+", r"\w+"),
        (r"content ~= \t", r"\t"),
        (r"content ~= \n", r"\n"),
        // CRITICAL: Escaped parentheses (reported as broken - "missing closing parenthesis")
        (r"content ~= \(", r"\("),
        (r"content ~= \)", r"\)"),
        (r"content ~= \\(", r"\\("), // Backslash followed by paren
        // Complex patterns with escaped parens
        (r"content ~= fn\s+\w+\(", r"fn\s+\w+\("),
        (r"content ~= \w+\(\)", r"\w+\(\)"),
        // Literal backslash
        (r"content ~= \\", r"\\"),
        (r"content ~= \\\\", r"\\\\"),
        // Word boundaries (reported as broken - returns 0 matches)
        (r"content ~= \b\w+\b", r"\b\w+\b"),
        (r"content ~= \bfn\b", r"\bfn\b"),
        (r"content ~= \B", r"\B"),
        // Anchors (reported as broken)
        (r"content ~= ^\s*use", r"^\s*use"),
        (r"content ~= \A\w+", r"\A\w+"),
        (r"content ~= \z", r"\z"),
        (r"content ~= \Z", r"\Z"),
        (r"content ~= \G", r"\G"),
        // Unicode and hex escapes (reported as broken)
        (r"content ~= \p{L}+", r"\p{L}+"),
        (r"content ~= \p{Nd}+", r"\p{Nd}+"),
        (r"content ~= \x{41}", r"\x{41}"),
        (r"content ~= \x41", r"\x41"),
        // Special escapes
        (r"content ~= \Q...\E", r"\Q...\E"),
        (r"content ~= \K", r"\K"),
        (r"content ~= \X", r"\X"),
    ];

    for (expr, expected_value) in test_cases {
        let result = RawParser::parse_raw_expr(expr);
        assert!(result.is_ok(), "Failed to parse: {}", expr);

        let pred = match result.unwrap().to_test_expr() {
            RawTestExpr::Predicate(p) => p,
            _ => panic!("Expected predicate for: {}", expr),
        };

        assert_eq!(
            pred.value.to_string(),
            expected_value,
            "Parser mangled escape sequence in: {}",
            expr
        );
    }
}

#[test]
fn test_curly_brace_quantifiers_on_escapes() {
    // CRITICAL: Verify \w{n,m} style quantifiers aren't split by grammar
    // Reported issue: \w{5,10} and \w{3} return 0 matches
    let test_cases = vec![
        // Exact count quantifiers
        (r"content ~= \w{3}", r"\w{3}"),
        (r"content ~= \d{5}", r"\d{5}"),
        (r"content ~= \s{2}", r"\s{2}"),
        // Range quantifiers (reported as broken - returns 0 matches)
        (r"content ~= \w{5,10}", r"\w{5,10}"),
        (r"content ~= \d{1,3}", r"\d{1,3}"),
        (r"content ~= \s{2,4}", r"\s{2,4}"),
        (r"content ~= \w{3,}", r"\w{3,}"), // Open-ended
        // Complex patterns with curly quantifiers
        (
            r"content ~= \d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}",
            r"\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}",
        ),
        (r"content ~= \w{3,5}_\w{2,4}", r"\w{3,5}_\w{2,4}"),
        (r"content ~= fn\w{1,20}\(", r"fn\w{1,20}\("),
        // Character classes with quantifiers (already tested elsewhere, verify consistency)
        (r"content ~= [a-z]{2,4}", "[a-z]{2,4}"),
        (r"content ~= [0-9]{3}", "[0-9]{3}"),
    ];

    for (expr, expected_value) in test_cases {
        let result = RawParser::parse_raw_expr(expr);
        assert!(result.is_ok(), "Failed to parse: {}", expr);

        let pred = match result.unwrap().to_test_expr() {
            RawTestExpr::Predicate(p) => p,
            _ => panic!("Expected predicate for: {}", expr),
        };

        assert_eq!(
            pred.value.to_string(),
            expected_value,
            "Curly brace quantifier was split or mangled: {}",
            expr
        );
    }
}

#[test]
fn test_quoted_vs_unquoted_escape_handling() {
    // Verify quoted and unquoted forms produce identical values for escape sequences
    let test_cases = vec![
        // Escaped parentheses
        (r"content ~= \(", r#"content ~= "\(""#, r"\("),
        (r"content ~= \)", r#"content ~= "\)""#, r"\)"),
        // Escape sequences
        (r"content ~= \s+", r#"content ~= "\s+""#, r"\s+"),
        (r"content ~= \d{3}", r#"content ~= "\d{3}""#, r"\d{3}"),
        (
            r"content ~= \w{5,10}",
            r#"content ~= "\w{5,10}""#,
            r"\w{5,10}",
        ),
        // Backslash handling
        (r"content ~= \\", r#"content ~= "\\""#, r"\\"),
        (r"content ~= \\\(", r#"content ~= "\\\(""#, r"\\\("),
        // Complex patterns
        (
            r"content ~= fn\s+\w+\(",
            r#"content ~= "fn\s+\w+\(""#,
            r"fn\s+\w+\(",
        ),
    ];

    for (unquoted_expr, quoted_expr, expected_value) in test_cases {
        // Test unquoted
        let unquoted_result = RawParser::parse_raw_expr(unquoted_expr);
        assert!(
            unquoted_result.is_ok(),
            "Failed to parse unquoted: {}",
            unquoted_expr
        );
        let unquoted_pred = match unquoted_result.unwrap().to_test_expr() {
            RawTestExpr::Predicate(p) => p,
            _ => panic!("Expected predicate"),
        };

        // Test quoted
        let quoted_result = RawParser::parse_raw_expr(quoted_expr);
        assert!(
            quoted_result.is_ok(),
            "Failed to parse quoted: {}",
            quoted_expr
        );
        let quoted_pred = match quoted_result.unwrap().to_test_expr() {
            RawTestExpr::Predicate(p) => p,
            _ => panic!("Expected predicate"),
        };

        // Both should produce same value
        assert_eq!(
            unquoted_pred.value.to_string(),
            expected_value,
            "Unquoted value mismatch: {}",
            unquoted_expr
        );

        assert_eq!(
            quoted_pred.value.to_string(),
            expected_value,
            "Quoted value mismatch: {}",
            quoted_expr
        );

        assert_eq!(
            unquoted_pred.value.to_string(),
            quoted_pred.value.to_string(),
            "Quoted/unquoted mismatch between {} and {}",
            unquoted_expr,
            quoted_expr
        );
    }
}

#[test]
fn test_regex_quantifiers_in_boolean_expressions() {
    // Category 1: Operator boundaries - quantified patterns followed by AND/OR
    // These test that the parser correctly terminates pattern matching at boolean operators
    let test_cases = vec![
        (
            "content ~= [0-9]+ AND size > 1kb",
            RawTestExpr::and(
                RawTestExpr::string_predicate("content", "~=", "[0-9]+"),
                RawTestExpr::string_predicate("size", ">", "1kb"),
            ),
        ),
        (
            "content ~= [a-z]+ OR name == bar",
            RawTestExpr::or(
                RawTestExpr::string_predicate("content", "~=", "[a-z]+"),
                RawTestExpr::string_predicate("name", "==", "bar"),
            ),
        ),
        (
            "content ~= [0-9]{2,4} AND name == test",
            RawTestExpr::and(
                RawTestExpr::string_predicate("content", "~=", "[0-9]{2,4}"),
                RawTestExpr::string_predicate("name", "==", "test"),
            ),
        ),
        (
            "name ~= (foo|bar)+ OR size > 1kb",
            RawTestExpr::or(
                RawTestExpr::string_predicate("name", "~=", "(foo|bar)+"),
                RawTestExpr::string_predicate("size", ">", "1kb"),
            ),
        ),
        (
            "content ~= [A-Z]* AND type == file",
            RawTestExpr::and(
                RawTestExpr::string_predicate("content", "~=", "[A-Z]*"),
                RawTestExpr::string_predicate("type", "==", "file"),
            ),
        ),
    ];

    for (expr, expected) in test_cases {
        let result = RawParser::parse_raw_expr(expr);
        assert!(result.is_ok(), "Should parse: {}", expr);
        assert_eq!(
            result.unwrap().to_test_expr(),
            expected,
            "Structure mismatch for: {}",
            expr
        );
    }

    // Category 2: Greedy trailing char consumption
    // Tests that non-whitespace chars after quantifier are captured as part of value
    let trailing_char_cases = vec![
        (
            "content ~= [0-9]+@domain AND name == test",
            RawTestExpr::and(
                RawTestExpr::string_predicate("content", "~=", "[0-9]+@domain"),
                RawTestExpr::string_predicate("name", "==", "test"),
            ),
        ),
        (
            "content ~= [0-9]+-[0-9]+ OR size > 1kb",
            RawTestExpr::or(
                RawTestExpr::string_predicate("content", "~=", "[0-9]+-[0-9]+"),
                RawTestExpr::string_predicate("size", ">", "1kb"),
            ),
        ),
        (
            "content ~= [a-z]+_suffix AND type == file",
            RawTestExpr::and(
                RawTestExpr::string_predicate("content", "~=", "[a-z]+_suffix"),
                RawTestExpr::string_predicate("type", "==", "file"),
            ),
        ),
        (
            "content ~= [0-9]+.ext OR name == bar",
            RawTestExpr::or(
                RawTestExpr::string_predicate("content", "~=", "[0-9]+.ext"),
                RawTestExpr::string_predicate("name", "==", "bar"),
            ),
        ),
        (
            "content ~= [0-9]+:port AND ext == rs",
            RawTestExpr::and(
                RawTestExpr::string_predicate("content", "~=", "[0-9]+:port"),
                RawTestExpr::string_predicate("ext", "==", "rs"),
            ),
        ),
        (
            "content ~= [a-z]+=value OR name == test",
            RawTestExpr::or(
                RawTestExpr::string_predicate("content", "~=", "[a-z]+=value"),
                RawTestExpr::string_predicate("name", "==", "test"),
            ),
        ),
    ];

    for (expr, expected) in trailing_char_cases {
        let result = RawParser::parse_raw_expr(expr);
        assert!(result.is_ok(), "Should parse: {}", expr);
        assert_eq!(
            result.unwrap().to_test_expr(),
            expected,
            "Structure mismatch for: {}",
            expr
        );
    }

    // Category 3: Multi-bracket patterns in boolean context
    // Tests complex regex patterns with multiple bracket groups combined with boolean operators
    let multi_bracket_cases = vec![
        (
            "content ~= [a-z]+[A-Z]+ AND size > 1kb",
            RawTestExpr::and(
                RawTestExpr::string_predicate("content", "~=", "[a-z]+[A-Z]+"),
                RawTestExpr::string_predicate("size", ">", "1kb"),
            ),
        ),
        (
            "content ~= [0-9]+\\.[0-9]+ OR name == test",
            RawTestExpr::or(
                RawTestExpr::string_predicate("content", "~=", "[0-9]+\\.[0-9]+"),
                RawTestExpr::string_predicate("name", "==", "test"),
            ),
        ),
        (
            "content ~= [a-z]+[A-Z]+[0-9]+ AND type == file",
            RawTestExpr::and(
                RawTestExpr::string_predicate("content", "~=", "[a-z]+[A-Z]+[0-9]+"),
                RawTestExpr::string_predicate("type", "==", "file"),
            ),
        ),
        (
            "content ~= \\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3} AND name == config",
            RawTestExpr::and(
                RawTestExpr::string_predicate(
                    "content",
                    "~=",
                    "\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}",
                ),
                RawTestExpr::string_predicate("name", "==", "config"),
            ),
        ),
        (
            "content ~= [a-z]+@[a-z]+\\.[a-z]+ OR size > 1mb",
            RawTestExpr::or(
                RawTestExpr::string_predicate("content", "~=", "[a-z]+@[a-z]+\\.[a-z]+"),
                RawTestExpr::string_predicate("size", ">", "1mb"),
            ),
        ),
    ];

    for (expr, expected) in multi_bracket_cases {
        let result = RawParser::parse_raw_expr(expr);
        assert!(result.is_ok(), "Should parse: {}", expr);
        assert_eq!(
            result.unwrap().to_test_expr(),
            expected,
            "Structure mismatch for: {}",
            expr
        );
    }

    // Category 7: Negation and grouping with quantified patterns
    let negation_cases = vec![
        (
            "NOT content ~= [0-9]+ AND name == test",
            RawTestExpr::and(
                RawTestExpr::not(RawTestExpr::string_predicate("content", "~=", "[0-9]+")),
                RawTestExpr::string_predicate("name", "==", "test"),
            ),
        ),
        (
            "(content ~= [0-9]+) AND size > 1kb",
            RawTestExpr::and(
                RawTestExpr::string_predicate("content", "~=", "[0-9]+"),
                RawTestExpr::string_predicate("size", ">", "1kb"),
            ),
        ),
        (
            "(content ~= [a-z]+ OR name == test) AND type == file",
            RawTestExpr::and(
                RawTestExpr::or(
                    RawTestExpr::string_predicate("content", "~=", "[a-z]+"),
                    RawTestExpr::string_predicate("name", "==", "test"),
                ),
                RawTestExpr::string_predicate("type", "==", "file"),
            ),
        ),
        (
            "NOT (content ~= [a-z]+[A-Z]+) OR size > 1kb",
            RawTestExpr::or(
                RawTestExpr::not(RawTestExpr::string_predicate(
                    "content",
                    "~=",
                    "[a-z]+[A-Z]+",
                )),
                RawTestExpr::string_predicate("size", ">", "1kb"),
            ),
        ),
        (
            "((content ~= [a-z]+) OR (name ~= [0-9]+)) AND type == file",
            RawTestExpr::and(
                RawTestExpr::or(
                    RawTestExpr::string_predicate("content", "~=", "[a-z]+"),
                    RawTestExpr::string_predicate("name", "~=", "[0-9]+"),
                ),
                RawTestExpr::string_predicate("type", "==", "file"),
            ),
        ),
    ];

    for (expr, expected) in negation_cases {
        let result = RawParser::parse_raw_expr(expr);
        assert!(result.is_ok(), "Should parse: {}", expr);
        assert_eq!(
            result.unwrap().to_test_expr(),
            expected,
            "Structure mismatch for: {}",
            expr
        );
    }

    // Category 9: Stress test combinations
    let stress_cases = vec![
        (
            "(content ~= [a-z]+[A-Z]+[0-9]+ OR content ~= \\d{1,3}\\.\\d{1,3}) AND (name ~= (foo|bar)+ OR size > 1kb) AND NOT type == dir",
            RawTestExpr::and(
                RawTestExpr::and(
                    RawTestExpr::or(
                        RawTestExpr::string_predicate("content", "~=", "[a-z]+[A-Z]+[0-9]+"),
                        RawTestExpr::string_predicate("content", "~=", "\\d{1,3}\\.\\d{1,3}"),
                    ),
                    RawTestExpr::or(
                        RawTestExpr::string_predicate("name", "~=", "(foo|bar)+"),
                        RawTestExpr::string_predicate("size", ">", "1kb"),
                    ),
                ),
                RawTestExpr::not(RawTestExpr::string_predicate("type", "==", "dir")),
            ),
        ),
        (
            "content ~= [0-9]+\\.[0-9]+\\.[0-9]+\\.[0-9]+ AND name ~= [a-z]+@[a-z]+\\.[a-z]+ OR ext in [rs, js, ts]",
            RawTestExpr::or(
                RawTestExpr::and(
                    RawTestExpr::string_predicate("content", "~=", "[0-9]+\\.[0-9]+\\.[0-9]+\\.[0-9]+"),
                    RawTestExpr::string_predicate("name", "~=", "[a-z]+@[a-z]+\\.[a-z]+"),
                ),
                RawTestExpr::string_predicate("ext", "in", "[rs, js, ts]"),
            ),
        ),
    ];

    for (expr, expected) in stress_cases {
        let result = RawParser::parse_raw_expr(expr);
        assert!(result.is_ok(), "Should parse: {}", expr);
        assert_eq!(
            result.unwrap().to_test_expr(),
            expected,
            "Structure mismatch for: {}",
            expr
        );
    }
}

#[test]
fn test_regex_quantifiers_whitespace_boundaries() {
    // Category 4: Whitespace should terminate pattern matching
    // These cases should FAIL because space splits the pattern
    let failure_cases = vec![
        "content ~= [0-9]+ [A-Z]+ AND next",
        "content ~= [0-9]+ bar AND next",
        "content ~= (foo)+ test OR name == bar",
    ];

    for expr in failure_cases {
        let result = RawParser::parse_raw_expr(expr);
        assert!(
            result.is_err(),
            "Should fail (whitespace splits pattern): {}",
            expr
        );
    }
}

#[test]
fn test_unquoted_bare_comma_separated_values() {
    // Bare comma-separated values without brackets should work
    let result = RawParser::parse_raw_expr("ext in rs,toml,md");
    assert!(result.is_ok(), "Should parse bare comma-separated values");
    let expected = RawTestExpr::string_predicate("ext", "in", "rs,toml,md");
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

// Comprehensive tests for special characters in bare tokens
#[test]
fn test_hash_with_brackets_in_bare_token() {
    // Rust attribute syntax: #[test], #[cfg], etc. should work unquoted
    let test_cases = vec![
        ("content contains #[test]", "#[test]"),
        ("content contains #[cfg]", "#[cfg]"),
        ("content contains #[derive]", "#[derive]"),
    ];

    for (expr, expected_value) in test_cases {
        let result = RawParser::parse_raw_expr(expr);
        let expected = RawTestExpr::string_predicate("content", "contains", expected_value);
        assert_eq!(
            result.unwrap().to_test_expr(),
            expected,
            "Failed for: {}",
            expr
        );
    }
}

#[test]
fn test_brackets_mid_token() {
    // Array access syntax: foo[0], array[index]
    let result = RawParser::parse_raw_expr("content contains foo[0]");
    assert!(result.is_ok(), "Should parse foo[0] as bare token");
    let expected = RawTestExpr::string_predicate("content", "contains", "foo[0]");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    let result = RawParser::parse_raw_expr("content contains array[index]");
    assert!(result.is_ok(), "Should parse array[index] as bare token");
    let expected = RawTestExpr::string_predicate("content", "contains", "array[index]");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Multiple brackets
    let result = RawParser::parse_raw_expr("content contains matrix[i][j]");
    assert!(result.is_ok(), "Should parse matrix[i][j] as bare token");
    let expected = RawTestExpr::string_predicate("content", "contains", "matrix[i][j]");
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

#[test]
fn test_brackets_vs_sets_disambiguation() {
    // Leading bracket = set (bracketed token matches first due to PEG ordering)
    let result = RawParser::parse_raw_expr("ext in [rs, js, ts]");
    assert!(result.is_ok(), "Should parse [rs, js, ts] as bracketed set");
    let expected = RawTestExpr::string_predicate("ext", "in", "[rs, js, ts]");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Non-leading bracket = bare token
    let result = RawParser::parse_raw_expr("content contains foo[bar]");
    assert!(result.is_ok(), "Should parse foo[bar] as bare token");
    let expected = RawTestExpr::string_predicate("content", "contains", "foo[bar]");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Regex character class with ~= operator
    let result = RawParser::parse_raw_expr("content ~= [0-9]");
    assert!(
        result.is_ok(),
        "Should parse [0-9] as bracketed token for regex"
    );
    let expected = RawTestExpr::string_predicate("content", "~=", "[0-9]");
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

#[test]
fn test_special_chars_in_contains() {
    // Various special characters that should work in literal contains
    let test_cases = vec![
        ("content contains #[test]", "#[test]"),
        ("content contains foo[0]", "foo[0]"),
        ("content contains array[i]", "array[i]"),
        ("content contains Vec<T>", "Vec<T>"),
        ("content contains Map<K,V>", "Map<K,V>"),
        ("content contains fn->ret", "fn->ret"),
        ("content contains a::b::c", "a::b::c"),
        ("content contains path/to/file", "path/to/file"),
        ("content contains x=5", "x=5"),
        ("content contains a+b", "a+b"),
        ("content contains a*b", "a*b"),
        ("content contains @override", "@override"),
        ("content contains $var", "$var"),
        ("content contains 50%", "50%"),
    ];

    for (expr, expected_value) in test_cases {
        let result = RawParser::parse_raw_expr(expr);
        assert!(result.is_ok(), "Should parse: {}", expr);
        let expected = RawTestExpr::string_predicate("content", "contains", expected_value);
        assert_eq!(
            result.unwrap().to_test_expr(),
            expected,
            "Failed for: {}",
            expr
        );
    }
}

#[test]
fn test_complex_patterns_with_brackets() {
    // Generic type patterns work unquoted (no closing ) before end)
    let result = RawParser::parse_raw_expr("content contains Vec<T>");
    assert!(result.is_ok(), "Should parse Vec<T>");
    let expected = RawTestExpr::string_predicate("content", "contains", "Vec<T>");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    let result = RawParser::parse_raw_expr("content contains HashMap<String,u8>");
    assert!(result.is_ok(), "Should parse HashMap<String,u8>");
    let expected = RawTestExpr::string_predicate("content", "contains", "HashMap<String,u8>");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Note: Patterns with closing ) need quotes because ) is structural
    // e.g., "#[cfg(test)]" must be quoted
}

#[test]
fn test_regression_sets_still_work() {
    // Ensure sets with leading brackets still parse correctly
    let test_cases = vec![
        "ext in [rs, toml]",
        "name in [foo, bar, baz]",
        "type in [file, dir]",
        "ext in [js, ts, jsx, tsx]",
    ];

    for expr in test_cases {
        let result = RawParser::parse_raw_expr(expr);
        assert!(result.is_ok(), "Should still parse set: {}", expr);
        // Verify it's recognized as a bracketed token (not bare)
        match result.unwrap().to_test_expr() {
            RawTestExpr::Predicate(pred) => {
                assert!(
                    pred.value.to_string().starts_with('['),
                    "Should be bracketed: {}",
                    expr
                );
            }
            _ => panic!("Expected predicate for: {}", expr),
        }
    }
}

#[test]
fn test_regression_regex_patterns_still_work() {
    // Ensure regex patterns still work after grammar change
    let test_cases = vec![
        ("content ~= [0-9]", "[0-9]"),
        ("content ~= [a-zA-Z]", "[a-zA-Z]"),
        ("content ~= (foo|bar)", "(foo|bar)"),
        ("content ~= \\d{1,3}", "\\d{1,3}"),
        ("content ~= \\w+", "\\w+"),
    ];

    for (expr, expected_value) in test_cases {
        let result = RawParser::parse_raw_expr(expr);
        assert!(result.is_ok(), "Should parse regex: {}", expr);
        let expected = RawTestExpr::string_predicate("content", "~=", expected_value);
        assert_eq!(
            result.unwrap().to_test_expr(),
            expected,
            "Failed for: {}",
            expr
        );
    }
}

#[test]
fn test_edge_case_empty_brackets() {
    // Empty brackets in different contexts
    let result = RawParser::parse_raw_expr("ext in []");
    assert!(result.is_ok(), "Should parse empty set");
    let expected = RawTestExpr::string_predicate("ext", "in", "[]");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Empty brackets as bare token (not at start)
    let result = RawParser::parse_raw_expr("content contains foo[]");
    assert!(result.is_ok(), "Should parse foo[] as bare token");
    let expected = RawTestExpr::string_predicate("content", "contains", "foo[]");
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

#[test]
fn test_mixed_bracket_scenarios() {
    // Combinations that test PEG ordering
    let result = RawParser::parse_raw_expr("content contains test[0] AND ext in [rs, js]");
    assert!(result.is_ok(), "Should parse mixed bracket usage");

    match result.unwrap().to_test_expr() {
        RawTestExpr::And(left, right) => {
            // Left side: test[0] as bare token
            match *left {
                RawTestExpr::Predicate(pred) => {
                    assert_eq!(pred.value.to_string(), "test[0]");
                }
                _ => panic!("Expected predicate on left"),
            }
            // Right side: [rs, js] as bracketed token
            match *right {
                RawTestExpr::Predicate(pred) => {
                    assert!(pred.value.to_string().starts_with('['));
                }
                _ => panic!("Expected predicate on right"),
            }
        }
        _ => panic!("Expected AND expression"),
    }
}

// ==============================================================================
// Set Parsing Tests
