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
    assert!(result.is_ok(), "Permissive grammar allows [foo as bare token");

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

// New functionality: Unquoted regex patterns with special characters
#[test]
fn test_unquoted_regex_patterns_with_brackets() {
    // Character classes should work unquoted
    let result = RawParser::parse_raw_expr("content ~= [0-9]");
    assert!(result.is_ok(), "Should parse character class [0-9]");
    let expected = RawTestExpr::string_predicate("content", "~=", "[0-9]");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // More complex character classes
    let result = RawParser::parse_raw_expr("name ~= [a-zA-Z_]");
    assert!(result.is_ok(), "Should parse character class [a-zA-Z_]");
    let expected = RawTestExpr::string_predicate("name", "~=", "[a-zA-Z_]");
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

#[test]
fn test_unquoted_regex_patterns_with_parentheses() {
    // Alternation groups should work unquoted
    let result = RawParser::parse_raw_expr("content ~= (Result|Option)");
    assert!(result.is_ok(), "Should parse alternation (Result|Option)");
    let expected = RawTestExpr::string_predicate("content", "~=", "(Result|Option)");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // Capture groups
    let result = RawParser::parse_raw_expr("name ~= (test)");
    assert!(result.is_ok(), "Should parse capture group (test)");
    let expected = RawTestExpr::string_predicate("name", "~=", "(test)");
    assert_eq!(result.unwrap().to_test_expr(), expected);
}

#[test]
fn test_unquoted_regex_patterns_with_curly_braces() {
    // Quantifiers should work unquoted
    let result = RawParser::parse_raw_expr("content ~= \\d{1,3}");
    assert!(result.is_ok(), "Should parse quantifier \\d{{1,3}}");
    let expected = RawTestExpr::string_predicate("content", "~=", "\\d{1,3}");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    let result = RawParser::parse_raw_expr("content ~= a{2,5}");
    assert!(result.is_ok(), "Should parse quantifier a{{2,5}}");
    let expected = RawTestExpr::string_predicate("content", "~=", "a{2,5}");
    assert_eq!(result.unwrap().to_test_expr(), expected);
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
    // Rust attribute syntax: #[test] works unquoted
    let result = RawParser::parse_raw_expr("content contains #[test]");
    assert!(result.is_ok(), "Should parse #[test] as bare token");
    let expected = RawTestExpr::string_predicate("content", "contains", "#[test]");
    assert_eq!(result.unwrap().to_test_expr(), expected);

    // More bracket patterns
    let result = RawParser::parse_raw_expr("content contains #[cfg]");
    assert!(result.is_ok(), "Should parse #[cfg]");

    let result = RawParser::parse_raw_expr("content contains #[derive]");
    assert!(result.is_ok(), "Should parse #[derive]");
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
    assert!(result.is_ok(), "Should parse [0-9] as bracketed token for regex");
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
        assert_eq!(result.unwrap().to_test_expr(), expected, "Failed for: {}", expr);
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
                assert!(pred.value.to_string().starts_with('['), "Should be bracketed: {}", expr);
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
        assert_eq!(result.unwrap().to_test_expr(), expected, "Failed for: {}", expr);
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
