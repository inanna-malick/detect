#[cfg(test)]
mod tests {
    use crate::parser::parse_expr;
    use crate::expr::Expr;
    use crate::predicate::{
        Predicate, NamePredicate, MetadataPredicate, StringMatcher, NumberMatcher,
        StreamingCompiledContentPredicate, Bound
    };
    use std::sync::Arc;

    // Test basic parsing produces expected compiled predicates

    #[test]
    fn parse_name_equality() {
        let parsed = parse_expr("@name == foo").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::Equals("foo".to_string()))
        )));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_name_not_equal() {
        let parsed = parse_expr("@name != bar").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::NotEquals("bar".to_string()))
        )));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_name_regex() {
        let parsed = parse_expr("@name ~= test.*").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::regex("test.*").unwrap())
        )));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_name_contains() {
        let parsed = parse_expr(r#"@name contains "test""#).unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::Contains("test".to_string()))
        )));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_name_glob() {
        let parsed = parse_expr(r#"@name glob "*.rs""#).unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::Glob("*.rs".to_string()))
        )));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_name_in_set() {
        let parsed = parse_expr("@name in [foo, bar, baz]").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::In(vec![
                "foo".to_string(),
                "bar".to_string(),
                "baz".to_string(),
            ]))
        )));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_path_predicate() {
        let parsed = parse_expr(r#"@path == "src/main.rs""#).unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Path(StringMatcher::Equals("src/main.rs".to_string()))
        )));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_extension_predicate() {
        let parsed = parse_expr("@ext == rs").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Extension(StringMatcher::Equals("rs".to_string()))
        )));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_extension_in_set() {
        let parsed = parse_expr("@ext in [js, ts, jsx, tsx]").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Extension(StringMatcher::In(vec![
                "js".to_string(),
                "ts".to_string(),
                "jsx".to_string(),
                "tsx".to_string(),
            ]))
        )));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_size_comparisons() {
        let cases = vec![
            ("@size == 100", NumberMatcher::Equals(100)),
            ("@size > 100", NumberMatcher::In(Bound::Left(100..))),
            ("@size >= 100", NumberMatcher::In(Bound::Left(99..))),
            ("@size < 100", NumberMatcher::In(Bound::Right(..101))),
            ("@size <= 100", NumberMatcher::In(Bound::Right(..100))),
        ];

        for (expr_str, expected_matcher) in cases {
            let parsed = parse_expr(expr_str).unwrap();
            let expected = Expr::Predicate(Predicate::Metadata(Arc::new(
                MetadataPredicate::Filesize(expected_matcher)
            )));
            assert_eq!(parsed, expected, "Failed for: {}", expr_str);
        }
    }

    #[test]
    fn parse_type_predicate() {
        let parsed = parse_expr("@type == dir").unwrap();
        let expected = Expr::Predicate(Predicate::Metadata(Arc::new(
            MetadataPredicate::Type(StringMatcher::Equals("dir".to_string()))
        )));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_content_regex() {
        let parsed = parse_expr(r#"@contents ~= "TODO|FIXME""#).unwrap();
        let expected = Expr::Predicate(Predicate::Content(
            StreamingCompiledContentPredicate::new("TODO|FIXME".to_string()).unwrap()
        ));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_content_contains() {
        let parsed = parse_expr(r#"@file contains "fn main""#).unwrap();
        // contains gets compiled to an escaped regex
        let expected = Expr::Predicate(Predicate::Content(
            StreamingCompiledContentPredicate::new(regex::escape("fn main")).unwrap()
        ));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_temporal_selectors() {
        // Just verify they parse to the correct variant
        let modified = parse_expr(r#"@modified > "-7.days""#).unwrap();
        assert!(matches!(
            modified,
            Expr::Predicate(Predicate::Metadata(ref meta)) 
                if matches!(**meta, MetadataPredicate::Modified(_))
        ));

        let created = parse_expr(r#"@created >= "today""#).unwrap();
        assert!(matches!(
            created,
            Expr::Predicate(Predicate::Metadata(ref meta))
                if matches!(**meta, MetadataPredicate::Created(_))
        ));

        let accessed = parse_expr(r#"@accessed < "2024-01-01""#).unwrap();
        assert!(matches!(
            accessed,
            Expr::Predicate(Predicate::Metadata(ref meta))
                if matches!(**meta, MetadataPredicate::Accessed(_))
        ));
    }

    // Test boolean operators

    #[test]
    fn parse_and_expression() {
        let parsed = parse_expr("@name == foo && @ext == rs").unwrap();
        let left = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::Equals("foo".to_string()))
        )));
        let right = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Extension(StringMatcher::Equals("rs".to_string()))
        )));
        let expected = Expr::and(left, right);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_or_expression() {
        let parsed = parse_expr("@name == foo || @name == bar").unwrap();
        let left = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::Equals("foo".to_string()))
        )));
        let right = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::Equals("bar".to_string()))
        )));
        let expected = Expr::or(left, right);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_not_expression() {
        let parsed = parse_expr("!@name == temp").unwrap();
        let inner = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::Equals("temp".to_string()))
        )));
        let expected = Expr::negate(inner);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_operator_precedence() {
        // @a == x || @b == y && @c == z
        // Should parse as: @a == x || (@b == y && @c == z)
        let parsed = parse_expr("@name == a || @name == b && @name == c").unwrap();
        
        let a = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::Equals("a".to_string()))
        )));
        let b = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::Equals("b".to_string()))
        )));
        let c = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::Equals("c".to_string()))
        )));
        
        let b_and_c = Expr::and(b, c);
        let expected = Expr::or(a, b_and_c);
        
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_parentheses_override() {
        // (@a == x || @b == y) && @c == z
        let parsed = parse_expr("(@name == a || @name == b) && @name == c").unwrap();
        
        let a = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::Equals("a".to_string()))
        )));
        let b = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::Equals("b".to_string()))
        )));
        let c = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::Equals("c".to_string()))
        )));
        
        let a_or_b = Expr::or(a, b);
        let expected = Expr::and(a_or_b, c);
        
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_complex_nested() {
        // !(@a == x || @b == y) && (@c == z || @d == w)
        let parsed = parse_expr("!(@name == x || @ext == y) && (@size > 100 || @type == dir)").unwrap();
        
        // Build expected tree
        let x = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::Equals("x".to_string()))
        )));
        let y = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Extension(StringMatcher::Equals("y".to_string()))
        )));
        let size_check = Expr::Predicate(Predicate::Metadata(Arc::new(
            MetadataPredicate::Filesize(NumberMatcher::In(Bound::Left(100..)))
        )));
        let type_check = Expr::Predicate(Predicate::Metadata(Arc::new(
            MetadataPredicate::Type(StringMatcher::Equals("dir".to_string()))
        )));
        
        let x_or_y = Expr::or(x, y);
        let not_x_or_y = Expr::negate(x_or_y);
        let size_or_type = Expr::or(size_check, type_check);
        let expected = Expr::and(not_x_or_y, size_or_type);
        
        assert_eq!(parsed, expected);
    }

    // Test special features

    #[test]
    fn parse_quoted_values() {
        let double_quoted = parse_expr(r#"@name == "my file.txt""#).unwrap();
        let single_quoted = parse_expr(r"@name == 'my file.txt'").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::Equals("my file.txt".to_string()))
        )));
        assert_eq!(double_quoted, expected);
        assert_eq!(single_quoted, expected);
    }

    #[test]
    fn parse_set_literal_variations() {
        // Empty set - not supported by grammar
        assert!(parse_expr("@ext in []").is_err());

        // Single item
        let single = parse_expr("@ext in [js]").unwrap();
        let expected_single = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Extension(StringMatcher::In(vec!["js".to_string()]))
        )));
        assert_eq!(single, expected_single);

        // Mixed quoted and unquoted
        let mixed = parse_expr(r#"@name in [foo, "bar baz", 'qux']"#).unwrap();
        let expected_mixed = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::In(vec![
                "foo".to_string(),
                "bar baz".to_string(),
                "qux".to_string(),
            ]))
        )));
        assert_eq!(mixed, expected_mixed);
    }

    #[test]
    fn parse_selector_aliases() {
        // Test that aliases produce identical results
        let name_variants = vec![
            ("@name == test", "@filename == test"),
            ("@path == foo", "@filepath == foo"),
            ("@ext == rs", "@extension == rs"),
            ("@size > 100", "@filesize > 100"),
            ("@type == dir", "@filetype == dir"),
            ("@contents ~= foo", "@file ~= foo"),
            ("@modified > today", "@mtime > today"),
            ("@created > today", "@ctime > today"),
            ("@accessed > today", "@atime > today"),
        ];

        for (expr1_str, expr2_str) in name_variants {
            let expr1 = parse_expr(expr1_str).unwrap();
            let expr2 = parse_expr(expr2_str).unwrap();
            assert_eq!(expr1, expr2, "{} should equal {}", expr1_str, expr2_str);
        }
    }

    #[test]
    fn parse_operator_aliases() {
        // = vs ==
        let eq1 = parse_expr("@name = foo").unwrap();
        let eq2 = parse_expr("@name == foo").unwrap();
        assert_eq!(eq1, eq2);

        // ~ vs ~= vs =~
        let regex1 = parse_expr("@name ~ pattern").unwrap();
        let regex2 = parse_expr("@name ~= pattern").unwrap();
        let regex3 = parse_expr("@name =~ pattern").unwrap();
        assert_eq!(regex1, regex2);
        assert_eq!(regex2, regex3);
    }

    // Error cases

    #[test]
    fn error_invalid_selector() {
        assert!(parse_expr("@invalid == foo").is_err());
    }

    #[test]
    fn error_incomplete_expressions() {
        let incomplete = vec![
            "@name ==",
            "@name",
            "@",
            "== foo",
            "@name == foo &&",
            "|| @name == foo",
        ];
        for expr in incomplete {
            assert!(parse_expr(expr).is_err(), "Should fail: {}", expr);
        }
    }

    #[test]
    fn error_malformed_sets() {
        let malformed = vec![
            "@ext in [js ts]",      // missing comma
            "@ext in [js,]",        // trailing comma
            "@ext in [,js]",        // leading comma
            "@ext in js, ts]",      // missing opening bracket
            "@ext in [js, ts",      // missing closing bracket
        ];
        for expr in malformed {
            assert!(parse_expr(expr).is_err(), "Should fail: {}", expr);
        }
    }

    #[test]
    fn error_mismatched_quotes() {
        let mismatched = vec![
            r#"@name == "unclosed"#,
            r#"@name == 'unclosed"#,
            r#"@name == "mixed'"#,
        ];
        for expr in mismatched {
            assert!(parse_expr(expr).is_err(), "Should fail: {}", expr);
        }
    }

    #[test]
    fn error_type_mismatches() {
        // Size with non-numeric value
        assert!(parse_expr(r#"@size > "large""#).is_err());
        
        // Invalid temporal format
        assert!(parse_expr(r#"@modified > "not-a-date""#).is_err());
        
        // Invalid regex
        assert!(parse_expr(r#"@name ~= "[unclosed""#).is_err());
    }

    #[test]
    fn test_empty_string_extension() {
        // Test parsing @ext == ""
        let parsed = parse_expr(r#"@ext == """#).unwrap();
        
        let expected = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Extension(StringMatcher::Equals("".to_string()))
        )));
        
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_empty_string_in_set() {
        // Test parsing empty string in set literal
        let parsed = parse_expr(r#"@ext in ["", txt, rs]"#).unwrap();
        
        let expected = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Extension(StringMatcher::In(vec![
                "".to_string(),
                "txt".to_string(),
                "rs".to_string(),
            ]))
        )));
        
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_extension_matching_behavior() {
        use std::path::Path;
        
        // Test actual predicate matching behavior
        let pred_empty = NamePredicate::Extension(StringMatcher::Equals("".to_string()));
        let pred_txt = NamePredicate::Extension(StringMatcher::Equals("txt".to_string()));
        
        // File with no extension
        assert!(!pred_empty.is_match(Path::new("README")));  // This will be false!
        assert!(!pred_txt.is_match(Path::new("README")));
        
        // File with extension
        assert!(!pred_empty.is_match(Path::new("file.txt")));
        assert!(pred_txt.is_match(Path::new("file.txt")));
        
        // Hidden file with no extension
        assert!(!pred_empty.is_match(Path::new(".gitignore")));
        assert!(!pred_txt.is_match(Path::new(".gitignore")));
    }

    #[test]
    fn test_in_operator_parsing() {
        // Test parsing of 'in' operator with bare identifiers
        let expr = parse_expr(r#"@ext in [js, ts]"#).unwrap();
        
        // Check what we get
        if let Expr::Predicate(Predicate::Name(name_pred)) = &expr {
            if let NamePredicate::Extension(StringMatcher::In(values)) = name_pred.as_ref() {
                println!("Parsed values: {:?}", values);
                // The parser should produce a JSON array string that parse_string will decode
                assert_eq!(values.len(), 2);
                assert_eq!(values[0], "js");
                assert_eq!(values[1], "ts");
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
        let pred = NamePredicate::Extension(StringMatcher::In(vec![
            "js".to_string(),
            "ts".to_string(),
        ]));
        
        assert!(pred.is_match(Path::new("file.js")));
        assert!(pred.is_match(Path::new("file.ts")));
        assert!(!pred.is_match(Path::new("file.rs")));
        assert!(!pred.is_match(Path::new("file.txt")));
    }

    #[test]
    fn test_name_in_operator_parsing() {
        // Test parsing @name in [index, main] - the case used in failing integration test
        let expr = parse_expr(r#"@name in [index, main]"#).unwrap();
        
        let expected = Expr::Predicate(Predicate::Name(Arc::new(
            NamePredicate::Filename(StringMatcher::In(vec![
                "index".to_string(),
                "main".to_string(),
            ]))
        )));
        
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_compound_in_expression_parsing() {
        // Test the exact expression from the failing integration test
        let expr = parse_expr(r#"@ext in [js, ts] && @name in [index, main]"#).unwrap();
        
        let expected = Expr::And(
            Box::new(Expr::Predicate(Predicate::Name(Arc::new(
                NamePredicate::Extension(StringMatcher::In(vec![
                    "js".to_string(),
                    "ts".to_string(),
                ]))
            )))),
            Box::new(Expr::Predicate(Predicate::Name(Arc::new(
                NamePredicate::Filename(StringMatcher::In(vec![
                    "index".to_string(),
                    "main".to_string(),
                ]))
            ))))
        );
        
        assert_eq!(expr, expected);
    }

    #[test]
    fn test_filename_in_matching() {
        use std::path::Path;
        
        // Test @name matching with 'in' operator
        let pred = NamePredicate::Filename(StringMatcher::In(vec![
            "index".to_string(),
            "main".to_string(),
        ]));
        
        // These should now match - checks both full name and stem
        assert!(pred.is_match(Path::new("index.js")));
        assert!(pred.is_match(Path::new("main.ts")));
        
        // These should also match
        assert!(pred.is_match(Path::new("index")));
        assert!(pred.is_match(Path::new("main")));
        
        // These should NOT match
        assert!(!pred.is_match(Path::new("app.js")));
        assert!(!pred.is_match(Path::new("test.ts")));
    }

    #[test]
    fn test_star_pattern_special_case() {
        // Test that * gets converted to .* for regex matching
        let expr = parse_expr(r#"@name ~= "*""#).unwrap();
        
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
        assert!(Regex::new("*").is_err(), "* should not be a valid regex by itself");
        assert!(Regex::new(".*").is_ok(), ".* should be a valid regex");
    }
}