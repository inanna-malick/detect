#[cfg(test)]
mod tests {
    use detect::parser::{parse_query, parse_expr};
    use detect::query::*;

    #[test]
    fn test_parse_bare_word() {
        let cases = vec![
            ("TODO", Pattern::Bare("TODO".to_string())),
            ("README", Pattern::Bare("README".to_string())),
            ("test123", Pattern::Bare("test123".to_string())),
            ("file.txt", Pattern::Bare("file.txt".to_string())),
        ];

        for (input, expected) in cases {
            let q = parse_query(input).unwrap();
            match q {
                Query::Expression(expr) => match expr.as_ref() {
                    Expression::Atom(Atom::Query(Query::Implicit(pattern))) => {
                        assert_eq!(pattern, &expected, "Failed for input: {}", input);
                    }
                    _ => panic!("Expected implicit query with pattern, got {:?}", expr),
                },
                Query::Implicit(pattern) => {
                    assert_eq!(pattern, expected, "Failed for input: {}", input);
                }
                _ => panic!("Expected expression or implicit, got {:?}", q),
            }
        }
    }

    #[test]
    fn test_parse_quoted_string() {
        let cases = vec![
            (r#""hello world""#, Pattern::Quoted("hello world".to_string())),
            (r#""test.txt""#, Pattern::Quoted("test.txt".to_string())),
            (r#""""#, Pattern::Quoted("".to_string())),
            (r#""with/path/to/file""#, Pattern::Quoted("with/path/to/file".to_string())),
        ];

        for (input, expected) in cases {
            let q = parse_query(input).unwrap();
            match q {
                Query::Expression(expr) => match expr.as_ref() {
                    Expression::Atom(Atom::Query(Query::Implicit(pattern))) => {
                        assert_eq!(pattern, &expected, "Failed for input: {}", input);
                    }
                    _ => panic!("Expected implicit query with pattern, got {:?}", expr),
                },
                Query::Implicit(pattern) => {
                    assert_eq!(pattern, expected, "Failed for input: {}", input);
                }
                _ => panic!("Expected expression or implicit, got {:?}", q),
            }
        }
    }

    #[test]
    fn test_parse_regex_pattern() {
        let cases = vec![
            ("/TODO/", Pattern::Regex("TODO".to_string(), "".to_string())),
            ("/TODO/i", Pattern::Regex("TODO".to_string(), "i".to_string())),
            ("/test.*file/im", Pattern::Regex("test.*file".to_string(), "im".to_string())),
            ("/^start/", Pattern::Regex("^start".to_string(), "".to_string())),
        ];

        for (input, expected) in cases {
            let q = parse_query(input).unwrap();
            match q {
                Query::Expression(expr) => match expr.as_ref() {
                    Expression::Atom(Atom::Query(Query::Implicit(pattern))) => {
                        assert_eq!(pattern, &expected, "Failed for input: {}", input);
                    }
                    _ => panic!("Expected implicit query with pattern, got {:?}", expr),
                },
                Query::Implicit(pattern) => {
                    assert_eq!(pattern, expected, "Failed for input: {}", input);
                }
                _ => panic!("Expected expression or implicit, got {:?}", q),
            }
        }
    }

    #[test]
    fn test_parse_glob_pattern() {
        let cases = vec![
            ("*.rs", Pattern::Glob("*.rs".to_string())),
            ("src/*.js", Pattern::Glob("src/*.js".to_string())),
            ("src/*.{js,ts}", Pattern::Glob("src/*.{js,ts}".to_string())),
            ("[abc]*.txt", Pattern::Glob("[abc]*.txt".to_string())),
            ("test?.rs", Pattern::Glob("test?.rs".to_string())),
        ];

        for (input, expected) in cases {
            let q = parse_query(input).unwrap();
            match q {
                Query::Expression(expr) => match expr.as_ref() {
                    Expression::Atom(Atom::Query(Query::Implicit(pattern))) => {
                        assert_eq!(pattern, &expected, "Failed for input: {}", input);
                    }
                    _ => panic!("Expected implicit query with pattern, got {:?}", expr),
                },
                Query::Implicit(pattern) => {
                    assert_eq!(pattern, expected, "Failed for input: {}", input);
                }
                _ => panic!("Expected expression or implicit, got {:?}", q),
            }
        }
    }

    #[test]
    fn test_parse_file_types() {
        let type_only_cases = vec![
            ("rust >10KB", FileType::Rust),
            ("python modified:today", FileType::Python),
            ("go executable", FileType::Go),
        ];

        for (input, expected) in type_only_cases {
            let q = parse_query(input).unwrap();
            match q {
                Query::Filtered { base, filters } => {
                    match base {
                        FilterBase::Type(file_type) => {
                            assert_eq!(file_type, expected, "Failed for input: {}", input);
                        }
                        _ => panic!("Expected file type base"),
                    }
                    assert!(filters.len() > 0);
                }
                _ => panic!("Expected filtered query"),
            }
        }

        let type_with_pattern_cases = vec![
            ("rust TODO", FileType::Rust, Pattern::Bare("TODO".to_string())),
            ("python main", FileType::Python, Pattern::Bare("main".to_string())),
            ("js *.test.js", FileType::JavaScript, Pattern::Glob("*.test.js".to_string())),
        ];

        for (input, expected_type, expected_pattern) in type_with_pattern_cases {
            let q = parse_query(input).unwrap();
            match q {
                Query::Filtered { base, filters } => {
                    match base {
                        FilterBase::TypeWithPattern(file_type, pattern) => {
                            assert_eq!(file_type, expected_type, "Failed for input: {}", input);
                            assert_eq!(pattern, expected_pattern, "Failed for input: {}", input);
                        }
                        _ => panic!("Expected type with pattern base"),
                    }
                    assert_eq!(filters.len(), 0);
                }
                _ => panic!("Expected filtered query"),
            }
        }
    }

    #[test]
    fn test_parse_size_filters() {
        let cases = vec![
            ("*.rs >1MB", SizeOp::Greater, 1.0, SizeUnit::Megabytes),
            ("*.rs >=100KB", SizeOp::GreaterEqual, 100.0, SizeUnit::Kilobytes),
            ("*.rs <10GB", SizeOp::Less, 10.0, SizeUnit::Gigabytes),
            ("*.rs <=500B", SizeOp::LessEqual, 500.0, SizeUnit::Bytes),
            ("*.rs =1024", SizeOp::Equal, 1024.0, SizeUnit::Bytes),
            ("*.rs >1.5M", SizeOp::Greater, 1.5, SizeUnit::Megabytes),
        ];

        for (input, expected_op, expected_val, expected_unit) in cases {
            let q = parse_query(input).unwrap();
            match q {
                Query::Filtered { base: _, filters } => {
                    assert_eq!(filters.len(), 1);
                    match &filters[0] {
                        Filter::Size(op, val, unit) => {
                            assert_eq!(op, &expected_op);
                            assert_eq!(val, &expected_val);
                            assert_eq!(unit, &expected_unit);
                        }
                        _ => panic!("Expected size filter"),
                    }
                }
                _ => panic!("Expected filtered query"),
            }
        }
    }

    #[test]
    fn test_parse_time_filters() {
        let cases = vec![
            ("*.rs modified:10d", TimeSelector::Modified, TimeExpr::Relative(10.0, TimeUnit::Days)),
            ("*.rs created:2h", TimeSelector::Created, TimeExpr::Relative(2.0, TimeUnit::Hours)),
            ("*.rs accessed:30m", TimeSelector::Accessed, TimeExpr::Relative(30.0, TimeUnit::Minutes)),
            ("*.rs m:1w", TimeSelector::Modified, TimeExpr::Relative(1.0, TimeUnit::Weeks)),
            ("*.rs c:today", TimeSelector::Created, TimeExpr::Keyword(TimeKeyword::Today)),
            ("*.rs a:yesterday", TimeSelector::Accessed, TimeExpr::Keyword(TimeKeyword::Yesterday)),
        ];

        for (input, expected_selector, expected_expr) in cases {
            let q = parse_query(input).unwrap();
            match q {
                Query::Filtered { base: _, filters } => {
                    assert_eq!(filters.len(), 1);
                    match &filters[0] {
                        Filter::Time(selector, expr) => {
                            assert_eq!(selector, &expected_selector);
                            assert_eq!(expr, &expected_expr);
                        }
                        _ => panic!("Expected time filter"),
                    }
                }
                _ => panic!("Expected filtered query"),
            }
        }
    }

    #[test]
    fn test_parse_path_filters() {
        let cases = vec![
            (r#"*.rs in:"src/test""#, "src/test"),
            (r#"*.rs dir:~/projects"#, "~/projects"),
            ("*.rs in:src/main", "src/main"),
        ];

        for (input, expected_path) in cases {
            let q = parse_query(input).unwrap();
            match q {
                Query::Filtered { base: _, filters } => {
                    assert_eq!(filters.len(), 1);
                    match &filters[0] {
                        Filter::Path(path) => {
                            assert_eq!(path, expected_path);
                        }
                        _ => panic!("Expected path filter"),
                    }
                }
                _ => panic!("Expected filtered query"),
            }
        }
    }

    #[test]
    fn test_parse_property_filters() {
        let cases = vec![
            ("*.rs executable", Property::Executable),
            ("*.rs hidden", Property::Hidden),
            ("*.rs empty", Property::Empty),
            ("*.rs binary", Property::Binary),
            ("*.rs symlink", Property::Symlink),
        ];

        for (input, expected_prop) in cases {
            let q = parse_query(input).unwrap();
            match q {
                Query::Filtered { base: _, filters } => {
                    assert_eq!(filters.len(), 1);
                    match &filters[0] {
                        Filter::Property(prop) => {
                            assert_eq!(prop, &expected_prop);
                        }
                        _ => panic!("Expected property filter"),
                    }
                }
                _ => panic!("Expected filtered query"),
            }
        }
    }

    #[test]
    fn test_parse_multiple_filters() {
        let q = parse_query("*.rs >100KB modified:today in:src executable").unwrap();
        match q {
            Query::Filtered { base, filters } => {
                assert!(matches!(base, FilterBase::Pattern(Pattern::Glob(s)) if s == "*.rs"));
                assert_eq!(filters.len(), 4);
                
                assert!(matches!(&filters[0], Filter::Size(SizeOp::Greater, v, SizeUnit::Kilobytes) if *v == 100.0));
                assert!(matches!(&filters[1], Filter::Time(TimeSelector::Modified, TimeExpr::Keyword(TimeKeyword::Today))));
                assert!(matches!(&filters[2], Filter::Path(p) if p == "src"));
                assert!(matches!(&filters[3], Filter::Property(Property::Executable)));
            }
            _ => panic!("Expected filtered query"),
        }
    }

    #[test]
    fn test_parse_predicates() {
        let cases = vec![
            ("name = \"test.rs\"", Selector::Name, CompOp::Equal, Value::String("test.rs".to_string())),
            ("path ~ /src/", Selector::Path, CompOp::Matches, Value::String("/src/".to_string())),
            ("size > 1000", Selector::Size, CompOp::Greater, Value::Number(1000.0, None)),
            ("size >= 1MB", Selector::Size, CompOp::GreaterEqual, Value::Number(1.0, Some(SizeUnit::Megabytes))),
            ("type = file", Selector::Type, CompOp::Equal, Value::String("file".to_string())),
            ("ext != rs", Selector::Ext, CompOp::NotEqual, Value::String("rs".to_string())),
        ];

        for (input, expected_sel, expected_op, expected_val) in cases {
            let q = parse_query(input).unwrap();
            match q {
                Query::Expression(expr) => match expr.as_ref() {
                    Expression::Atom(Atom::Predicate(PredicateExpr::Comparison(sel, op, val))) => {
                        assert_eq!(sel, &expected_sel);
                        assert_eq!(op, &expected_op);
                        assert_eq!(val, &expected_val);
                    }
                    _ => panic!("Expected comparison predicate"),
                },
                _ => panic!("Expected expression"),
            }
        }
    }

    #[test]
    fn test_parse_property_predicates() {
        let cases = vec!["binary", "empty"];

        for input in cases {
            let q = parse_query(input).unwrap();
            match q {
                Query::Expression(expr) => match expr.as_ref() {
                    Expression::Atom(Atom::Predicate(PredicateExpr::Property(sel))) => {
                        match input {
                            "binary" => assert_eq!(sel, &Selector::Binary),
                            "empty" => assert_eq!(sel, &Selector::Empty),
                            _ => panic!("Unexpected property"),
                        }
                    }
                    _ => panic!("Expected property predicate"),
                },
                _ => panic!("Expected expression"),
            }
        }
    }

    #[test]
    fn test_parse_contains_expr() {
        let cases = vec![
            (r#"contains("TODO")"#, Pattern::Quoted("TODO".to_string())),
            ("contains(/unsafe/i)", Pattern::Regex("unsafe".to_string(), "i".to_string())),
        ];

        for (input, expected_pattern) in cases {
            let q = parse_query(input).unwrap();
            match q {
                Query::Expression(expr) => match expr.as_ref() {
                    Expression::Atom(Atom::Predicate(PredicateExpr::Contains(pattern))) => {
                        assert_eq!(pattern, &expected_pattern);
                    }
                    _ => panic!("Expected contains predicate"),
                },
                _ => panic!("Expected expression"),
            }
        }
    }

    #[test]
    fn test_parse_boolean_expressions() {
        // AND
        let q = parse_query("*.rs && size > 1000").unwrap();
        assert!(matches!(q, Query::Expression(ref expr) if matches!(expr.as_ref(), Expression::And(_, _))));

        // OR
        let q = parse_query("*.rs || *.go").unwrap();
        assert!(matches!(q, Query::Expression(ref expr) if matches!(expr.as_ref(), Expression::Or(_, _))));

        // NOT
        let q = parse_query("!hidden").unwrap();
        assert!(matches!(q, Query::Expression(ref expr) if matches!(expr.as_ref(), Expression::Not(_))));
    }

    #[test]
    fn test_operator_precedence() {
        // AND has higher precedence than OR
        let q = parse_query("a || b && c").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Or(left, right) => {
                    // Left should be 'a'
                    match left.as_ref() {
                        Expression::Atom(Atom::Query(Query::Implicit(Pattern::Bare(s)))) => assert_eq!(s, "a"),
                        _ => panic!("Expected 'a' on left"),
                    }
                    // Right should be 'b && c'
                    assert!(matches!(right.as_ref(), Expression::And(_, _)));
                }
                _ => panic!("Expected OR at top level"),
            },
            _ => panic!("Expected expression"),
        }

        // Parentheses override precedence
        let q = parse_query("(a || b) && c").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::And(left, right) => {
                    // Left should be '(a || b)'
                    assert!(matches!(left.as_ref(), Expression::Or(_, _)));
                    // Right should be 'c'
                    match right.as_ref() {
                        Expression::Atom(Atom::Query(Query::Implicit(Pattern::Bare(s)))) => assert_eq!(s, "c"),
                        _ => panic!("Expected 'c' on right"),
                    }
                }
                _ => panic!("Expected AND at top level"),
            },
            _ => panic!("Expected expression"),
        }

        // Complex precedence: a && b || c && d should be (a && b) || (c && d)
        let q = parse_query("a && b || c && d").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Or(left, right) => {
                    // Both sides should be AND expressions
                    assert!(matches!(left.as_ref(), Expression::And(_, _)));
                    assert!(matches!(right.as_ref(), Expression::And(_, _)));
                }
                _ => panic!("Expected OR at top level"),
            },
            _ => panic!("Expected expression"),
        }

        // NOT has highest precedence
        let q = parse_query("!a && b").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::And(left, right) => {
                    // Left should be '!a'
                    assert!(matches!(left.as_ref(), Expression::Not(_)));
                    // Right should be 'b'
                    match right.as_ref() {
                        Expression::Atom(Atom::Query(Query::Implicit(Pattern::Bare(s)))) => assert_eq!(s, "b"),
                        _ => panic!("Expected 'b' on right"),
                    }
                }
                _ => panic!("Expected AND at top level"),
            },
            _ => panic!("Expected expression"),
        }

        // Multiple NOTs
        let q = parse_query("!a || !b").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Or(left, right) => {
                    // Both should be NOT expressions
                    assert!(matches!(left.as_ref(), Expression::Not(_)));
                    assert!(matches!(right.as_ref(), Expression::Not(_)));
                }
                _ => panic!("Expected OR at top level"),
            },
            _ => panic!("Expected expression"),
        }

        // Nested parentheses
        let q = parse_query("((a || b) && (c || d)) || e").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Or(left, right) => {
                    // Left should be complex AND expression
                    assert!(matches!(left.as_ref(), Expression::And(_, _)));
                    // Right should be 'e'
                    match right.as_ref() {
                        Expression::Atom(Atom::Query(Query::Implicit(Pattern::Bare(s)))) => assert_eq!(s, "e"),
                        _ => panic!("Expected 'e' on right"),
                    }
                }
                _ => panic!("Expected OR at top level"),
            },
            _ => panic!("Expected expression"),
        }
    }

    #[test]
    fn test_complex_expressions() {
        // Nested expressions with filters - fixed syntax
        let q = parse_query("(*.rs >10KB || *.go >5KB) && !hidden");
        assert!(q.is_ok());

        // Multiple predicates - fixed with quotes
        let q = parse_query("name ~ \"test\" && size > 1000 && type = file");
        assert!(q.is_ok());

        // Mixed queries and predicates - fixed syntax
        let q = parse_query("(rust TODO || contains(/FIXME/)) && size < 1MB");
        assert!(q.is_ok());
    }

    #[test]
    fn test_parse_errors() {
        // Invalid syntax
        assert!(parse_query("&&").is_err());
        assert!(parse_query("||").is_err());
        assert!(parse_query("()").is_err());
        assert!(parse_query("name ==").is_err());
        assert!(parse_query("size >").is_err());
        
        // Unclosed quotes
        assert!(parse_query(r#""unclosed"#).is_err());
        
        // Invalid regex
        assert!(parse_query("/unclosed").is_err());
        
        // Invalid operators
        assert!(parse_query("size >> 100").is_err());
        
        // Missing parentheses
        assert!(parse_query("contains()").is_err());
        
        // Invalid time units
        assert!(parse_query("*.rs modified:10x").is_err());
        
        // Invalid size units
        assert!(parse_query("*.rs >10XB").is_err());
        
        // Malformed expressions
        assert!(parse_query("name == == value").is_err());
        assert!(parse_query("and and").is_err());
        assert!(parse_query("or or").is_err());
        
        // Missing operands
        assert!(parse_query("&&c").is_err());
        assert!(parse_query("a||").is_err());
        assert!(parse_query("!").is_err());
        
        // Invalid predicates
        assert!(parse_query("unknown_selector == value").is_err());
        
        // Unbalanced parentheses
        assert!(parse_query("(a && b").is_err());
        assert!(parse_query("a && b)").is_err());
        assert!(parse_query("((a && b)").is_err());
        
        // Invalid filter syntax
        assert!(parse_query("*.rs >").is_err());
        assert!(parse_query("*.rs modified:").is_err());
        assert!(parse_query("*.rs in:").is_err());
        
        // Empty expressions
        assert!(parse_query("").is_err());
        assert!(parse_query("   ").is_err());
    }

    #[test]
    fn test_edge_cases() {
        // Empty string in quotes
        let q = parse_query(r#""""#).unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Quoted(s)))) => {
                    assert_eq!(s, "");
                }
                _ => panic!("Expected empty quoted string"),
            },
            Query::Implicit(Pattern::Quoted(s)) => {
                assert_eq!(s, "");
            }
            _ => panic!("Expected expression or implicit"),
        }

        // Numbers with decimals
        let q = parse_query("size > 1.5MB").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Predicate(PredicateExpr::Comparison(_, _, Value::Number(v, _)))) => {
                    assert_eq!(v, &1.5);
                }
                _ => panic!("Expected size comparison"),
            },
            _ => panic!("Expected expression"),
        }

        // Path filter
        let q = parse_query("*.rs in:~/my-project_v2.0").unwrap();
        match q {
            Query::Filtered { filters, .. } => {
                assert!(matches!(&filters[0], Filter::Path(p) if p == "~/my-project_v2.0"));
            }
            _ => panic!("Expected filtered query"),
        }
    }

    #[test]
    fn test_parse_expr_integration() {
        // Test that parse_expr works and produces valid Expr type
        let expr = parse_expr("name = \"test.rs\" && size > 1000");
        assert!(expr.is_ok());

        let expr = parse_expr("*.rs || *.go");
        assert!(expr.is_ok());

        let expr = parse_expr("!hidden && executable");
        assert!(expr.is_ok());
    }

    #[test]
    fn test_parse_path_patterns() {
        // Test path patterns are recognized as glob
        let cases = vec![
            ("src/main.rs", Pattern::Glob("src/main.rs".to_string())),
            ("./test.txt", Pattern::Glob("./test.txt".to_string())),
        ];

        for (input, expected) in cases {
            let q = parse_query(input).unwrap();
            match q {
                Query::Expression(expr) => match expr.as_ref() {
                    Expression::Atom(Atom::Query(Query::Implicit(pattern))) => {
                        assert_eq!(pattern, &expected, "Failed for input: {}", input);
                    }
                    _ => panic!("Expected implicit query with pattern, got {:?}", expr),
                },
                Query::Implicit(pattern) => {
                    assert_eq!(pattern, expected, "Failed for input: {}", input);
                }
                _ => panic!("Expected expression or implicit, got {:?}", q),
            }
        }
    }

    #[test]
    fn test_mixed_queries() {
        // Test mixing different query types - fixed syntax
        let q = parse_query("(rust TODO || python FIXME) && size < 10KB").unwrap();
        assert!(matches!(q, Query::Expression(_)));

        // Test filters with expressions
        let q = parse_query("*.rs >100KB && (contains(/unsafe/) || contains(/unwrap/))").unwrap();
        assert!(matches!(q, Query::Expression(_)));

        // Test property selectors in expressions
        let q = parse_query("binary || (executable && size > 1MB)").unwrap();
        assert!(matches!(q, Query::Expression(_)));
    }

    #[test]
    fn test_whitespace_handling() {
        // Test that whitespace is properly handled
        let cases = vec![
            ("   TODO   ", "TODO"),
            ("*.rs    &&    *.go", "*.rs"),
            ("   name   ==   \"test\"   ", "name"),
        ];

        for (input, _) in cases {
            assert!(parse_query(input).is_ok(), "Failed to parse: {}", input);
        }
    }

    #[test]
    fn test_case_sensitivity() {
        // Predicates should be case-sensitive
        let q = parse_query("name = Test").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Predicate(PredicateExpr::Comparison(_, _, Value::String(s)))) => {
                    assert_eq!(s, "Test"); // Should preserve case
                }
                _ => panic!("Expected comparison predicate"),
            },
            _ => panic!("Expected expression"),
        }
    }
}