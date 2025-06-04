#[cfg(test)]
mod llm_syntax_tests {
    use detect::parser::parse_query;
    use detect::query::*;

    // ============================================================================
    // LEVEL 1: Simple Patterns (90% of use cases)
    // ============================================================================

    #[test]
    fn test_bare_words_search_content() {
        // Bare words should search file contents
        let cases = vec!["TODO", "FIXME", "DEBUG", "hello", "test123"];

        for case in cases {
            let result = parse_query(case);
            assert!(result.is_ok(), "Failed to parse bare word: {}", case);

            // TODO: Verify it creates a content search predicate
        }
    }

    #[test]
    fn test_glob_patterns_search_filenames() {
        // Glob patterns should search filenames
        let cases = vec![
            "*.rs",
            "*.py",
            "src/**/*.js",
            "test_*.txt",
            "foo?.rs",
            "[abc].txt",
            "{foo,bar}.rs",
        ];

        for case in cases {
            let result = parse_query(case);
            assert!(result.is_ok(), "Failed to parse glob pattern: {}", case);

            // TODO: Verify it creates a filename search predicate
        }
    }

    #[test]
    fn test_quoted_strings_exact_match() {
        // Quoted strings for exact filename matches
        let cases = vec![
            r#""hello world.txt""#,
            r#""README.md""#,
            r#""file with spaces.rs""#,
        ];

        for case in cases {
            let result = parse_query(case);
            assert!(result.is_ok(), "Failed to parse quoted string: {}", case);

            // TODO: Verify it creates an exact filename match
        }
    }

    #[test]
    fn test_regex_patterns() {
        // Regex patterns for advanced matching
        let cases = vec![
            "/TODO.*urgent/",
            "/TODO.*urgent/i",
            r"/\bFIXME\b/",
            "/test_\\d+/",
            "/^import.*from/m",
        ];

        for case in cases {
            let result = parse_query(case);
            assert!(result.is_ok(), "Failed to parse regex pattern: {}", case);

            // TODO: Verify it creates a regex content search
        }
    }

    // ============================================================================
    // LEVEL 2: Filtered Searches (9% of use cases)
    // ============================================================================

    #[test]
    fn test_file_type_shortcuts() {
        // File type shortcuts
        let cases = vec![
            ("rust", FileType::Rust),
            ("python", FileType::Python),
            ("javascript", FileType::JavaScript),
            ("typescript", FileType::TypeScript),
            ("go", FileType::Go),
            ("java", FileType::Java),
            ("c", FileType::C),
            ("cpp", FileType::Cpp),
        ];

        for (input, expected_type) in cases {
            let result = parse_query(input);
            assert!(result.is_ok(), "Failed to parse file type: {}", input);

            let query = result.unwrap();
            println!("Parsed '{}' to: {:?}", input, query);
            match query {
                Query::Filtered { base, .. } => match base {
                    FilterBase::Type(file_type) => {
                        assert_eq!(file_type, expected_type, "Wrong file type for {}", input);
                    }
                    _ => panic!("Expected file type filter for {}", input),
                },
                _ => panic!("Expected filtered query for {}, got {:?}", input, query),
            }
        }
    }

    #[test]
    fn test_file_type_with_content() {
        // File type + content search
        let cases = vec!["rust TODO", "python FIXME", "javascript console.log"];

        for case in cases {
            let result = parse_query(case);
            assert!(result.is_ok(), "Failed to parse type + content: {}", case);

            // TODO: Verify it combines file type and content search
        }
    }

    #[test]
    fn test_size_filters() {
        // Size filters with human-friendly units
        let cases = vec![
            "*.rs >1MB",
            "*.log <100KB",
            "image >5MB",
            "* >=10KB",
            "*.txt <=1GB",
            "rust >0", // bytes
        ];

        for case in cases {
            let result = parse_query(case);
            assert!(result.is_ok(), "Failed to parse size filter: {}", case);

            // TODO: Verify size predicate with correct value and unit
        }
    }

    #[test]
    fn test_time_filters() {
        // Time filters - need a base pattern/type
        let cases = vec![
            "* modified:today",
            "* modified:yesterday",
            "* modified:1d",
            "* modified:2w",
            "* modified:1m",
            "*.log m:1d",
            "* created:7d",
            "* accessed:1h",
        ];

        for case in cases {
            let result = parse_query(case);
            assert!(result.is_ok(), "Failed to parse time filter: {}", case);

            // TODO: Verify time predicate
        }
    }

    #[test]
    fn test_path_filters() {
        // Path filters
        let cases = vec![
            "*.py in:src",
            "TODO dir:tests",
            "*.rs in:src/lib",
            "*.txt path:foo/bar",
        ];

        for case in cases {
            println!("Parsing path filter: '{}'", case);
            let result = parse_query(case);
            match &result {
                Ok(query) => println!("Parsed to: {:?}", query),
                Err(e) => println!("Parse error: {}", e),
            }
            assert!(result.is_ok(), "Failed to parse path filter: {}", case);

            // TODO: Verify path constraint
        }
    }

    #[test]
    fn test_property_filters() {
        // Property filters
        let cases = vec!["executable", "hidden", "empty", "binary"];

        for case in cases {
            let result = parse_query(case);
            assert!(result.is_ok(), "Failed to parse property filter: {}", case);

            // TODO: Verify property predicate
        }
    }

    #[test]
    fn test_combined_filters() {
        // Multiple filters
        let cases = vec![
            "*.rs >1MB modified:today",
            "python TODO",       // This is type + pattern, not filters
            "executable",        // This is a property filter
            "*.log hidden m:1d", // Pattern with property and time filter
        ];

        for case in cases {
            println!("Parsing: '{}'", case);
            let result = parse_query(case);
            match &result {
                Ok(q) => println!("Parsed to: {:?}", q),
                Err(e) => println!("Error: {}", e),
            }
            assert!(result.is_ok(), "Failed to parse combined filters: {}", case);

            // TODO: Verify all filters are applied
        }
    }

    // ============================================================================
    // LEVEL 3: Full Expressions (1% of use cases)
    // ============================================================================

    #[test]
    fn test_boolean_and() {
        // Boolean AND
        let cases = vec!["*.rs && TODO", "hidden && empty", "rust && size > 1000"];

        for case in cases {
            let result = parse_query(case);
            assert!(result.is_ok(), "Failed to parse AND expression: {}", case);

            // TODO: Verify AND expression structure
        }
    }

    #[test]
    fn test_boolean_or() {
        // Boolean OR
        let cases = vec!["*.rs || *.go", "hidden || empty", "TODO || FIXME"];

        for case in cases {
            let result = parse_query(case);
            assert!(result.is_ok(), "Failed to parse OR expression: {}", case);

            // TODO: Verify OR expression structure
        }
    }

    #[test]
    fn test_boolean_not() {
        // Boolean NOT
        let cases = vec!["!binary", "!hidden", "!(*.rs)"];

        for case in cases {
            let result = parse_query(case);
            assert!(result.is_ok(), "Failed to parse NOT expression: {}", case);

            // TODO: Verify NOT expression structure
        }
    }

    #[test]
    fn test_complex_expressions() {
        // Complex queries with parentheses
        let cases = vec![
            "(*.rs || *.go) && size > 1MB",
            "name = \"test.rs\" || contains(/test/)",
            "(rust || go) && !empty",
            "hidden && (*.log || *.tmp)",
        ];

        for case in cases {
            println!("Testing complex expression: '{}'", case);
            let result = parse_query(case);
            match &result {
                Ok(q) => println!("Parsed to: {:?}", q),
                Err(e) => println!("Error: {}", e),
            }
            assert!(
                result.is_ok(),
                "Failed to parse complex expression: {}",
                case
            );

            // TODO: Verify expression structure
        }
    }

    #[test]
    fn test_predicates() {
        // Various predicates
        let cases = vec![
            "size > 1000",
            "size >= 1KB",
            "lines < 100",
            "lines <= 500",
            "ext = rs",
            "ext = \"rs\"",
            "name = test.rs",
            "name = \"test.rs\"",
            "type = file",
            "type = dir",
            "path ~ /src/",
            "contains(/unsafe/)",
            "contains(\"TODO\")",
        ];

        for case in cases {
            println!("Testing predicate: '{}'", case);
            let result = parse_query(case);
            match &result {
                Ok(q) => println!("Parsed to: {:?}", q),
                Err(e) => println!("Error: {}", e),
            }
            assert!(result.is_ok(), "Failed to parse predicate: {}", case);

            // TODO: Verify predicate structure
        }
    }

    #[test]
    fn test_mixed_predicates_and_patterns() {
        // Mixing predicates with patterns
        let cases = vec![
            "ext = rs && !contains(/unsafe/)",
            "size > 1000 && lines < 100",
            "*.test.js && contains(/describe/)",
            "python && size < 10KB && TODO",
        ];

        for case in cases {
            let result = parse_query(case);
            assert!(result.is_ok(), "Failed to parse mixed expression: {}", case);

            // TODO: Verify expression combines predicates and patterns correctly
        }
    }

    // ============================================================================
    // ERROR CASES - These should fail gracefully
    // ============================================================================

    #[test]
    fn test_invalid_syntax() {
        let cases = vec![
            "&&",
            "||",
            "*()",
            "size >",
            "ext =",
            "contains()",
            "modified:",
        ];

        for case in cases {
            let result = parse_query(case);
            assert!(
                result.is_err(),
                "Should have failed to parse invalid syntax: {}",
                case
            );
        }
    }

    // ============================================================================
    // INTEGRATION - Test actual file finding behavior
    // ============================================================================

    #[test]
    #[ignore] // These require actual file system setup
    fn test_integration_bare_word() {
        // Test that "TODO" actually finds files containing TODO
        // This would require setting up test files and running the full pipeline
    }

    #[test]
    #[ignore]
    fn test_integration_glob_pattern() {
        // Test that "*.rs" actually finds Rust files
    }

    #[test]
    #[ignore]
    fn test_integration_complex_query() {
        // Test that "rust TODO && size > 1KB" works end-to-end
    }
}
