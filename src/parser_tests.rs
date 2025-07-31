#[cfg(test)]
mod tests {
    use crate::expr::Expr;
    use crate::parse_error::ParseError;
    use crate::parser::parse_expr;
    use crate::predicate::{
        Bound, MetadataPredicate, NamePredicate, NumberMatcher, Predicate,
        StreamingCompiledContentPredicate, StringMatcher,
    };
    use std::sync::Arc;

    // Test basic parsing produces expected compiled predicates

    #[test]
    fn parse_name_equality() {
        let parsed = parse_expr("filename == foo").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("foo".to_string()),
        ))));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_name_not_equal() {
        let parsed = parse_expr("filename != bar").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::NotEquals("bar".to_string()),
        ))));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_name_regex() {
        let parsed = parse_expr("filename ~= test.*").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::regex("test.*").unwrap(),
        ))));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_regex_with_special_chars() {
        // Test regex with curly braces, parentheses, etc.
        let cases = vec![
            r"filename ~= ^[0-9]{10,13}.*\.ts$",
            r#"filename ~= "(foo|bar)""#,
            r"filename ~= test\?.*",
            r"contents ~= TODO.*\{.*\}",
        ];
        
        for expr in cases {
            let result = parse_expr(expr);
            assert!(result.is_ok(), "Failed to parse: {}", expr);
        }
    }

    #[test]
    fn parse_name_contains() {
        let parsed = parse_expr(r#"filename contains "test""#).unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Contains("test".to_string()),
        ))));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_name_in_set() {
        // Test case that appears to be failing
        let result = parse_expr("filename in [foo, bar, baz]");
        
        // Let's see what the actual error is
        match result {
            Ok(parsed) => {
                let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
                    StringMatcher::In(vec![
                        "foo".to_string(),
                        "bar".to_string(),
                        "baz".to_string(),
                    ]),
                ))));
                assert_eq!(parsed, expected);
            }
            Err(e) => {
                panic!("Failed to parse 'filename in [foo, bar, baz]': {:?}", e);
            }
        }
    }

    #[test]
    fn debug_set_parsing() {
        // Test different variations to understand the parsing issue
        
        // Try without spaces
        let result1 = parse_expr("filename in [foo,bar,baz]");
        println!("Without spaces: {:?}", result1.is_ok());
        
        // Try with quoted strings
        let result2 = parse_expr(r#"filename in ["foo", "bar", "baz"]"#);
        println!("With quotes: {:?}", result2.is_ok());
        
        // Try mixed
        let result3 = parse_expr(r#"filename in [foo, "bar", baz]"#);
        println!("Mixed: {:?}", result3.is_ok());
        
        // Try the original that's failing
        let result4 = parse_expr("filename in [foo, bar, baz]");
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
        let working = parse_expr("filename in [foo,bar,baz]");
        assert!(working.is_ok(), "Should parse without spaces");
        
        let also_working = parse_expr(r#"filename in ["foo","bar","baz"]"#);
        assert!(also_working.is_ok(), "Should parse with quotes and no spaces");
        
        let also_working2 = parse_expr(r#"filename in ["foo", "bar", "baz"]"#);
        assert!(also_working2.is_ok(), "Should parse with quotes and spaces");
        
        // This fails due to parser bug
        let failing = parse_expr("filename in [foo, bar, baz]");
        assert!(failing.is_ok(), "Should parse with spaces - but doesn't due to parser bug: {:?}", failing);
    }

    #[test]
    fn parse_path_predicate() {
        let parsed = parse_expr(r#"fullpath == "src/main.rs""#).unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FullPath(
            StringMatcher::Equals("src/main.rs".to_string()),
        ))));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_extension_predicate() {
        let parsed = parse_expr("ext == rs").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::Equals("rs".to_string()),
        ))));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_extension_in_set() {
        let parsed = parse_expr("ext in [js, ts, jsx, tsx]").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::In(vec![
                "js".to_string(),
                "ts".to_string(),
                "jsx".to_string(),
                "tsx".to_string(),
            ]),
        ))));
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_size_comparisons() {
        let cases = vec![
            ("size == 100", NumberMatcher::Equals(100)),
            ("size > 100", NumberMatcher::In(Bound::Left(100..))),
            ("size >= 100", NumberMatcher::In(Bound::Left(99..))),
            ("size < 100", NumberMatcher::In(Bound::Right(..101))),
            ("size <= 100", NumberMatcher::In(Bound::Right(..100))),
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
        let parsed = parse_expr("filename == foo && ext == rs").unwrap();
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
        let parsed = parse_expr("filename == foo || filename == bar").unwrap();
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
        let parsed = parse_expr("!filename == temp").unwrap();
        let inner = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("temp".to_string()),
        ))));
        let expected = Expr::negate(inner);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parse_operator_precedence() {
        // filename == x || filename == y && filename == z
        // Should parse as: filename == x || (filename == y && filename == z)
        let parsed = parse_expr("filename == a || filename == b && filename == c").unwrap();

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
        // (filename == x || filename == y) && filename == z
        let parsed = parse_expr("(filename == a || filename == b) && filename == c").unwrap();

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
        // !(filename == x || ext == y) && (size > z || type == w)
        let parsed = parse_expr("!(filename == x || ext == y) && (size > 100 || type == dir)").unwrap();

        // Build expected tree
        let x = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("x".to_string()),
        ))));
        let y = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::Equals("y".to_string()),
        ))));
        let size_check = Expr::Predicate(Predicate::Metadata(Arc::new(
            MetadataPredicate::Filesize(NumberMatcher::In(Bound::Left(100..))),
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
        let double_quoted = parse_expr(r#"filename == "my file.txt""#).unwrap();
        let single_quoted = parse_expr(r"filename == 'my file.txt'").unwrap();
        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Equals("my file.txt".to_string()),
        ))));
        assert_eq!(double_quoted, expected);
        assert_eq!(single_quoted, expected);
    }

    #[test]
    fn parse_set_literal_variations() {
        let empty = parse_expr("ext in []").unwrap();
        let expected_empty = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::In(vec![]),
        ))));
        assert_eq!(empty, expected_empty);

        // Single item
        let single = parse_expr("ext in [js]").unwrap();
        let expected_single = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::In(vec!["js".to_string()]),
        ))));
        assert_eq!(single, expected_single);

        // Mixed quoted and unquoted
        let mixed = parse_expr(r#"filename in [foo, "bar baz", 'qux']"#).unwrap();
        let expected_mixed = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::In(vec![
                "foo".to_string(),
                "bar baz".to_string(),
                "qux".to_string(),
            ]),
        ))));
        assert_eq!(mixed, expected_mixed);
    }

    #[test]
    fn parse_selector_aliases() {
        // Test that aliases produce identical results
        let name_variants = vec![
            ("filename == test", "file == test"),
            ("fullpath == foo", "full == foo"),
            ("ext == rs", "extension == rs"),
            ("size > 100kb", "filesize > 100kb"),
            ("type == dir", "filetype == dir"),
            ("contents ~= foo", "content ~= foo"),
            ("modified > today", "mtime > today"),
            ("created > today", "ctime > today"),
            ("accessed > today", "atime > today"),
        ];

        for (expr1_str, expr2_str) in name_variants {
            let expr1 = parse_expr(expr1_str).unwrap_or_else(|e| panic!("Failed to parse '{}': {:?}", expr1_str, e));
            let expr2 = parse_expr(expr2_str).unwrap_or_else(|e| panic!("Failed to parse '{}': {:?}", expr2_str, e));
            assert_eq!(expr1, expr2, "{} should equal {}", expr1_str, expr2_str);
        }
    }

    #[test]
    fn test_size_comparison_parsing() {
        // Size comparisons can use units or bare numbers
        // Bare numbers are interpreted as bytes
        assert!(parse_expr("size > 100").is_ok(), "Size with bare number should work (bytes)");
        assert!(parse_expr("size > 100kb").is_ok(), "Size with kb unit should work");
        assert!(parse_expr("size > 100mb").is_ok(), "Size with mb unit should work");
        assert!(parse_expr("size > 100gb").is_ok(), "Size with gb unit should work");
        assert!(parse_expr("size > 100tb").is_ok(), "Size with tb unit should work");
        
        // Test that filesize alias works (was broken due to PEG ordering issue)
        assert!(parse_expr("filesize > 100").is_ok(), "Filesize with bare number should work");
        assert!(parse_expr("filesize > 100kb").is_ok(), "Filesize with kb unit should work");
        
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
        // Test that longer aliases are matched before shorter prefixes
        // This was a bug where "filesize" would match "file" first
        
        // filesize/filetype should not be confused with filename
        assert!(parse_expr("filesize > 100kb").is_ok(), "filesize should parse");
        assert!(parse_expr("filetype == dir").is_ok(), "filetype should parse");
        
        // Verify they parse to the correct predicates
        if let Ok(Expr::Predicate(pred)) = parse_expr("filesize > 100kb") {
            assert!(matches!(pred, Predicate::Metadata(_)), "filesize should be metadata predicate");
        }
        
        if let Ok(Expr::Predicate(pred)) = parse_expr("filetype == dir") {
            assert!(matches!(pred, Predicate::Metadata(_)), "filetype should be metadata predicate");
        }
        
        if let Ok(Expr::Predicate(pred)) = parse_expr("filename == test") {
            assert!(matches!(pred, Predicate::Name(_)), "filename should be name predicate");
        }
    }

    #[test]
    fn parse_operator_aliases() {
        // = vs ==
        let eq1 = parse_expr("filename = foo").unwrap();
        let eq2 = parse_expr("filename == foo").unwrap();
        assert_eq!(eq1, eq2);

        // ~ vs ~= vs =~
        let regex1 = parse_expr("filename ~ pattern").unwrap();
        let regex2 = parse_expr("filename ~= pattern").unwrap();
        let regex3 = parse_expr("filename =~ pattern").unwrap();
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
            "filename ==",
            "filename",
            "@",
            "== foo",
            "filename == foo &&",
            "|| filename == foo",
        ];
        for expr in incomplete {
            assert!(parse_expr(expr).is_err(), "Should fail: {}", expr);
        }
    }

    #[test]
    fn error_malformed_sets() {
        let malformed = vec![
            "ext in [js ts]", // missing comma
            "ext in js, ts]", // missing opening bracket
            "ext in [js, ts", // missing closing bracket
        ];
        for expr in malformed {
            assert!(parse_expr(expr).is_err(), "Should fail: {}", expr);
        }
    }

    #[test]
    fn error_mismatched_quotes() {
        let mismatched = vec![
            r#"filename == "unclosed"#,
            r#"filename == 'unclosed"#,
            r#"filename == "mixed'"#,
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
        assert!(parse_expr(r#"filename ~= "[unclosed""#).is_err());
    }

    #[test]
    fn test_empty_string_extension() {
        // Test parsing ext == ""
        let parsed = parse_expr(r#"ext == """#).unwrap();

        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::Equals("".to_string()),
        ))));

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_empty_string_in_set() {
        // Test parsing empty string in set literal
        let parsed = parse_expr(r#"ext in ["", txt, rs]"#).unwrap();

        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::Extension(
            StringMatcher::In(vec!["".to_string(), "txt".to_string(), "rs".to_string()]),
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
        assert!(!pred_empty.is_match(Path::new("README"))); // This will be false!
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
        let expr = parse_expr(r#"ext in [js, ts]"#).unwrap();

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
        let pred =
            NamePredicate::Extension(StringMatcher::In(vec!["js".to_string(), "ts".to_string()]));

        assert!(pred.is_match(Path::new("file.js")));
        assert!(pred.is_match(Path::new("file.ts")));
        assert!(!pred.is_match(Path::new("file.rs")));
        assert!(!pred.is_match(Path::new("file.txt")));
    }

    #[test]
    fn test_name_in_operator_parsing() {
        // Test parsing name in [index, main] - the case used in failing integration test
        let expr = parse_expr(r#"filename in [index, main]"#).unwrap();

        let expected = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::In(vec!["index".to_string(), "main".to_string()]),
        ))));

        assert_eq!(expr, expected);
    }

    #[test]
    fn test_compound_in_expression_parsing() {
        // Test the exact expression from the failing integration test
        let expr = parse_expr(r#"ext in [js, ts] && filename in [index, main]"#).unwrap();

        let expected = Expr::And(
            Box::new(Expr::Predicate(Predicate::Name(Arc::new(
                NamePredicate::Extension(StringMatcher::In(vec![
                    "js".to_string(),
                    "ts".to_string(),
                ])),
            )))),
            Box::new(Expr::Predicate(Predicate::Name(Arc::new(
                NamePredicate::FileName(StringMatcher::In(vec![
                    "index".to_string(),
                    "main".to_string(),
                ])),
            )))),
        );

        assert_eq!(expr, expected);
    }

    #[test]
    fn test_filename_in_matching() {
        use std::path::Path;

        // Test name matching with 'in' operator
        let pred = NamePredicate::FileName(StringMatcher::In(vec![
            "index".to_string(),
            "main".to_string(),
        ]));

        // Only exact filename matches should work with FileName
        assert!(!pred.is_match(Path::new("index.js"))); // "index" != "index.js"
        assert!(!pred.is_match(Path::new("main.ts")));  // "main" != "main.ts"

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
        let expr = parse_expr(r#"filename ~= "*""#).unwrap();

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
        let parsed = parse_expr(r#"!(filename contains "test")"#).unwrap();

        let inner_pred = Expr::Predicate(Predicate::Name(Arc::new(NamePredicate::FileName(
            StringMatcher::Contains("test".to_string()),
        ))));
        let expected = Expr::Not(Box::new(inner_pred));

        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_complex_negation_parsing() {
        // Test the exact expression from the beta tester's bug report
        let parsed = parse_expr(r#"ext == "rs" && !(filename contains "test")"#).unwrap();

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
        // Test the exact problematic case: ext == "rs" && !(filename contains "lib")
        let parsed = parse_expr(r#"ext == "rs" && !(filename contains "lib")"#).unwrap();

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
        // Test negation without parentheses: !filename == "test"
        let parsed = parse_expr(r#"!filename == "test""#).unwrap();

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
            (r#"filename == test\n"#, "test\n"),
            (r#"filename == test\\"#, "test\\"),
        ];

        for (expr_str, expected_value) in supported_escapes {
            let parsed = parse_expr(expr_str).unwrap();
            assert_name_equals(&parsed, expected_value);
        }

        let unsupported_escapes = vec![
            r#"filename ~= draft\.final\.final\.*"#,
            r#"filename ~= \d+\.\d+"#,
            r#"filename ~= test\$"#,
            r#"filename ~= \^start"#,
            r#"filename ~= foo\+bar"#,
            r#"filename ~= test\?"#,
            r#"filename ~= \(group\)"#,
            r#"filename ~= \[abc\]"#,
            r#"filename ~= a\{2,4\}"#,
            r#"filename ~= one\|two"#,
        ];

        for expr_str in unsupported_escapes {
            assert_parse_error(expr_str);
        }
    }

    #[test]
    fn test_bare_token_escaped_regex_patterns() {
        let test_cases = vec![
            (
                r#"filename ~= draft\.final\.final\.pptx"#,
                "draft.final.final.pptx",
                true,
            ),
            (
                r#"filename ~= draft\.final\.final\.pptx"#,
                "draft-final-final-pptx",
                false,
            ),
            (r#"filename ~= v\d+\.\d+\.\d+"#, "v1.2.3", true),
            (r#"filename ~= v\d+\.\d+\.\d+"#, "v1-2-3", false),
            (r#"filename ~= .*\.rs$"#, "main.rs", true),
            (r#"filename ~= .*\.rs$"#, "main.rs.bak", false), // Should NOT match - filename ends with .bak
            (
                r#"filename ~= \[DRAFT\]\..*\.docx"#,
                "[DRAFT].report.docx",
                true,
            ),
            (r#"filename ~= \[DRAFT\]\..*\.docx"#, "DRAFT.report.docx", false),
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

        let bare_expr = format!(r#"filename contains {}"#, dot_pattern);
        assert_parse_error(&bare_expr);

        let quoted_expr = format!(r#"filename contains "{}""#, dot_pattern);
        let parsed = parse_expr(&quoted_expr).unwrap();
        assert_name_contains(&parsed, "draft\\.final\\.final");
    }

    #[test]
    fn test_size_unit_parsing() {
        // Test various size units - all should be parsed successfully
        let test_cases = vec![
            // Kilobytes
            ("size > 1kb", 1024),
            ("size > 1KB", 1024),
            ("size > 1k", 1024),
            ("size > 1K", 1024),
            // Megabytes
            ("size > 2mb", 2 * 1024 * 1024),
            ("size > 2MB", 2 * 1024 * 1024),
            ("size > 2m", 2 * 1024 * 1024),
            ("size > 2M", 2 * 1024 * 1024),
            // Gigabytes
            ("size > 3gb", 3 * 1024 * 1024 * 1024),
            ("size > 3GB", 3 * 1024 * 1024 * 1024),
            ("size > 3g", 3 * 1024 * 1024 * 1024),
            ("size > 3G", 3 * 1024 * 1024 * 1024),
            // Terabytes
            ("size > 1tb", 1024u64 * 1024 * 1024 * 1024),
            ("size > 1TB", 1024u64 * 1024 * 1024 * 1024),
            ("size > 1t", 1024u64 * 1024 * 1024 * 1024),
            ("size > 1T", 1024u64 * 1024 * 1024 * 1024),
        ];

        for (expr_str, expected_bytes) in test_cases {
            let parsed = parse_expr(expr_str).expect(&format!("Failed to parse: {}", expr_str));
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
            ("size > 2.5mb", (2.5 * 1024.0 * 1024.0) as u64),
            ("size > 1.5gb", (1.5 * 1024.0 * 1024.0 * 1024.0) as u64),
            ("size > 0.5kb", (0.5 * 1024.0) as u64),
        ];

        for (expr_str, expected_bytes) in test_cases {
            let parsed = parse_expr(expr_str).expect(&format!("Failed to parse: {}", expr_str));
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
                NumberMatcher::In(Bound::Right(..(500 * 1024 + 1))),
            ),
            (
                "size <= 1gb",
                NumberMatcher::In(Bound::Right(..(1024 * 1024 * 1024))),
            ),
            (
                "size >= 100mb",
                NumberMatcher::In(Bound::Left((100 * 1024 * 1024 - 1)..)),
            ),
        ];

        for (expr_str, expected_matcher) in test_cases {
            let parsed = parse_expr(expr_str).expect(&format!("Failed to parse: {}", expr_str));
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
                assert_eq!(plain_range.start, 1000);
                assert_eq!(size_range.start, 1024);
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
            ("(filename contains test)", true),
            // Parentheses with OR
            ("(filename contains spec || filename contains test)", true),
            // Complex expression from bug report
            ("dirpath contains multivendor-plugin && (filename contains spec || filename contains test)", true),
            // Nested parentheses
            ("((filename contains a || filename contains b) && ext == rs)", true),
            // Multiple groups
            ("(filename contains foo) && (size > 1000 || ext == txt)", true),
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
}
