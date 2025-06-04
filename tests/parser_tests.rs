#[cfg(test)]
mod parser_tests {
    use detect::parser::parse_query;
    use detect::query::*;

    // ============================================================================
    // WHITESPACE TESTS
    // ============================================================================

    #[test]
    fn test_whitespace_handling() {
        // Multiple spaces
        assert!(parse_query("*.rs  &&  TODO").is_ok());
        // Tabs
        assert!(parse_query("*.rs\t&&\tTODO").is_ok());
        // Newlines
        assert!(parse_query("*.rs\n&&\nTODO").is_ok());
        // Mixed whitespace
        assert!(parse_query("  \t*.rs  \n  &&   TODO\t  ").is_ok());
    }

    // ============================================================================
    // IMPLICIT SEARCH TESTS
    // ============================================================================

    #[test]
    fn test_quoted_strings() {
        // Simple quoted string
        let q = parse_query("\"hello world\"").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Quoted(s)))) => {
                    assert_eq!(s, "hello world");
                }
                _ => panic!("Expected quoted string pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Empty quoted string
        let q = parse_query("\"\"").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Quoted(s)))) => {
                    assert_eq!(s, "");
                }
                _ => panic!("Expected quoted string pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Quoted string with special chars
        let q = parse_query("\"test-file_2024.txt\"").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Quoted(s)))) => {
                    assert_eq!(s, "test-file_2024.txt");
                }
                _ => panic!("Expected quoted string pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Quoted string with spaces and punctuation
        let q = parse_query("\"TODO: Fix this!\"").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Quoted(s)))) => {
                    assert_eq!(s, "TODO: Fix this!");
                }
                _ => panic!("Expected quoted string pattern"),
            },
            _ => panic!("Expected expression"),
        }
    }

    #[test]
    fn test_regex_patterns() {
        // Simple regex
        let q = parse_query("/test/").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Regex(p, f)))) => {
                    assert_eq!(p, "test");
                    assert_eq!(f, "");
                }
                _ => panic!("Expected regex pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Regex with flags
        let q = parse_query("/TODO/i").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Regex(p, f)))) => {
                    assert_eq!(p, "TODO");
                    assert_eq!(f, "i");
                }
                _ => panic!("Expected regex pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Regex with multiple flags
        let q = parse_query("/pattern/ims").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Regex(p, f)))) => {
                    assert_eq!(p, "pattern");
                    assert_eq!(f, "ims");
                }
                _ => panic!("Expected regex pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Empty regex
        let q = parse_query("//").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Regex(p, f)))) => {
                    assert_eq!(p, "");
                    assert_eq!(f, "");
                }
                _ => panic!("Expected regex pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Complex regex pattern
        let q = parse_query("/^\\w+\\.rs$/").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Regex(p, f)))) => {
                    assert_eq!(p, "^\\w+\\.rs$");
                    assert_eq!(f, "");
                }
                _ => panic!("Expected regex pattern"),
            },
            _ => panic!("Expected expression"),
        }
    }

    #[test]
    fn test_glob_patterns() {
        // Star glob
        let q = parse_query("*.rs").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Glob(s)))) => {
                    assert_eq!(s, "*.rs");
                }
                _ => panic!("Expected glob pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Question mark glob
        let q = parse_query("test?.txt").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Glob(s)))) => {
                    assert_eq!(s, "test?.txt");
                }
                _ => panic!("Expected glob pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Bracket glob
        let q = parse_query("file[0-9].txt").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Glob(s)))) => {
                    assert_eq!(s, "file[0-9].txt");
                }
                _ => panic!("Expected glob pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Brace glob
        let q = parse_query("{src,test}/*.rs").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Glob(s)))) => {
                    assert_eq!(s, "{src,test}/*.rs");
                }
                _ => panic!("Expected glob pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Path as glob
        let q = parse_query("src/main.rs").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Glob(s)))) => {
                    assert_eq!(s, "src/main.rs");
                }
                _ => panic!("Expected glob pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Double star glob
        let q = parse_query("**/*.js").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Glob(s)))) => {
                    assert_eq!(s, "**/*.js");
                }
                _ => panic!("Expected glob pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Complex glob
        let q = parse_query("src/**/test_*.{rs,py}").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Glob(s)))) => {
                    assert_eq!(s, "src/**/test_*.{rs,py}");
                }
                _ => panic!("Expected glob pattern"),
            },
            _ => panic!("Expected expression"),
        }
    }

    #[test]
    fn test_bare_words() {
        // Simple bare word
        let q = parse_query("TODO").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Bare(s)))) => {
                    assert_eq!(s, "TODO");
                }
                _ => panic!("Expected bare word pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Bare word with dots and dashes
        let q = parse_query("file-name.ext").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Bare(s)))) => {
                    assert_eq!(s, "file-name.ext");
                }
                _ => panic!("Expected bare word pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Bare word with underscores
        let q = parse_query("test_file_123").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Bare(s)))) => {
                    assert_eq!(s, "test_file_123");
                }
                _ => panic!("Expected bare word pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // All caps bare word
        let q = parse_query("README").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Bare(s)))) => {
                    assert_eq!(s, "README");
                }
                _ => panic!("Expected bare word pattern"),
            },
            _ => panic!("Expected expression"),
        }
    }

    // ============================================================================
    // FILTERED SEARCH TESTS
    // ============================================================================

    #[test]
    fn test_file_type_filters() {
        // Rust file type
        let q = parse_query("rust >1MB").unwrap();
        match q {
            Query::Filtered { base, filters } => {
                assert!(matches!(base, FilterBase::Type(FileType::Rust)));
                assert_eq!(filters.len(), 1);
            }
            _ => panic!("Expected filtered query"),
        }

        // Python file type (short form)
        let q = parse_query("py modified:today").unwrap();
        match q {
            Query::Filtered { base, filters } => {
                assert!(matches!(base, FilterBase::Type(FileType::Python)));
                assert_eq!(filters.len(), 1);
            }
            _ => panic!("Expected filtered query"),
        }

        // All file types
        let types = vec![
            ("rust", FileType::Rust),
            ("rs", FileType::Rust),
            ("python", FileType::Python),
            ("py", FileType::Python),
            ("javascript", FileType::JavaScript),
            ("js", FileType::JavaScript),
            ("typescript", FileType::TypeScript),
            ("ts", FileType::TypeScript),
            ("go", FileType::Go),
            ("java", FileType::Java),
            ("cpp", FileType::Cpp),
            ("c", FileType::C),
            ("image", FileType::Image),
            ("video", FileType::Video),
            ("audio", FileType::Audio),
            ("text", FileType::Text),
            ("binary", FileType::Binary),
        ];

        for (type_str, expected_type) in types {
            let q = parse_query(type_str).unwrap();
            match q {
                Query::Filtered { base, filters } => {
                    match base {
                        FilterBase::Type(t) => assert_eq!(t, expected_type),
                        _ => panic!("Expected type filter for {}", type_str),
                    }
                    assert_eq!(filters.len(), 0);
                }
                _ => panic!("Expected filtered query for {}", type_str),
            }
        }
    }

    #[test]
    fn test_type_with_pattern() {
        // Type with pattern
        let q = parse_query("rust TODO").unwrap();
        match q {
            Query::Filtered { base, filters } => {
                assert!(
                    matches!(base, FilterBase::TypeWithPattern(FileType::Rust, Pattern::Bare(s)) if s == "TODO")
                );
                assert_eq!(filters.len(), 0);
            }
            _ => panic!("Expected filtered query"),
        }

        // Type with glob pattern
        let q = parse_query("python *.py").unwrap();
        match q {
            Query::Filtered { base, filters } => {
                assert!(
                    matches!(base, FilterBase::TypeWithPattern(FileType::Python, Pattern::Glob(s)) if s == "*.py")
                );
                assert_eq!(filters.len(), 0);
            }
            _ => panic!("Expected filtered query"),
        }
    }

    #[test]
    fn test_size_filters() {
        // Basic size comparisons
        let test_cases = vec![
            ("*.rs >100", SizeOp::Greater, 100.0, SizeUnit::Bytes),
            ("*.rs >=1KB", SizeOp::GreaterEqual, 1.0, SizeUnit::Kilobytes),
            ("*.rs <2.5MB", SizeOp::Less, 2.5, SizeUnit::Megabytes),
            ("*.rs <=1GB", SizeOp::LessEqual, 1.0, SizeUnit::Gigabytes),
            ("*.rs =512B", SizeOp::Equal, 512.0, SizeUnit::Bytes),
        ];

        for (input, expected_op, expected_val, expected_unit) in test_cases {
            let q = parse_query(input).unwrap();
            match q {
                Query::Filtered { base, filters } => {
                    assert!(matches!(base, FilterBase::Pattern(Pattern::Glob(s)) if s == "*.rs"));
                    assert_eq!(filters.len(), 1);
                    match &filters[0] {
                        Filter::Size(op, val, unit) => {
                            assert_eq!(*op, expected_op);
                            assert_eq!(*val, expected_val);
                            assert_eq!(*unit, expected_unit);
                        }
                        _ => panic!("Expected size filter"),
                    }
                }
                _ => panic!("Expected filtered query"),
            }
        }

        // Different size units
        let units = vec![
            ("100B", 100.0, SizeUnit::Bytes),
            ("1K", 1.0, SizeUnit::Kilobytes),
            ("2KB", 2.0, SizeUnit::Kilobytes),
            ("3M", 3.0, SizeUnit::Megabytes),
            ("4MB", 4.0, SizeUnit::Megabytes),
            ("5G", 5.0, SizeUnit::Gigabytes),
            ("6GB", 6.0, SizeUnit::Gigabytes),
        ];

        for (size_str, expected_val, expected_unit) in units {
            let q = parse_query(&format!("*.txt >{}", size_str)).unwrap();
            match q {
                Query::Filtered { base: _, filters } => match &filters[0] {
                    Filter::Size(_, val, unit) => {
                        assert_eq!(*val, expected_val);
                        assert_eq!(*unit, expected_unit);
                    }
                    _ => panic!("Expected size filter"),
                },
                _ => panic!("Expected filtered query"),
            }
        }
    }

    #[test]
    fn test_time_filters() {
        // Modified time with keyword
        let q = parse_query("*.rs modified:today").unwrap();
        match q {
            Query::Filtered { base: _, filters } => {
                assert_eq!(filters.len(), 1);
                match &filters[0] {
                    Filter::Time(TimeSelector::Modified, TimeExpr::Keyword(TimeKeyword::Today)) => {
                    }
                    _ => panic!("Expected modified:today filter"),
                }
            }
            _ => panic!("Expected filtered query"),
        }

        // Time selectors
        let selectors = vec![
            ("modified", TimeSelector::Modified),
            ("m", TimeSelector::Modified),
            ("created", TimeSelector::Created),
            ("c", TimeSelector::Created),
            ("accessed", TimeSelector::Accessed),
            ("a", TimeSelector::Accessed),
        ];

        for (sel_str, expected_sel) in selectors {
            let q = parse_query(&format!("*.txt {}:1d", sel_str)).unwrap();
            match q {
                Query::Filtered { base: _, filters } => match &filters[0] {
                    Filter::Time(sel, TimeExpr::Relative(1.0, TimeUnit::Days)) => {
                        assert_eq!(*sel, expected_sel);
                    }
                    _ => panic!("Expected time filter"),
                },
                _ => panic!("Expected filtered query"),
            }
        }

        // Time units
        let units = vec![
            ("1s", 1.0, TimeUnit::Seconds),
            ("30m", 30.0, TimeUnit::Minutes),
            ("2h", 2.0, TimeUnit::Hours),
            ("7d", 7.0, TimeUnit::Days),
            ("4w", 4.0, TimeUnit::Weeks),
            ("3mo", 3.0, TimeUnit::Months),
            ("1y", 1.0, TimeUnit::Years),
        ];

        for (time_str, expected_val, expected_unit) in units {
            let q = parse_query(&format!("*.log modified:{}", time_str)).unwrap();
            match q {
                Query::Filtered { base: _, filters } => match &filters[0] {
                    Filter::Time(_, TimeExpr::Relative(val, unit)) => {
                        assert_eq!(*val, expected_val);
                        assert_eq!(*unit, expected_unit);
                    }
                    _ => panic!("Expected relative time"),
                },
                _ => panic!("Expected filtered query"),
            }
        }

        // Time keywords
        let keywords = vec![
            ("today", TimeKeyword::Today),
            ("yesterday", TimeKeyword::Yesterday),
            ("now", TimeKeyword::Now),
        ];

        for (kw_str, expected_kw) in keywords {
            let q = parse_query(&format!("*.txt created:{}", kw_str)).unwrap();
            match q {
                Query::Filtered { base: _, filters } => match &filters[0] {
                    Filter::Time(_, TimeExpr::Keyword(kw)) => {
                        assert_eq!(*kw, expected_kw);
                    }
                    _ => panic!("Expected time keyword"),
                },
                _ => panic!("Expected filtered query"),
            }
        }
    }

    #[test]
    fn test_path_filters() {
        // With quoted path
        let q = parse_query("*.rs in:\"src/main\"").unwrap();
        match q {
            Query::Filtered { base: _, filters } => {
                assert_eq!(filters.len(), 1);
                match &filters[0] {
                    Filter::Path(path) => assert_eq!(path, "src/main"),
                    _ => panic!("Expected path filter"),
                }
            }
            _ => panic!("Expected filtered query"),
        }

        // Different prefixes
        let prefixes = vec!["in:", "dir:", "path:"];
        for prefix in prefixes {
            let q = parse_query(&format!("*.py {}~/projects", prefix)).unwrap();
            match q {
                Query::Filtered { base: _, filters } => match &filters[0] {
                    Filter::Path(path) => assert_eq!(path, "~/projects"),
                    _ => panic!("Expected path filter"),
                },
                _ => panic!("Expected filtered query"),
            }
        }

        // Path with spaces (quoted)
        let q = parse_query("*.txt in:\"My Documents/Projects\"").unwrap();
        match q {
            Query::Filtered { base: _, filters } => match &filters[0] {
                Filter::Path(path) => assert_eq!(path, "My Documents/Projects"),
                _ => panic!("Expected path filter"),
            },
            _ => panic!("Expected filtered query"),
        }
    }

    #[test]
    fn test_property_filters() {
        let properties = vec![
            ("executable", Property::Executable),
            ("hidden", Property::Hidden),
            ("empty", Property::Empty),
            ("binary", Property::Binary),
            ("symlink", Property::Symlink),
        ];

        for (prop_str, expected_prop) in properties {
            let q = parse_query(&format!("*.txt {}", prop_str)).unwrap();
            match q {
                Query::Filtered { base: _, filters } => {
                    assert_eq!(filters.len(), 1);
                    match &filters[0] {
                        Filter::Property(prop) => assert_eq!(*prop, expected_prop),
                        _ => panic!("Expected property filter"),
                    }
                }
                _ => panic!("Expected filtered query"),
            }
        }
    }

    #[test]
    fn test_multiple_filters() {
        let q = parse_query("rust >1MB modified:today in:src executable").unwrap();
        match q {
            Query::Filtered { base, filters } => {
                assert!(matches!(base, FilterBase::Type(FileType::Rust)));
                assert_eq!(filters.len(), 4);

                // Check each filter type is present
                let has_size = filters.iter().any(|f| matches!(f, Filter::Size(_, _, _)));
                let has_time = filters.iter().any(|f| matches!(f, Filter::Time(_, _)));
                let has_path = filters.iter().any(|f| matches!(f, Filter::Path(_)));
                let has_prop = filters.iter().any(|f| matches!(f, Filter::Property(_)));

                assert!(has_size && has_time && has_path && has_prop);
            }
            _ => panic!("Expected filtered query"),
        }

        // Pattern with multiple filters
        let q = parse_query("*.rs >100KB <1MB modified:7d in:src/lib").unwrap();
        match q {
            Query::Filtered { base: _, filters } => {
                assert_eq!(filters.len(), 4);
            }
            _ => panic!("Expected filtered query"),
        }
    }

    // ============================================================================
    // EXPRESSION TESTS
    // ============================================================================

    #[test]
    fn test_boolean_operators() {
        // AND with different forms
        assert!(parse_query("*.rs && TODO").is_ok());
        assert!(parse_query("*.rs and TODO").is_ok());

        // OR with different forms
        assert!(parse_query("*.rs || *.py").is_ok());
        assert!(parse_query("*.rs or *.py").is_ok());

        // NOT with different forms
        assert!(parse_query("!hidden").is_ok());
        assert!(parse_query("not hidden").is_ok());

        // Complex boolean expressions
        assert!(parse_query("*.rs && (TODO || FIXME)").is_ok());
        assert!(parse_query("!hidden && (*.rs || *.py) && size > 1KB").is_ok());
    }

    #[test]
    fn test_operator_precedence() {
        // AND has higher precedence than OR
        let q = parse_query("*.rs && TODO || *.py").unwrap();
        match q {
            Query::Expression(expr) => {
                // Should parse as (*.rs && TODO) || *.py
                assert!(matches!(expr.as_ref(), Expression::Or(_, _)));
            }
            _ => panic!("Expected expression"),
        }

        // NOT has highest precedence
        let q = parse_query("!hidden && executable").unwrap();
        match q {
            Query::Expression(expr) => {
                // Should parse as (!hidden) && executable
                assert!(matches!(expr.as_ref(), Expression::And(_, _)));
            }
            _ => panic!("Expected expression"),
        }
    }

    #[test]
    fn test_parentheses() {
        let q = parse_query("(*.rs || *.py) && TODO").unwrap();
        match q {
            Query::Expression(expr) => {
                assert!(matches!(expr.as_ref(), Expression::And(_, _)));
            }
            _ => panic!("Expected expression"),
        }

        // Nested parentheses
        assert!(parse_query("((*.rs || *.py) && TODO) || *.md").is_ok());
        assert!(parse_query("(((*.rs)))").is_ok());

        // Complex nesting
        assert!(parse_query("((*.rs && !hidden) || (*.py && executable)) && size > 100KB").is_ok());
    }

    #[test]
    fn test_predicates() {
        // Selector with comparison
        let q = parse_query("name == \"test.rs\"").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Predicate(PredicateExpr::Comparison(sel, op, val))) => {
                    assert!(matches!(sel, Selector::Name));
                    assert!(matches!(op, CompOp::Equal));
                    assert!(matches!(val, Value::String(s) if s == "test.rs"));
                }
                _ => panic!("Expected predicate comparison"),
            },
            _ => panic!("Expected expression"),
        }

        // All selectors
        let selectors = vec![
            "name", "path", "ext", "size", "type", "content", "lines", "binary", "empty",
        ];

        for sel in &selectors {
            // Test with comparison (except boolean properties)
            if *sel != "binary" && *sel != "empty" {
                let q = parse_query(&format!("{} == test", sel)).unwrap();
                assert!(matches!(q, Query::Expression(_)));
            }
        }

        // Boolean properties
        for sel in &["binary", "empty"] {
            let q = parse_query(sel).unwrap();
            match q {
                Query::Expression(expr) => match expr.as_ref() {
                    Expression::Atom(Atom::Predicate(PredicateExpr::Property(_))) => {}
                    _ => panic!("Expected property predicate"),
                },
                _ => panic!("Expected expression"),
            }
        }
    }

    #[test]
    fn test_comparison_operators() {
        let ops = vec![
            ("==", CompOp::Equal),
            ("=", CompOp::Equal),
            ("!=", CompOp::NotEqual),
            ("~=", CompOp::Matches),
            (">", CompOp::Greater),
            (">=", CompOp::GreaterEqual),
            ("<", CompOp::Less),
            ("<=", CompOp::LessEqual),
        ];

        for (op_str, expected_op) in ops {
            let q = parse_query(&format!("size {} 100", op_str)).unwrap();
            match q {
                Query::Expression(expr) => match expr.as_ref() {
                    Expression::Atom(Atom::Predicate(PredicateExpr::Comparison(_, op, _))) => {
                        assert_eq!(*op, expected_op);
                    }
                    _ => panic!("Expected comparison"),
                },
                _ => panic!("Expected expression"),
            }
        }
    }

    #[test]
    fn test_contains_expression() {
        // Contains with regex
        let q = parse_query("contains(/TODO/)").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Predicate(PredicateExpr::Contains(Pattern::Regex(
                    p,
                    f,
                )))) => {
                    assert_eq!(p, "TODO");
                    assert_eq!(f, "");
                }
                _ => panic!("Expected contains predicate"),
            },
            _ => panic!("Expected expression"),
        }

        // Contains with quoted string
        let q = parse_query("contains(\"exact match\")").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Predicate(PredicateExpr::Contains(Pattern::Quoted(s)))) => {
                    assert_eq!(s, "exact match");
                }
                _ => panic!("Expected contains predicate"),
            },
            _ => panic!("Expected expression"),
        }

        // Contains with regex flags
        let q = parse_query("contains(/todo/i)").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Predicate(PredicateExpr::Contains(Pattern::Regex(
                    p,
                    f,
                )))) => {
                    assert_eq!(p, "todo");
                    assert_eq!(f, "i");
                }
                _ => panic!("Expected contains predicate"),
            },
            _ => panic!("Expected expression"),
        }
    }

    #[test]
    fn test_numeric_values() {
        // Integer values
        let q = parse_query("size > 1000").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Predicate(PredicateExpr::Comparison(_, _, val))) => {
                    assert!(matches!(val, Value::Number(n, None) if *n == 1000.0));
                }
                _ => panic!("Expected numeric comparison"),
            },
            _ => panic!("Expected expression"),
        }

        // Float values
        let q = parse_query("size > 123.456").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Predicate(PredicateExpr::Comparison(_, _, val))) => {
                    assert!(matches!(val, Value::Number(n, None) if *n == 123.456));
                }
                _ => panic!("Expected numeric comparison"),
            },
            _ => panic!("Expected expression"),
        }

        // With size units
        let q = parse_query("size <= 2.5MB").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Predicate(PredicateExpr::Comparison(_, _, val))) => {
                    assert!(
                        matches!(val, Value::Number(n, Some(SizeUnit::Megabytes)) if *n == 2.5)
                    );
                }
                _ => panic!("Expected numeric comparison"),
            },
            _ => panic!("Expected expression"),
        }
    }

    #[test]
    fn test_complex_expressions() {
        // Mixed implicit searches and predicates
        let q = parse_query("(*.rs || *.py) && size > 1KB && contains(/TODO/)").unwrap();
        assert!(matches!(q, Query::Expression(_)));

        // Filtered search in expression
        let q = parse_query("(rust >1MB) || (python <100KB)").unwrap();
        assert!(matches!(q, Query::Expression(_)));

        // Deep nesting
        let q = parse_query("!(!(!hidden))").unwrap();
        assert!(matches!(q, Query::Expression(_)));

        // Very complex query
        let q =
            parse_query("(rust TODO || python FIXME) && size > 100KB && modified:7d && !hidden")
                .unwrap();
        assert!(matches!(q, Query::Expression(_)));
    }

    // ============================================================================
    // EDGE CASES AND ERROR TESTS
    // ============================================================================

    #[test]
    fn test_edge_cases() {
        // Empty program should fail
        assert!(parse_query("").is_err());

        // Just whitespace should fail
        assert!(parse_query("   \t\n  ").is_err());

        // Incomplete expressions
        assert!(parse_query("*.rs &&").is_err());
        assert!(parse_query("|| *.py").is_err());
        assert!(parse_query("name ==").is_err());
        assert!(parse_query("size >").is_err());

        // Invalid operators
        assert!(parse_query("*.rs & *.py").is_err());
        assert!(parse_query("size >> 100").is_err());
        assert!(parse_query("name === test").is_err());

        // Mismatched parentheses
        assert!(parse_query("(*.rs").is_err());
        assert!(parse_query("*.rs)").is_err());
        assert!(parse_query("((*.rs)").is_err());
        assert!(parse_query("(*.rs))").is_err());

        // Invalid regex
        assert!(parse_query("/unclosed").is_err());

        // Invalid contains
        assert!(parse_query("contains()").is_err());
        assert!(parse_query("contains(bare)").is_err());

        // Invalid filters
        assert!(parse_query("*.rs >").is_err());
        assert!(parse_query("*.rs modified:").is_err());
        assert!(parse_query("*.rs in:").is_err());
    }

    #[test]
    fn test_special_characters_in_strings() {
        // Escaping in quoted strings would be nice but not required for MVP
        let q = parse_query("\"file with spaces.txt\"").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Quoted(s)))) => {
                    assert_eq!(s, "file with spaces.txt");
                }
                _ => panic!("Expected quoted string pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // Special chars in regex
        let q = parse_query("/\\w+\\.(rs|py)/").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Regex(p, _)))) => {
                    assert_eq!(p, "\\w+\\.(rs|py)");
                }
                _ => panic!("Expected regex pattern"),
            },
            _ => panic!("Expected expression"),
        }
    }

    #[test]
    fn test_ambiguous_cases() {
        // "not" as a bare word vs operator
        let q = parse_query("not").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Bare(s)))) => {
                    assert_eq!(s, "not");
                }
                _ => panic!("Expected bare word pattern"),
            },
            _ => panic!("Expected expression"),
        }

        // "not " with space triggers operator
        let q = parse_query("not hidden").unwrap();
        match q {
            Query::Expression(expr) => {
                assert!(matches!(expr.as_ref(), Expression::Not(_)));
            }
            _ => panic!("Expected NOT expression"),
        }

        // File type that could be mistaken for other things
        let q = parse_query("c").unwrap();
        match q {
            Query::Filtered { base, filters } => {
                assert!(matches!(base, FilterBase::Type(FileType::C)));
                assert_eq!(filters.len(), 0);
            }
            _ => panic!("Expected C file type"),
        }
    }

    #[test]
    fn test_realistic_queries() {
        // Find TODOs in Rust files
        assert!(parse_query("rust contains(/TODO|FIXME/)").is_ok());

        // Large Python files in src
        assert!(parse_query("*.py >100KB in:src").is_ok());

        // Recent changes
        assert!(parse_query("modified:7d && (*.rs || *.py)").is_ok());

        // Executable scripts
        assert!(parse_query("executable && (*.sh || *.py)").is_ok());

        // Find test files
        assert!(parse_query("(name ~= /test/ || path ~= /test/) && *.rs").is_ok());

        // Complex real-world query
        assert!(parse_query(
            "(rust || python) && size > 1KB && modified:30d && contains(/TODO/) && !path ~= /vendor/"
        ).is_ok());
    }
}
