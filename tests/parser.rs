use detect::parser::test_utils::RawTestExpr;
use detect::parser::*;

// ==============================================================================
// Basic Syntax Tests - Predicates, Values, Quotes
// ==============================================================================

#[test]
fn test_simple_predicate() {
    let result = RawParser::parse_raw_expr("name == foo").unwrap();
    let expected = RawTestExpr::string_predicate("name", "==", "foo");
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_quoted_values() {
    let result = RawParser::parse_raw_expr(r#"filename == "my file.txt""#).unwrap();
    let expected = RawTestExpr::quoted_predicate("filename", "==", "my file.txt");
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_single_quoted_values() {
    let result = RawParser::parse_raw_expr("filename == 'my file.txt'").unwrap();
    let expected = RawTestExpr::quoted_predicate("filename", "==", "my file.txt");
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_escape_sequences() {
    // Test double quote escapes
    let result = RawParser::parse_raw_expr(r#"name == "file\"with\"quotes""#).unwrap();
    let expected = RawTestExpr::quoted_predicate("name", "==", r#"file\"with\"quotes"#);
    assert_eq!(result.to_test_expr(), expected);

    // Test various escape sequences
    let result = RawParser::parse_raw_expr(r#"content == "line1\nline2\ttab\\backslash""#).unwrap();
    let expected =
        RawTestExpr::quoted_predicate("content", "==", r#"line1\nline2\ttab\\backslash"#);
    assert_eq!(result.to_test_expr(), expected);

    // Test single quote escapes
    let result = RawParser::parse_raw_expr(r"name == 'file\'with\'quotes'").unwrap();
    let expected = RawTestExpr::quoted_predicate("name", "==", r"file\'with\'quotes");
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_set_values() {
    let result = RawParser::parse_raw_expr("ext in [rs, js, ts]").unwrap();
    // With new parser, sets are raw tokens - spaces preserved
    let expected = RawTestExpr::string_predicate("ext", "in", "[rs, js, ts]");
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_mixed_set() {
    let result = RawParser::parse_raw_expr(r#"name in [README, "my file", config]"#).unwrap();
    let expected = RawTestExpr::string_predicate("name", "in", r#"[README, "my file", config]"#);
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_set_with_quotes_and_escapes() {
    let result = RawParser::parse_raw_expr(r#"name in ["file\"1", 'file\'2', plain]"#).unwrap();
    let expected = RawTestExpr::string_predicate("name", "in", r#"["file\"1", 'file\'2', plain]"#);
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_empty_set() {
    let result = RawParser::parse_raw_expr("ext in []").unwrap();
    let expected = RawTestExpr::string_predicate("ext", "in", "[]");
    assert_eq!(result.to_test_expr(), expected);
}

// ==============================================================================
// Boolean Logic Tests - AND, OR, NOT, Precedence
// ==============================================================================

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
    let result =
        RawParser::parse_raw_expr(r#"(name == "test.rs" OR ext in [js, ts]) AND NOT size > 1mb"#)
            .unwrap();

    let expected = RawTestExpr::and(
        RawTestExpr::or(
            RawTestExpr::quoted_predicate("name", "==", "test.rs"),
            RawTestExpr::string_predicate("ext", "in", "[js, ts]"),
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

    // Unclosed bracket - now parses as bare token (grammar is permissive)
    // This is valid: searches for files named "[foo"
    let result = RawParser::parse_raw_expr("name in [foo");
    assert!(
        result.is_ok(),
        "Permissive grammar allows [foo as bare token"
    );

    // Unclosed quote
    let result = RawParser::parse_raw_expr(r#"name == "unclosed"#);
    assert!(result.is_err());
}

#[test]
fn test_invalid_escape_sequences() {
    // Since we're a syntax-only parser, we preserve escape sequences without validating them
    // This previously "invalid" escape sequence is now just preserved as-is
    let result = RawParser::parse_raw_expr(r#"name == "invalid\x""#);
    let expected = RawTestExpr::quoted_predicate("name", "==", r"invalid\x");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Unterminated string (this is actually a syntax error, not escape error)
    let result = RawParser::parse_raw_expr("name == \"unterminated");
    assert!(result.is_err());
}

// ==============================================================================
// Quote Error Detection Tests - Unterminated and Stray Quotes
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
fn test_whitespace_handling() {
    // Basic predicate whitespace tolerance
    let result1 = RawParser::parse_raw_expr("name==foo").unwrap();
    let result2 = RawParser::parse_raw_expr("name == foo").unwrap();
    let result3 = RawParser::parse_raw_expr("  name   ==   foo  ").unwrap();

    assert_eq!(result1.to_test_expr(), result2.to_test_expr());
    assert_eq!(result2.to_test_expr(), result3.to_test_expr());

    // Whitespace in sets (preserved as raw token)
    let result = RawParser::parse_raw_expr("ext in [ rs , js , ts ]").unwrap();
    let expected = RawTestExpr::string_predicate("ext", "in", "[ rs , js , ts ]");
    assert_eq!(result.to_test_expr(), expected);
}

#[test]
fn test_edge_cases() {
    // Empty string value
    let result = RawParser::parse_raw_expr(r#"name == """#).unwrap();
    let expected = RawTestExpr::quoted_predicate("name", "==", "");
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

// ==============================================================================
// Edge case and adversarial tests (formerly in stress_test_parser.rs)
// ==============================================================================

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
