#[cfg(test)]
mod tests {
    use crate::expr::Expr;
    use crate::parser::parse_expr;
    use crate::predicate::{
        Bound, MetadataPredicate, NamePredicate, NumberMatcher, Predicate,
        StreamingCompiledContentPredicate, StringMatcher, TimeMatcher,
    };
    use chrono::{Duration, Local};
    use proptest::prelude::*;

    // Strategy for generating valid string values
    fn valid_string() -> impl Strategy<Value = String> {
        // Avoid backslashes in generated strings to prevent regex issues
        "[a-zA-Z0-9_./\\-+@#$%^&*~\\[\\]{}()|?,]*"
    }

    // Strategy for generating valid regex patterns
    fn valid_regex() -> impl Strategy<Value = String> {
        prop_oneof![
            Just(".*".to_string()),
            Just("^test.*$".to_string()),
            Just("[0-9]+".to_string()),
            Just("\\w+".to_string()),
            Just("foo.*bar".to_string()),
            Just("test[0-9]+".to_string()),
            Just("^[a-zA-Z]+$".to_string()),
            Just(".+\\.txt$".to_string()),
        ]
    }

    // Strategy for StringMatcher
    fn arb_string_matcher() -> impl Strategy<Value = StringMatcher> {
        prop_oneof![
            valid_regex().prop_filter_map("valid regex", |s| { StringMatcher::regex(&s).ok() }),
            valid_string().prop_map(StringMatcher::Equals),
            valid_string().prop_map(StringMatcher::NotEquals),
            valid_string().prop_map(StringMatcher::Contains),
            prop::collection::hash_set(valid_string(), 1..5).prop_map(StringMatcher::In),
        ]
    }

    // Strategy for Bound
    fn arb_bound() -> impl Strategy<Value = Bound> {
        prop_oneof![
            (0u64..1000, 1u64..1000).prop_map(|(start, len)| Bound::Full(start..start + len)),
            (0u64..1000).prop_map(|start| Bound::Left(start..)),
            (1u64..1000).prop_map(|end| Bound::Right(..end)),
        ]
    }

    // Strategy for NumberMatcher
    fn arb_number_matcher() -> impl Strategy<Value = NumberMatcher> {
        prop_oneof![
            arb_bound().prop_map(NumberMatcher::In),
            (0u64..1000000).prop_map(NumberMatcher::Equals),
            (0u64..1000000).prop_map(NumberMatcher::NotEquals),
        ]
    }

    // Strategy for TimeMatcher
    fn arb_time_matcher() -> impl Strategy<Value = TimeMatcher> {
        let base_time = Local::now();
        prop_oneof![
            (-365i64..365)
                .prop_map(move |days| TimeMatcher::Before(base_time + Duration::days(days))),
            (-365i64..365)
                .prop_map(move |days| TimeMatcher::After(base_time + Duration::days(days))),
            (-365i64..365)
                .prop_map(move |days| TimeMatcher::Equals(base_time + Duration::days(days))),
            (-365i64..365)
                .prop_map(move |days| TimeMatcher::NotEquals(base_time + Duration::days(days))),
        ]
    }

    // Strategy for NamePredicate
    fn arb_name_predicate() -> impl Strategy<Value = NamePredicate> {
        prop_oneof![
            arb_string_matcher().prop_map(NamePredicate::BaseName),
            arb_string_matcher().prop_map(NamePredicate::FileName),
            arb_string_matcher().prop_map(NamePredicate::DirPath),
            arb_string_matcher().prop_map(NamePredicate::FullPath),
            arb_string_matcher().prop_map(NamePredicate::Extension),
            arb_string_matcher().prop_map(NamePredicate::ParentDir),
        ]
    }

    // Strategy for MetadataPredicate
    fn arb_metadata_predicate() -> impl Strategy<Value = MetadataPredicate> {
        prop_oneof![
            arb_number_matcher().prop_map(MetadataPredicate::Filesize),
            arb_string_matcher().prop_map(MetadataPredicate::Type),
            arb_time_matcher().prop_map(MetadataPredicate::Modified),
            arb_time_matcher().prop_map(MetadataPredicate::Created),
            arb_time_matcher().prop_map(MetadataPredicate::Accessed),
            arb_number_matcher().prop_map(MetadataPredicate::Depth),
        ]
    }

    // Strategy for StreamingCompiledContentPredicate
    fn arb_content_predicate() -> impl Strategy<Value = StreamingCompiledContentPredicate> {
        valid_regex().prop_filter_map("valid content predicate", |s| {
            StreamingCompiledContentPredicate::new(s).ok()
        })
    }

    // Strategy for Predicate
    fn arb_predicate() -> impl Strategy<
        Value = Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>,
    > {
        prop_oneof![
            arb_name_predicate().prop_map(Predicate::name),
            arb_metadata_predicate().prop_map(Predicate::meta),
            arb_content_predicate().prop_map(Predicate::contents),
        ]
    }

    // Strategy for Expr - using idiomatic prop_recursive
    fn arb_expr() -> impl Strategy<
        Value = Expr<
            Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>,
        >,
    > {
        // Define the leaf (non-recursive) strategy
        let leaf = arb_predicate().prop_map(Expr::Predicate);

        // Use prop_recursive to handle recursive generation safely
        leaf.prop_recursive(
            8,   // depth: up to 8 levels deep
            256, // desired_size: target ~256 nodes maximum
            10,  // expected_branch_size: tuning parameter
            |inner| {
                // Define recursive cases using the inner strategy
                prop_oneof![
                    // Unary operator - single recursive child
                    2 => inner.clone()
                        .prop_map(|e| Expr::Not(Box::new(e))),

                    // Binary operators - two recursive children
                    1 => (inner.clone(), inner.clone())
                        .prop_map(|(a, b)| Expr::And(Box::new(a), Box::new(b))),

                    1 => (inner.clone(), inner.clone())
                        .prop_map(|(a, b)| Expr::Or(Box::new(a), Box::new(b))),
                ]
            },
        )
    }

    #[test]
    fn test_basic_display() {
        let pred = NamePredicate::FileName(StringMatcher::Equals("test.txt".to_string()));
        let expr: Expr<
            Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>,
        > = Expr::Predicate(Predicate::name(pred));
        let s = expr.to_string();
        println!("Basic display: {}", s);
        assert!(s.contains("path.name"));
    }

    #[test]
    fn test_deep_nesting() {
        let pred = NamePredicate::FileName(StringMatcher::Equals("test.txt".to_string()));
        let base: Expr<
            Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>,
        > = Expr::Predicate(Predicate::name(pred));

        // Create deeply nested expression
        let mut expr = base;
        for i in 0..5 {
            expr = Expr::Not(Box::new(expr));
            println!("Depth {}: {}", i + 1, expr);
        }
    }

    #[test]
    fn test_generation() {
        use proptest::test_runner::{Config, TestRunner};

        let mut runner = TestRunner::new(Config {
            cases: 5,
            max_shrink_iters: 0,
            ..Config::default()
        });

        println!("Testing generation...");
        let result = runner.run(&arb_expr(), |expr| {
            println!("Generated: {:?}", expr);
            Ok(())
        });

        match result {
            Ok(_) => println!("Generation successful"),
            Err(e) => panic!("Generation failed: {:?}", e),
        }
    }

    #[test]
    fn test_simple_round_trip() {
        // Test a simple expression round trip
        let pred = NamePredicate::FileName(StringMatcher::Equals("test.txt".to_string()));
        let expr: Expr<
            Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>,
        > = Expr::Predicate(Predicate::name(pred));

        let expr_str = expr.to_string();
        println!("Expression string: {}", expr_str);

        match parse_expr(&expr_str) {
            Ok(parsed) => {
                println!("Parsed successfully");
                println!("Parsed expr: {:?}", parsed);
            }
            Err(e) => {
                panic!("Failed to parse '{}': {:?}", expr_str, e);
            }
        }
    }

    #[test]
    fn test_expr_generation_only() {
        use proptest::test_runner::{Config, TestRunner};

        let mut runner = TestRunner::new(Config {
            cases: 10,
            max_shrink_iters: 0,
            ..Config::default()
        });

        println!("Testing expression generation at depth 3...");
        let result = runner.run(&arb_expr(), |expr| {
            // Just generate, don't convert to string yet
            println!("Generated expression (depth analysis):");
            analyze_depth(&expr, 0);
            Ok(())
        });

        match result {
            Ok(_) => println!("Generation successful"),
            Err(e) => panic!("Generation failed: {:?}", e),
        }
    }

    fn analyze_depth<T>(expr: &Expr<T>, depth: usize) {
        let indent = "  ".repeat(depth);
        match expr {
            Expr::Not(e) => {
                println!("{}Not", indent);
                analyze_depth(e, depth + 1);
            }
            Expr::And(a, b) => {
                println!("{}And", indent);
                analyze_depth(a, depth + 1);
                analyze_depth(b, depth + 1);
            }
            Expr::Or(a, b) => {
                println!("{}Or", indent);
                analyze_depth(a, depth + 1);
                analyze_depth(b, depth + 1);
            }
            Expr::Predicate(_) => println!("{}Predicate", indent),
            Expr::Literal(_) => println!("{}Literal", indent),
        }
    }

    proptest! {
        #[test]
        fn test_expr_round_trip(expr in arb_expr()) {
            // Convert expression to string
            let expr_str = expr.to_string();

            // Parse it back
            match parse_expr(&expr_str) {
                Ok(_parsed_expr) => {
                    // For now, just check that parsing succeeded
                    // Full equality check would require normalizing expressions
                    prop_assert!(true);
                }
                Err(e) => {
                    // If parsing failed, print debug info
                    // eprintln!("Failed to parse generated expression:");
                    // eprintln!("  Expression: {}", expr_str);
                    // eprintln!("  Error: {}", e);
                    prop_assert!(false, "Failed to parse {expr:?} -> `{expr_str}`: {}", e);
                }
            }
        }

        #[test]
        fn test_predicate_display_parse(pred in arb_predicate()) {
            // Convert predicate to string
            let pred_str = pred.to_string();

            // Try to parse it directly as an expression
            match parse_expr(&pred_str) {
                Ok(_) => prop_assert!(true),
                Err(e) => {
                    // eprintln!("Failed to parse generated predicate:");
                    // eprintln!("  Predicate: {}", pred_str);
                    // eprintln!("  Error: {}", e);
                    prop_assert!(false, "Failed to parse: {}", e);
                }
            }
        }
    }
}
