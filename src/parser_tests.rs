#[cfg(test)]
mod tests {
    use crate::expr::Expr;
    use crate::parse_error::ParseError;
    use crate::parser::parse_expr;
    use crate::predicate::{
        Bound, MetadataPredicate, NamePredicate, NumberMatcher, Predicate,
        StreamingCompiledContentPredicate, StringMatcher,
    };
    use std::collections::HashSet;
    use std::sync::Arc;

    // Test basic parsing produces expected compiled predicates

    #[test]
    fn test_bare_path_shorthands() {
        // Test bare name shorthand
        let expr = parse_expr("name == README.md").unwrap();
        assert_eq!(
            expr,
            Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
                StringMatcher::Equals("README.md".to_owned())
            ))))
        );
        
        // Test bare stem shorthand
        let expr = parse_expr("stem == README").unwrap();
        assert_eq!(
            expr,
            Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::BaseName(
                StringMatcher::Equals("README".to_owned())
            ))))
        );
        
        // Test bare extension shorthand
        let expr = parse_expr("extension == md").unwrap();
        assert_eq!(
            expr,
            Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
                StringMatcher::Equals("md".to_owned())
            ))))
        );
        
        // Test short form 'ext'
        let expr = parse_expr("ext == rs").unwrap();
        assert_eq!(
            expr,
            Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
                StringMatcher::Equals("rs".to_owned())
            ))))
        );
        
        // Test bare parent shorthand
        let expr = parse_expr("parent == src").unwrap();
        assert_eq!(
            expr,
            Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::DirPath(
                StringMatcher::Equals("src".to_owned())
            ))))
        );
        
        // Test bare full shorthand
        let expr = parse_expr(r#"full == "/home/user/file.txt""#).unwrap();
        assert_eq!(
            expr,
            Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FullPath(
                StringMatcher::Equals("/home/user/file.txt".to_owned())
            ))))
        );
    }

    #[test]
    fn test_content_selector_forms() {
        // All forms should compile to the same content predicate
        let expected = Expr::Predicate(Predicate::Content(
            StreamingCompiledContentPredicate::new(regex::escape("TODO")).unwrap()
        ));
        
        // Test canonical form: content.text
        let expr = parse_expr(r#"content.text contains "TODO""#).unwrap();
        assert_eq!(expr, expected);
        
        // Test bare shorthand: text
        let expr = parse_expr(r#"text contains "TODO""#).unwrap();
        assert_eq!(expr, expected);
        
        // Test legacy form: contents (for backward compat)
        let expr = parse_expr(r#"contents contains "TODO""#).unwrap();
        assert_eq!(expr, expected);
        
        // Test legacy form: content (for backward compat)
        let expr = parse_expr(r#"content contains "TODO""#).unwrap();
        assert_eq!(expr, expected);
    }

    #[test]
    fn parse_name_equality() {
        let parsed = parse_expr("path.name == foo").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("foo".to_string()),
        ))));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_name_not_equal() {
        let parsed = parse_expr("path.name != bar").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::NotEquals("bar".to_string()),
        ))));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_name_regex() {
        let parsed = parse_expr("path.name ~= test.*").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::regex("test.*").unwrap(),
        ))));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_regex_with_special_chars() {
        // Test regex with curly braces, parentheses, etc.
        let cases = vec![
            r"path.name ~= ^[0-9]{10,13}.*\.ts$",
            r#"path.name ~= "(foo|bar)""#,
            r"path.name ~= test\?.*",
            r"contents ~= TODO.*\{.*\}",
        ];

        for expr in cases {
            let result = parse_expr(expr);
            assert!(result.is_ok(), "Failed to parse: {}", expr);
        }
    }

    #[test]
    fn parse_name_contains() {
        let parsed = parse_expr(r#"path.name contains "test""#).unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Contains("test".to_string()),
        ))));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_name_in_set() {
        // Test case that appears to be failing
        let result = parse_expr("path.name in [foo, bar, baz]");

        // Let's see what the actual error is
        match result {
            Ok(parsed) => {
                let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
                    StringMatcher::In(
                        ["foo".to_string(), "bar".to_string(), "baz".to_string()]
                            .into_iter()
                            .collect(),
                    ),
                ))));
                assert_eq!(parsed, expected);
            }
            Err(e) => {
                panic!("Failed to parse 'path.name in [foo, bar, baz]': {:?}", e);
            }
        }
    }

    #[test]
    fn debug_set_parsing() {
        // Test different variations to understand the parsing issue

        // Try without spaces
        let result1 = parse_expr("path.name in [foo,bar,baz]");
        println!("Without spaces: {:?}", result1.is_ok());

        // Try with quoted strings
        let result2 = parse_expr(r#"path.name in ["foo", "bar", "baz"]"#);
        println!("With quotes: {:?}", result2.is_ok());

        // Try mixed
        let result3 = parse_expr(r#"path.name in [foo, "bar", baz]"#);
        println!("Mixed: {:?}", result3.is_ok());

        // Try the original that's failing
        let result4 = parse_expr("path.name in [foo, bar, baz]");
        match &result4 {
            Ok(_) => println!("Original with spaces: OK"),
            Err(e) => println!("Original with spaces: Error - {:?}", e),
        }
    }

    #[test]
    fn test_set_parsing_bug() {
        // This is a parser bug: bare_char includes comma but set_char doesn't
        // This causes confusion when parsing sets with spaces after commas

        // These should all parse the same way, but they don't due to the bug
        let working = parse_expr("path.name in [foo,bar,baz]");
        assert!(working.is_ok(), "Should parse without spaces");

        let also_working = parse_expr(r#"path.name in ["foo","bar","baz"]"#);
        assert!(
            also_working.is_ok(),
            "Should parse with quotes and no spaces"
        );

        let also_working2 = parse_expr(r#"path.name in ["foo", "bar", "baz"]"#);
        assert!(also_working2.is_ok(), "Should parse with quotes and spaces");

        // This fails due to parser bug
        let failing = parse_expr("path.name in [foo, bar, baz]");
        assert!(
            failing.is_ok(),
            "Should parse with spaces - but doesn't due to parser bug: {:?}",
            failing
        );
    }

    #[test]
    fn parse_path_predicate() {
        let parsed = parse_expr(r#"path.full == "src/main.rs""#).unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FullPath(
            StringMatcher::Equals("src/main.rs".to_string()),
        ))));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_extension_predicate() {
        let parsed = parse_expr("path.extension == rs").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::Equals("rs".to_string()),
        ))));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_extension_in_set() {
        let parsed = parse_expr("path.extension in [js, ts, jsx, tsx]").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::In(
                [
                    "js".to_string(),
                    "ts".to_string(),
                    "jsx".to_string(),
                    "tsx".to_string(),
                ]
                .into_iter()
                .collect(),
            ),
        ))));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_meta_domain_support() {
        // Test canonical form: meta.size
        let result = parse_expr("meta.size > 1000");
        assert!(result.is_ok(), "Failed to parse meta.size");
        
        // Test canonical form: meta.type
        let result = parse_expr(r#"meta.type == "file""#);
        assert!(result.is_ok(), "Failed to parse meta.type");
        
        // Test canonical form: meta.depth
        let result = parse_expr("meta.depth > 2");
        assert!(result.is_ok(), "Failed to parse meta.depth");
        
        // Test bare forms still work
        let result = parse_expr("size > 1000");
        assert!(result.is_ok(), "Failed to parse bare size");
        
        // Verify both forms produce the same result
        let canonical_size = parse_expr("meta.size == 1000").unwrap();
        let bare_size = parse_expr("size == 1000").unwrap();
        assert_eq!(canonical_size, bare_size, "Size forms should be equivalent");
        
        let canonical_type = parse_expr(r#"meta.type == "file""#).unwrap();
        let bare_type = parse_expr(r#"type == "file""#).unwrap();
        assert_eq!(canonical_type, bare_type, "Type forms should be equivalent");
    }
    
    #[test]
    fn parse_size_comparisons() {
        let cases = vec![
            ("size == 100", NumberMatcher::Equals(100)),
            ("size > 100", NumberMatcher::In(Bound::Left(101..))),
            ("size >= 100", NumberMatcher::In(Bound::Left(100..))),
            ("size < 100", NumberMatcher::In(Bound::Right(..100))),
            ("size <= 100", NumberMatcher::In(Bound::Right(..101))),
        ];

        for (expr_str, expected_matcher) in cases {
            let parsed = parse_expr(expr_str).unwrap();
            let expected = Expr::Predicate(Predicate::Metadata(Arc::new(
                MetadataPredicate::Filesize(expected_matcher),
            )));
            assert_eq!(parsed, expected, "Failed for: {}", expr_str);
        }
    }

    #[test]
    fn parse_type_predicate() {
        let parsed = parse_expr("type == dir").unwrap();
        let expected = Expr::Predicate(Predicate::Metadata(Arc::new(MetadataPredicate::Type(
            StringMatcher::Equals("dir".to_string()),
        ))));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_content_regex() {
        let parsed = parse_expr(r#"contents ~= "TODO|FIXME""#).unwrap();
        let expected = Expr::Predicate(Predicate::Content(
            StreamingCompiledContentPredicate::new("TODO|FIXME".to_string()).unwrap(),
        ));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_content_contains() {
        let parsed = parse_expr(r#"contents contains "fn main""#).unwrap();
        // contains gets compiled to an escaped regex
        let expected = Expr::Predicate(Predicate::Content(
            StreamingCompiledContentPredicate::new(regex::escape("fn main")).unwrap(),
        ));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_time_domain_support() {
        use crate::predicate::TimeMatcher;
        
        // Test canonical form: time.modified
        let result = parse_expr(r#"time.modified > "-7.days""#);
        assert!(result.is_ok(), "Failed to parse time.modified");
        
        // Test canonical form: time.created
        let result = parse_expr(r#"time.created < "2024-01-01""#);
        assert!(result.is_ok(), "Failed to parse time.created");
        
        // Test canonical form: time.accessed
        let result = parse_expr(r#"time.accessed > "-30.minutes""#);
        assert!(result.is_ok(), "Failed to parse time.accessed");
        
        // Test bare forms still work
        let result = parse_expr(r#"modified > "-7.days""#);
        assert!(result.is_ok(), "Failed to parse bare modified");
        
        // Verify both forms produce the same result
        let canonical = parse_expr(r#"time.modified == "2024-01-01""#).unwrap();
        let bare = parse_expr(r#"modified == "2024-01-01""#).unwrap();
        assert_eq!(canonical, bare, "Canonical and bare forms should be equivalent");
    }
    
    #[test]
    fn parse_temporal_selectors() {
        // Just verify they parse to the correct variant
        let modified = parse_expr(r#"modified > "-7.days""#).unwrap();
        assert!(matches!(
            modified,
            Expr::Predicate(Predicate::Metadata(ref meta))
                if matches!(**meta, MetadataPredicate::Modified(_))
        ));

        let created = parse_expr(r#"created >= "today""#).unwrap();
        assert!(matches!(
            created,
            Expr::Predicate(Predicate::Metadata(ref meta))
                if matches!(**meta, MetadataPredicate::Created(_))
        ));

        let accessed = parse_expr(r#"accessed < "2024-01-01""#).unwrap();
        assert!(matches!(
            accessed,
            Expr::Predicate(Predicate::Metadata(ref meta))
                if matches!(**meta, MetadataPredicate::Accessed(_))
        ));
    }

    // Test boolean operators

    #[test]
    fn parse_and_expression() {
        let parsed = parse_expr("path.name == foo && path.extension == rs").unwrap();
        let left = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("foo".to_string()),
        ))));
        let right = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::Equals("rs".to_string()),
        ))));
        let expected = Expr::and(left, right);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_or_expression() {
        let parsed = parse_expr("path.name == foo || path.name == bar").unwrap();
        let left = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("foo".to_string()),
        ))));
        let right = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("bar".to_string()),
        ))));
        let expected = Expr::or(left, right);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_not_expression() {
        let parsed = parse_expr("!path.name == temp").unwrap();
        let inner = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("temp".to_string()),
        ))));
        let expected = Expr::negate(inner);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_operator_precedence() {
        // path.name == x || path.name == y && path.name == z
        // Should parse as: path.name == x || (path.name == y && path.name == z)
        let parsed = parse_expr("path.name == a || path.name == b && path.name == c").unwrap();

        let a = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("a".to_string()),
        ))));
        let b = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("b".to_string()),
        ))));
        let c = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("c".to_string()),
        ))));

        let b_and_c = Expr::and(b, c);
        let expected = Expr::or(a, b_and_c);

        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_parentheses_override() {
        // (path.name == x || path.name == y) && path.name == z
        let parsed = parse_expr("(path.name == a || path.name == b) && path.name == c").unwrap();

        let a = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("a".to_string()),
        ))));
        let b = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("b".to_string()),
        ))));
        let c = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("c".to_string()),
        ))));

        let a_or_b = Expr::or(a, b);
        let expected = Expr::and(a_or_b, c);

        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_complex_nested() {
        // !(path.name == x || path.extension == y) && (size > z || type == w)
        let parsed =
            parse_expr("!(path.name == x || path.extension == y) && (size > 100 || type == dir)")
                .unwrap();

        // Build expected tree
        let x = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("x".to_string()),
        ))));
        let y = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::Equals("y".to_string()),
        ))));
        let size_check = Expr::Predicate(Predicate::Metadata(Arc::new(
            MetadataPredicate::Filesize(NumberMatcher::In(Bound::Left(101..))),
        )));
        let type_check = Expr::Predicate(Predicate::Metadata(Arc::new(MetadataPredicate::Type(
            StringMatcher::Equals("dir".to_string()),
        ))));

        let x_or_y = Expr::or(x, y);
        let not_x_or_y = Expr::negate(x_or_y);
        let size_or_type = Expr::or(size_check, type_check);
        let expected = Expr::and(not_x_or_y, size_or_type);

        assert_eq!(parsed, expected);
    }

    // Test special features

    #[test]
    fn parse_quoted_values() {
        let double_quoted = parse_expr(r#"path.name == "my file.txt""#).unwrap();
        let single_quoted = parse_expr(r"path.name == 'my file.txt'").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("my file.txt".to_string()),
        ))));
        assert_eq!(double_quoted, expected);
        assert_eq!(single_quoted, expected);
    }

    #[test]
    fn parse_set_literal_variations() {
        let empty = parse_expr("path.extension in []").unwrap();
        let expected_empty = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::In(HashSet::new()),
        ))));
        assert_eq!(empty, expected_empty);

        // Single item
        let single = parse_expr("path.extension in [js]").unwrap();
        let expected_single = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::In(["js".to_string()].into_iter().collect()),
        ))));
        assert_eq!(single, expected_single);

        // Mixed quoted and unquoted
        let mixed = parse_expr(r#"path.name in [foo, "bar baz", 'qux']"#).unwrap();
        let expected_mixed = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::In(
                ["foo".to_string(), "bar baz".to_string(), "qux".to_string()]
                    .into_iter()
                    .collect(),
            ),
        ))));
        assert_eq!(mixed, expected_mixed);
    }

    #[test]
    fn parse_selector_aliases() {
        // Test remaining aliases (non-path selectors)
        let alias_variants = vec![
            ("size > 100kb", "filesize > 100kb"),
            ("type == dir", "filetype == dir"),
            ("contents ~= foo", "content ~= foo"),
            ("modified > today", "mtime > today"),
            ("created > today", "ctime > today"),
            ("accessed > today", "atime > today"),
        ];

        for (expr1_str, expr2_str) in alias_variants {
            let expr1 = parse_expr(expr1_str)
                .unwrap_or_else(|e| panic!("Failed to parse '{}': {:?}", expr1_str, e));
            let expr2 = parse_expr(expr2_str)
                .unwrap_or_else(|e| panic!("Failed to parse '{}': {:?}", expr2_str, e));
            assert_eq!(expr1, expr2, "{} should equal {}", expr1_str, expr2_str);
        }

        // Test that path is alias for path.full
        let path_alias = parse_expr("path == test").unwrap();
        let path_canonical = parse_expr("path.full == test").unwrap();
        assert_eq!(path_alias, path_canonical);
    }

    #[test]
    fn test_size_comparison_parsing() {
        // Size comparisons can use units or bare numbers
        // Bare numbers are interpreted as bytes
        assert!(
            parse_expr("size > 100").is_ok(),
            "Size with bare number should work (bytes)"
        );
        assert!(
            parse_expr("size > 100kb").is_ok(),
            "Size with kb unit should work"
        );
        assert!(
            parse_expr("size > 100mb").is_ok(),
            "Size with mb unit should work"
        );
        assert!(
            parse_expr("size > 100gb").is_ok(),
            "Size with gb unit should work"
        );
        assert!(
            parse_expr("size > 100tb").is_ok(),
            "Size with tb unit should work"
        );

        // Test that filesize alias works (was broken due to PEG ordering issue)
        assert!(
            parse_expr("filesize > 100").is_ok(),
            "Filesize with bare number should work"
        );
        assert!(
            parse_expr("filesize > 100kb").is_ok(),
            "Filesize with kb unit should work"
        );

        // Verify what bare numbers parse to
        if let Ok(expr) = parse_expr("size > 100") {
            // Should parse as Filesize(In(Left(100..)))
            if let Expr::Predicate(Predicate::Metadata(meta)) = expr {
                assert!(matches!(meta.as_ref(), MetadataPredicate::Filesize(_)));
            } else {
                panic!("size > 100 should parse as metadata predicate");
            }
        }
    }

    #[test]
    fn test_peg_ordering_aliases() {
        // Test that aliases parse correctly to the right predicates

        assert!(
            parse_expr("filesize > 100kb").is_ok(),
            "filesize should parse"
        );
        assert!(
            parse_expr("filetype == dir").is_ok(),
            "filetype should parse"
        );

        // Verify they parse to the correct predicates
        if let Ok(Expr::Predicate(pred)) = parse_expr("filesize > 100kb") {
            assert!(
                matches!(pred, Predicate::Metadata(_)),
                "filesize should be metadata predicate"
            );
        }

        if let Ok(Expr::Predicate(pred)) = parse_expr("filetype == dir") {
            assert!(
                matches!(pred, Predicate::Metadata(_)),
                "filetype should be metadata predicate"
            );
        }

        if let Ok(Expr::Predicate(pred)) = parse_expr("path.name == test") {
            assert!(
                matches!(pred, Predicate::Name(_)),
                "path.name should be name predicate"
            );
        }
    }

    #[test]
    fn test_word_form_boolean_operators() {
        // Test 'and' word form
        let word_and = parse_expr("name == foo and size > 100").unwrap();
        let symbol_and = parse_expr("name == foo && size > 100").unwrap();
        assert_eq!(word_and, symbol_and, "'and' and '&&' should be equivalent");
        
        // Test 'or' word form
        let word_or = parse_expr("name == foo or name == bar").unwrap();
        let symbol_or = parse_expr("name == foo || name == bar").unwrap();
        assert_eq!(word_or, symbol_or, "'or' and '||' should be equivalent");
        
        // Test 'not' word form
        let word_not = parse_expr("not name == foo").unwrap();
        let symbol_not = parse_expr("!name == foo").unwrap();
        assert_eq!(word_not, symbol_not, "'not' and '!' should be equivalent");
        
        // Test complex expression with word forms
        let complex_word = parse_expr("name == foo and not (size > 100 or type == dir)").unwrap();
        let complex_symbol = parse_expr("name == foo && !(size > 100 || type == dir)").unwrap();
        assert_eq!(complex_word, complex_symbol, "Complex expressions should work with word forms");
        
        // Test mixed forms (word and symbol)
        let mixed = parse_expr("name == foo and size > 100 || not type == dir");
        assert!(mixed.is_ok(), "Mixed word and symbol forms should work");
    }
    
    #[test]
    fn parse_operator_aliases() {
        // = vs ==
        let eq1 = parse_expr("path.name = foo").unwrap();
        let eq2 = parse_expr("path.name == foo").unwrap();
        assert_eq!(eq1, eq2);

        // ~ vs ~= vs =~
        let regex1 = parse_expr("path.name ~ pattern").unwrap();
        let regex2 = parse_expr("path.name ~= pattern").unwrap();
        let regex3 = parse_expr("path.name =~ pattern").unwrap();
        assert_eq!(regex1, regex2);
        assert_eq!(regex2, regex3);
    }

    // Error cases

    #[test]
    fn error_invalid_selector() {
        assert!(parse_expr("invalid == foo").is_err());
    }

    #[test]
    fn error_incomplete_expressions() {
        let incomplete = vec![
            "path.name ==",
            "path.name",
            "@",
            "== foo",
            "path.name == foo &&",
            "|| path.name == foo",
        ];
        for expr in incomplete {
            assert!(parse_expr(expr).is_err(), "Should fail: {}", expr);
        }
    }

    #[test]
    fn error_malformed_sets() {
        let malformed = vec![
            "path.extension in [js ts]", // missing comma
            "path.extension in js, ts]", // missing opening bracket
            "path.extension in [js, ts", // missing closing bracket
        ];
        for expr in malformed {
            assert!(parse_expr(expr).is_err(), "Should fail: {}", expr);
        }
    }

    #[test]
    fn error_mismatched_quotes() {
        let mismatched = vec![
            r#"path.name == "unclosed"#,
            r#"path.name == 'unclosed"#,
            r#"path.name == "mixed'"#,
        ];
        for expr in mismatched {
            assert!(parse_expr(expr).is_err(), "Should fail: {}", expr);
        }
    }

    #[test]
    fn error_type_mismatches() {
        // Size with non-numeric value
        assert!(parse_expr(r#"size > "large""#).is_err());

        // Invalid temporal format
        assert!(parse_expr(r#"modified > "not-a-date""#).is_err());

        // Invalid regex
        assert!(parse_expr(r#"path.name ~= "[unclosed""#).is_err());
    }

    #[test]
    fn test_empty_string_extension() {
        // Test parsing path.extension == ""
        let parsed = parse_expr(r#"path.extension == """#).unwrap();

        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::Equals("".to_string()),
        ))));

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_empty_string_in_set() {
        // Test parsing empty string in set literal
        let parsed = parse_expr(r#"path.extension in ["", txt, rs]"#).unwrap();

        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::In(
                ["".to_string(), "txt".to_string(), "rs".to_string()]
                    .into_iter()
                    .collect(),
            ),
        ))));

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_extension_matching_behavior() {
        use std::path::Path;

        // Test actual predicate matching behavior
        let pred_empty = NamePredicate::Extension(StringMatcher::Equals("".to_string()));
        let pred_txt = NamePredicate::Extension(StringMatcher::Equals("txt".to_string()));

        // File with no extension
        assert!(pred_empty.is_match(Path::new("README"))); // Now matches empty extension
        assert!(!pred_txt.is_match(Path::new("README")));

        // File with extension
        assert!(!pred_empty.is_match(Path::new("file.txt")));
        assert!(pred_txt.is_match(Path::new("file.txt")));

        // Hidden file with no extension
        assert!(pred_empty.is_match(Path::new(".gitignore"))); // Now matches empty extension
        assert!(!pred_txt.is_match(Path::new(".gitignore")));
    }

    #[test]
    fn test_in_operator_parsing() {
        // Test parsing of 'in' operator with bare identifiers
        let expr = parse_expr(r#"path.extension in [js, ts]"#).unwrap();

        // Check what we get
        if let Expr::Predicate(Predicate::Name(name_pred)) = &expr {
            if let NamePredicate::Extension(StringMatcher::In(values)) = name_pred.as_ref() {
                println!("Parsed values: {:?}", values);
                // The parser should produce a JSON array string that parse_string will decode
                assert_eq!(values.len(), 2);
                assert!(values.contains("js"));
                assert!(values.contains("ts"));
            } else {
                panic!("Expected In matcher, got: {:?}", name_pred);
            }
        } else {
            panic!("Expected Name predicate, got: {:?}", expr);
        }
    }

    #[test]
    fn test_in_operator_matching() {
        use std::path::Path;

        // Test actual matching with 'in' operator
        let pred = NamePredicate::Extension(StringMatcher::In(
            ["js".to_string(), "ts".to_string()].into_iter().collect(),
        ));

        assert!(pred.is_match(Path::new("file.js")));
        assert!(pred.is_match(Path::new("file.ts")));
        assert!(!pred.is_match(Path::new("file.rs")));
        assert!(!pred.is_match(Path::new("file.txt")));
    }

    #[test]
    fn test_name_in_operator_parsing() {
        // Test parsing name in [index, main] - the case used in failing integration test
        let expr = parse_expr(r#"path.name in [index, main]"#).unwrap();

        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::In(
                ["index".to_string(), "main".to_string()]
                    .into_iter()
                    .collect(),
            ),
        ))));

        assert_eq!(expr, expected);
    }

    #[test]
    fn test_compound_in_expression_parsing() {
        // Test the exact expression from the failing integration test
        let expr = parse_expr(r#"path.extension in [js, ts] && path.name in [index, main]"#).unwrap();

        let expected = Expr::And(
            Box::new(Expr::Predicate(Predicate::Name(Arc::new(
                NamePredicate::Extension(StringMatcher::In(
                    ["js".to_string(), "ts".to_string()].into_iter().collect(),
                )),
            )))),
            Box::new(Expr::Predicate(Predicate::Name(Arc::new(
                NamePredicate::FileName(StringMatcher::In(
                    ["index".to_string(), "main".to_string()]
                        .into_iter()
                        .collect(),
                )),
            )))),
        );

        assert_eq!(expr, expected);
    }

    #[test]
    fn test_filename_in_matching() {
        use std::path::Path;

        // Test name matching with 'in' operator
        let pred = NamePredicate::FileName(StringMatcher::In(
            ["index".to_string(), "main".to_string()]
                .into_iter()
                .collect(),
        ));

        // Only exact path.name matches should work with FileName
        assert!(!pred.is_match(Path::new("index.js"))); // "index" != "index.js"
        assert!(!pred.is_match(Path::new("main.ts"))); // "main" != "main.ts"

        // These exact matches should work
        assert!(pred.is_match(Path::new("index")));
        assert!(pred.is_match(Path::new("main")));

        // These should NOT match
        assert!(!pred.is_match(Path::new("app.js")));
        assert!(!pred.is_match(Path::new("test.ts")));
    }

    #[test]
    fn test_star_pattern_special_case() {
        // Test that * gets converted to .* for regex matching
        let expr = parse_expr(r#"path.name ~= "*""#).unwrap();

        // The expression should parse successfully
        if let Expr::Predicate(Predicate::Name(name_pred)) = expr {
            // Create a path to test against
            use std::path::Path;
            let test_path = Path::new("any_file_name.txt");

            // Should match any filename
            assert!(name_pred.is_match(test_path));
        } else {
            panic!("Expected name predicate");
        }

        // Also verify that plain * in regex context doesn't work without our special case
        use regex::Regex;
        assert!(
            Regex::new("*").is_err(),
            "* should not be a valid regex by itself"
        );
        assert!(Regex::new(".*").is_ok(), ".* should be a valid regex");
    }

    #[test]
    fn test_negation_operator_parsing() {
        // Test that negation operator produces correct AST
        let parsed = parse_expr(r#"!(path.name contains "test")"#).unwrap();

        let inner_pred = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Contains("test".to_string()),
        ))));
        let expected = Expr::Not(Box::new(inner_pred));

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_complex_negation_parsing() {
        // Test the exact expression from the beta tester's bug report
        let parsed = parse_expr(r#"path.extension == "rs" && !(path.name contains "test")"#).unwrap();

        let left = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::Equals("rs".to_string()),
        ))));
        let inner_pred = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Contains("test".to_string()),
        ))));
        let right = Expr::Not(Box::new(inner_pred));
        let expected = Expr::And(Box::new(left), Box::new(right));

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_negation_with_contains_in_compound() {
        // Test the exact problematic case: path.extension == "rs" && !(path.name contains "lib")
        let parsed = parse_expr(r#"path.extension == "rs" && !(path.name contains "lib")"#).unwrap();

        // Build expected AST
        let ext_pred = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::Equals("rs".to_string()),
        ))));

        let name_contains = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Contains("lib".to_string()),
        ))));
        let negated_name = Expr::Not(Box::new(name_contains));

        let expected = Expr::And(Box::new(ext_pred), Box::new(negated_name));

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_negation_without_parentheses() {
        // Test negation without parentheses: !path.name == "test"
        let parsed = parse_expr(r#"!path.name == "test""#).unwrap();

        let inner_pred = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("test".to_string()),
        ))));
        let expected = Expr::Not(Box::new(inner_pred));

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_escaped_quotes_in_strings() {
        // Test that parsing strings with escaped quotes currently fails
        // This is expected until the grammar is updated to support escape sequences
        let test_cases = vec![
            // Basic escaped double quote
            r#"contents contains "\"error\"" "#,
            // Multiple escaped quotes
            r#"contents contains "say \"hello\" to me" "#,
        ];

        for expr_str in test_cases {
            match parse_expr(expr_str) {
                Ok(_) => {
                    panic!(
                        "Expected parse error for '{}', but it parsed successfully",
                        expr_str
                    );
                }
                Err(e) => {
                    // Expected - the grammar doesn't support escape sequences yet
                    assert!(
                        matches!(e, ParseError::Syntax(_)),
                        "Expected syntax error for '{}', got: {:?}",
                        expr_str,
                        e
                    );
                }
            }
        }

        // Test that single quotes in double quoted strings work
        // (because they don't need escaping)
        let valid_expr = r#"contents contains "it's" "#;
        match parse_expr(valid_expr) {
            Ok(parsed) => {
                if let Expr::Predicate(Predicate::Content(content_pred)) = parsed {
                    let expected_regex = regex::escape("it's");
                    let expected_pred =
                        StreamingCompiledContentPredicate::new(expected_regex).unwrap();
                    assert_eq!(
                        content_pred, expected_pred,
                        "Parsed content doesn't match for expression: {}",
                        valid_expr
                    );
                } else {
                    panic!("Expected content predicate for: {}", valid_expr);
                }
            }
            Err(e) => {
                panic!("Failed to parse '{}': {}", valid_expr, e);
            }
        }
    }

    #[test]
    #[ignore] // FIXME: Grammar doesn't support extended escape sequences yet
    fn test_bare_token_escape_sequences() {
        let supported_escapes = vec![
            (r#"path.name == test\n"#, "test\n"),
            (r#"path.name == test\\"#, "test\\"),
        ];

        for (expr_str, expected_value) in supported_escapes {
            let parsed = parse_expr(expr_str).unwrap();
            assert_name_equals(&parsed, expected_value);
        }

        let unsupported_escapes = vec![
            r#"path.name ~= draft\.final\.final\.*"#,
            r#"path.name ~= \d+\.\d+"#,
            r#"path.name ~= test\$"#,
            r#"path.name ~= \^start"#,
            r#"path.name ~= foo\+bar"#,
            r#"path.name ~= test\?"#,
            r#"path.name ~= \(group\)"#,
            r#"path.name ~= \[abc\]"#,
            r#"path.name ~= a\{2,4\}"#,
            r#"path.name ~= one\|two"#,
        ];

        for expr_str in unsupported_escapes {
            assert_parse_error(expr_str);
        }
    }

    #[test]
    fn test_bare_token_escaped_regex_patterns() {
        let test_cases = vec![
            (
                r#"path.name ~= draft\.final\.final\.pptx"#,
                "draft.final.final.pptx",
                true,
            ),
            (
                r#"path.name ~= draft\.final\.final\.pptx"#,
                "draft-final-final-pptx",
                false,
            ),
            (r#"path.name ~= v\d+\.\d+\.\d+"#, "v1.2.3", true),
            (r#"path.name ~= v\d+\.\d+\.\d+"#, "v1-2-3", false),
            (r#"path.name ~= .*\.rs$"#, "main.rs", true),
            (r#"path.name ~= .*\.rs$"#, "main.rs.bak", false), // Should NOT match - path.name ends with .bak
            (
                r#"path.name ~= \[DRAFT\]\..*\.docx"#,
                "[DRAFT].report.docx",
                true,
            ),
            (
                r#"path.name ~= \[DRAFT\]\..*\.docx"#,
                "DRAFT.report.docx",
                false,
            ),
        ];

        for (expr_str, test_filename, should_match) in test_cases {
            match parse_expr(expr_str) {
                Ok(parsed) => {
                    verify_name_match(&parsed, test_filename, should_match);
                }
                Err(e) => {
                    panic!("Failed to parse '{}': {:?}", expr_str, e);
                }
            }
        }
    }

    #[test]
    #[ignore] // FIXME: Bare tokens need extended escape sequence support
    fn test_bare_token_vs_quoted_string_escapes() {
        let dot_pattern = r#"draft\.final\.final"#;

        let bare_expr = format!(r#"path.name contains {}"#, dot_pattern);
        assert_parse_error(&bare_expr);

        let quoted_expr = format!(r#"path.name contains "{}""#, dot_pattern);
        let parsed = parse_expr(&quoted_expr).unwrap();
        assert_name_contains(&parsed, "draft\\.final\\.final");
    }

    #[test]
    fn test_size_unit_parsing() {
        // Test various size units - all should be parsed successfully
        let test_cases = vec![
            // Kilobytes
            ("size > 1kb", 1025),
            ("size > 1KB", 1025),
            ("size > 1k", 1025),
            ("size > 1K", 1025),
            // Megabytes
            ("size > 2mb", 2 * 1024 * 1024 + 1),
            ("size > 2MB", 2 * 1024 * 1024 + 1),
            ("size > 2m", 2 * 1024 * 1024 + 1),
            ("size > 2M", 2 * 1024 * 1024 + 1),
            // Gigabytes
            ("size > 3gb", 3 * 1024 * 1024 * 1024 + 1),
            ("size > 3GB", 3 * 1024 * 1024 * 1024 + 1),
            ("size > 3g", 3 * 1024 * 1024 * 1024 + 1),
            ("size > 3G", 3 * 1024 * 1024 * 1024 + 1),
            // Terabytes
            ("size > 1tb", 1024u64 * 1024 * 1024 * 1024 + 1),
            ("size > 1TB", 1024u64 * 1024 * 1024 * 1024 + 1),
            ("size > 1t", 1024u64 * 1024 * 1024 * 1024 + 1),
            ("size > 1T", 1024u64 * 1024 * 1024 * 1024 + 1),
        ];

        for (expr_str, expected_bytes) in test_cases {
            let parsed =
                parse_expr(expr_str).unwrap_or_else(|_| panic!("Failed to parse: {}", expr_str));
            if let Expr::Predicate(Predicate::Metadata(meta_pred)) = parsed {
                if let MetadataPredicate::Filesize(NumberMatcher::In(Bound::Left(range))) =
                    meta_pred.as_ref()
                {
                    assert_eq!(
                        range.start, expected_bytes,
                        "Wrong byte value for '{}': expected {}, got {}",
                        expr_str, expected_bytes, range.start
                    );
                    continue;
                }
            }
            panic!("Expected size > predicate for: {}", expr_str);
        }
    }

    #[test]
    fn test_size_decimal_parsing() {
        // Test decimal size values
        let test_cases = vec![
            ("size > 2.5mb", (2.5 * 1024.0 * 1024.0) as u64 + 1),
            ("size > 1.5gb", (1.5 * 1024.0 * 1024.0 * 1024.0) as u64 + 1),
            ("size > 0.5kb", (0.5 * 1024.0) as u64 + 1),
        ];

        for (expr_str, expected_bytes) in test_cases {
            let parsed =
                parse_expr(expr_str).unwrap_or_else(|_| panic!("Failed to parse: {}", expr_str));
            if let Expr::Predicate(Predicate::Metadata(meta_pred)) = parsed {
                if let MetadataPredicate::Filesize(NumberMatcher::In(Bound::Left(range))) =
                    meta_pred.as_ref()
                {
                    assert_eq!(
                        range.start, expected_bytes,
                        "Wrong byte value for '{}': expected {}, got {}",
                        expr_str, expected_bytes, range.start
                    );
                    continue;
                }
            }
            panic!("Expected size > predicate for: {}", expr_str);
        }
    }

    #[test]
    fn test_size_with_different_operators() {
        // Test size with various operators
        let test_cases = vec![
            ("size == 1mb", NumberMatcher::Equals(1024 * 1024)),
            (
                "size != 2gb",
                NumberMatcher::NotEquals(2 * 1024 * 1024 * 1024),
            ),
            (
                "size < 500kb",
                NumberMatcher::In(Bound::Right(..(500 * 1024))),
            ),
            (
                "size <= 1gb",
                NumberMatcher::In(Bound::Right(..(1024 * 1024 * 1024 + 1))),
            ),
            (
                "size >= 100mb",
                NumberMatcher::In(Bound::Left((100 * 1024 * 1024)..)),
            ),
        ];

        for (expr_str, expected_matcher) in test_cases {
            let parsed =
                parse_expr(expr_str).unwrap_or_else(|_| panic!("Failed to parse: {}", expr_str));
            if let Expr::Predicate(Predicate::Metadata(meta_pred)) = parsed {
                if let MetadataPredicate::Filesize(matcher) = meta_pred.as_ref() {
                    assert_eq!(
                        matcher, &expected_matcher,
                        "Wrong matcher for '{}'",
                        expr_str
                    );
                    continue;
                }
            }
            panic!("Expected size predicate for: {}", expr_str);
        }
    }

    #[test]
    fn test_mixed_size_and_number_parsing() {
        // Test that both plain numbers and size units work
        let plain_number = parse_expr("size > 1000").unwrap();
        let size_unit = parse_expr("size > 1kb").unwrap();

        // Both should parse successfully
        assert!(matches!(
            plain_number,
            Expr::Predicate(Predicate::Metadata(_))
        ));
        assert!(matches!(size_unit, Expr::Predicate(Predicate::Metadata(_))));

        // 1kb should be greater than 1000
        if let (
            Expr::Predicate(Predicate::Metadata(plain_pred)),
            Expr::Predicate(Predicate::Metadata(size_pred)),
        ) = (plain_number, size_unit)
        {
            if let (
                MetadataPredicate::Filesize(NumberMatcher::In(Bound::Left(plain_range))),
                MetadataPredicate::Filesize(NumberMatcher::In(Bound::Left(size_range))),
            ) = (plain_pred.as_ref(), size_pred.as_ref())
            {
                assert_eq!(plain_range.start, 1001);
                assert_eq!(size_range.start, 1025);
                assert!(size_range.start > plain_range.start);
                return;
            }
        }
        panic!("Expected size predicates with ranges");
    }

    // Helper functions

    fn assert_name_equals(
        expr: &Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
        expected: &str,
    ) {
        if let Expr::Predicate(Predicate::Name(name_pred)) = expr {
            if let NamePredicate::FileName(StringMatcher::Equals(val)) = name_pred.as_ref() {
                assert_eq!(val, expected);
                return;
            }
        }
        panic!("Expected Name Equals predicate");
    }

    fn assert_name_contains(
        expr: &Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
        expected: &str,
    ) {
        if let Expr::Predicate(Predicate::Name(name_pred)) = expr {
            if let NamePredicate::FileName(StringMatcher::Contains(val)) = name_pred.as_ref() {
                assert_eq!(val, expected);
                return;
            }
        }
        panic!("Expected Name Contains predicate");
    }

    fn assert_parse_error(expr_str: &str) {
        match parse_expr(expr_str) {
            Ok(_) => panic!("Expected parse error for '{}'", expr_str),
            Err(e) => assert!(matches!(e, ParseError::Syntax(_))),
        }
    }

    fn verify_name_match(
        expr: &Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
        filename: &str,
        should_match: bool,
    ) {
        if let Expr::Predicate(Predicate::Name(name_pred)) = expr {
            use std::path::Path;
            let test_path = Path::new(filename);
            let matches = name_pred.is_match(test_path);
            assert_eq!(
                matches, should_match,
                "Pattern match failed for '{}'",
                filename
            );
        } else {
            panic!("Expected Name predicate");
        }
    }

    #[test]
    fn test_parentheses_parsing() {
        // Test parentheses support in expressions
        let test_cases = vec![
            // Simple parentheses
            ("(path.name contains test)", true),
            // Parentheses with OR
            ("(path.name contains spec || path.name contains test)", true),
            // Complex expression from bug report
            ("path.parent contains multivendor-plugin && (path.name contains spec || path.name contains test)", true),
            // Nested parentheses
            ("((path.name contains a || path.name contains b) && path.extension == rs)", true),
            // Multiple groups
            ("(path.name contains foo) && (size > 1000 || path.extension == txt)", true),
        ];

        for (expr, should_succeed) in test_cases {
            let result = parse_expr(expr);
            if should_succeed && result.is_err() {
                eprintln!("Failed to parse: {}", expr);
                eprintln!("Error: {:?}", result);
            }
            assert_eq!(result.is_ok(), should_succeed, "Failed for: {}", expr);
        }
    }

    #[test]
    fn test_path_selectors() {
        // Test simple path selector maps to FullPath (alias)
        let parsed = parse_expr(r#"path == "src/lib.rs""#).unwrap();
        if let Expr::Predicate(Predicate::Name(name_pred)) = parsed {
            match name_pred.as_ref() {
                NamePredicate::FullPath(_) => (),
                _ => panic!("Expected FullPath predicate, got {:?}", name_pred),
            }
        } else {
            panic!("Expected name predicate");
        }

        // Test path.full canonical form
        let parsed = parse_expr(r#"path.full == "src/lib.rs""#).unwrap();
        if let Expr::Predicate(Predicate::Name(name_pred)) = parsed {
            match name_pred.as_ref() {
                NamePredicate::FullPath(_) => (),
                _ => panic!("Expected FullPath predicate, got {:?}", name_pred),
            }
        } else {
            panic!("Expected name predicate");
        }

        // Test path contains (alias)
        let parsed = parse_expr(r#"path contains "src""#).unwrap();
        if let Expr::Predicate(Predicate::Name(name_pred)) = parsed {
            match name_pred.as_ref() {
                NamePredicate::FullPath(StringMatcher::Contains(s)) => assert_eq!(s, "src"),
                _ => panic!("Expected FullPath contains predicate"),
            }
        } else {
            panic!("Expected name predicate");
        }

        // Test path.name maps to FileName
        let parsed = parse_expr(r#"path.name == "lib.rs""#).unwrap();
        if let Expr::Predicate(Predicate::Name(name_pred)) = parsed {
            match name_pred.as_ref() {
                NamePredicate::FileName(StringMatcher::Equals(s)) => assert_eq!(s, "lib.rs"),
                _ => panic!("Expected FileName predicate, got {:?}", name_pred),
            }
        } else {
            panic!("Expected name predicate");
        }

        // Test path.parent maps to DirPath
        let parsed = parse_expr(r#"path.parent contains "src""#).unwrap();
        if let Expr::Predicate(Predicate::Name(name_pred)) = parsed {
            match name_pred.as_ref() {
                NamePredicate::DirPath(StringMatcher::Contains(s)) => assert_eq!(s, "src"),
                _ => panic!("Expected DirPath predicate, got {:?}", name_pred),
            }
        } else {
            panic!("Expected name predicate");
        }

        // Test path.stem maps to BaseName
        let parsed = parse_expr(r#"path.stem == "lib""#).unwrap();
        if let Expr::Predicate(Predicate::Name(name_pred)) = parsed {
            match name_pred.as_ref() {
                NamePredicate::BaseName(StringMatcher::Equals(s)) => assert_eq!(s, "lib"),
                _ => panic!("Expected BaseName predicate, got {:?}", name_pred),
            }
        } else {
            panic!("Expected name predicate");
        }

        // Test path.extension maps to Extension (without dot)
        let parsed = parse_expr(r#"path.extension == "rs""#).unwrap();
        if let Expr::Predicate(Predicate::Name(name_pred)) = parsed {
            match name_pred.as_ref() {
                NamePredicate::Extension(StringMatcher::Equals(s)) => assert_eq!(s, "rs"),
                _ => panic!("Expected Extension predicate, got {:?}", name_pred),
            }
        } else {
            panic!("Expected name predicate");
        }

        // Test path.extension regex matching (without dots)
        let parsed = parse_expr(r#"path.extension ~= "(rs|toml)""#).unwrap();
        if let Expr::Predicate(Predicate::Name(name_pred)) = parsed {
            match name_pred.as_ref() {
                NamePredicate::Extension(StringMatcher::Regex(_)) => (),
                _ => panic!("Expected Extension regex predicate"),
            }
        } else {
            panic!("Expected name predicate");
        }

        // Test complex path queries
        let parsed = parse_expr(r#"path.parent contains "src" && path.extension == ".rs""#).unwrap();
        if let Expr::And(left, right) = parsed {
            if let Expr::Predicate(Predicate::Name(left_pred)) = left.as_ref() {
                assert!(matches!(left_pred.as_ref(), NamePredicate::DirPath(_)));
            }
            if let Expr::Predicate(Predicate::Name(right_pred)) = right.as_ref() {
                assert!(matches!(right_pred.as_ref(), NamePredicate::Extension(_)));
            }
        } else {
            panic!("Expected And expression");
        }
    }

    #[test]
    fn test_path_selector_evaluation() {
        use std::path::Path;

        // Test actual evaluation of path selectors
        let test_path = Path::new("src/parser_tests.rs");

        // Helper to verify name matching
        fn verify_name_match(
            expr: &Expr<
                Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>,
            >,
            path: &Path,
            expected: bool,
        ) {
            if let Expr::Predicate(Predicate::Name(name_pred)) = expr {
                assert_eq!(name_pred.is_match(path), expected);
            } else {
                panic!("Expected name predicate");
            }
        }

        // Test path (full path)
        let expr = parse_expr(r#"path contains "src/parser""#).unwrap();
        verify_name_match(&expr, test_path, true);

        // Test path.name
        let expr = parse_expr(r#"path.name == "parser_tests.rs""#).unwrap();
        verify_name_match(&expr, test_path, true);

        // Test path.parent
        let expr = parse_expr(r#"path.parent == "src""#).unwrap();
        verify_name_match(&expr, test_path, true);

        // Test path.stem
        let expr = parse_expr(r#"path.stem == "parser_tests""#).unwrap();
        verify_name_match(&expr, test_path, true);

        // Test path.extension (note: extension is stored without dot internally)
        let expr = parse_expr(r#"path.extension == "rs""#).unwrap();
        verify_name_match(&expr, test_path, true);

        // Test negative cases
        let expr = parse_expr(r#"path.name == "wrong.rs""#).unwrap();
        verify_name_match(&expr, test_path, false);
    }
}
