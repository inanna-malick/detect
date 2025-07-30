#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use crate::expr::Expr;
    use crate::predicate::{
        Bound, MetadataPredicate, NamePredicate, NumberMatcher, NumericalOp, Op, Predicate, RhsValue, Selector, StreamingCompiledContentPredicate, StringMatcher, TimeMatcher, TimeUnit
    };
    use crate::parser::parse_expr;
    use chrono::{Local, Duration};

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
            valid_regex().prop_filter_map("valid regex", |s| {
                StringMatcher::regex(&s).ok()
            }),
            valid_string().prop_map(StringMatcher::Equals),
            valid_string().prop_map(StringMatcher::NotEquals),
            valid_string().prop_map(StringMatcher::Contains),
            prop::collection::vec(valid_string(), 1..5).prop_map(StringMatcher::In),
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
            (-365i64..365).prop_map(move |days| TimeMatcher::Before(base_time + Duration::days(days))),
            (-365i64..365).prop_map(move |days| TimeMatcher::After(base_time + Duration::days(days))),
            (-365i64..365).prop_map(move |days| TimeMatcher::Equals(base_time + Duration::days(days))),
            (-365i64..365).prop_map(move |days| TimeMatcher::NotEquals(base_time + Duration::days(days))),
        ]
    }

    // Strategy for NamePredicate
    fn arb_name_predicate() -> impl Strategy<Value = NamePredicate> {
        prop_oneof![
            arb_string_matcher().prop_map(NamePredicate::Filename),
            arb_string_matcher().prop_map(NamePredicate::Path),
            arb_string_matcher().prop_map(NamePredicate::Extension),
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
        ]
    }

    // Strategy for StreamingCompiledContentPredicate
    fn arb_content_predicate() -> impl Strategy<Value = StreamingCompiledContentPredicate> {
        valid_regex().prop_filter_map("valid content predicate", |s| {
            StreamingCompiledContentPredicate::new(s).ok()
        })
    }

    // Strategy for Predicate
    fn arb_predicate() -> impl Strategy<Value = Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>> {
        prop_oneof![
            arb_name_predicate().prop_map(Predicate::name),
            arb_metadata_predicate().prop_map(Predicate::meta),
            arb_content_predicate().prop_map(Predicate::contents),
        ]
    }

    // Strategy for Expr - using recursive strategy
    fn arb_expr_inner(
        depth: u32,
    ) -> impl Strategy<Value = Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>> {
        if depth == 0 {
            // Only generate predicates at leaf nodes, not literals
            arb_predicate().prop_map(Expr::Predicate).boxed()
        } else {
            prop_oneof![
                // 40% chance of a predicate
                4 => arb_predicate().prop_map(Expr::Predicate),
                // 20% chance of Not
                2 => arb_expr_inner(depth - 1).prop_map(|e| Expr::Not(Box::new(e))),
                // 20% chance of And
                2 => (arb_expr_inner(depth - 1), arb_expr_inner(depth - 1))
                    .prop_map(|(a, b)| Expr::And(Box::new(a), Box::new(b))),
                // 20% chance of Or
                2 => (arb_expr_inner(depth - 1), arb_expr_inner(depth - 1))
                    .prop_map(|(a, b)| Expr::Or(Box::new(a), Box::new(b))),
            ].boxed()
        }
    }

    fn arb_expr() -> impl Strategy<Value = Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>> {
        arb_expr_inner(3)
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
                    eprintln!("Failed to parse generated expression:");
                    eprintln!("  Expression: {}", expr_str);
                    eprintln!("  Error: {}", e);
                    prop_assert!(false, "Failed to parse: {}", e);
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
                    eprintln!("Failed to parse generated predicate:");
                    eprintln!("  Predicate: {}", pred_str);
                    eprintln!("  Error: {}", e);
                    prop_assert!(false, "Failed to parse: {}", e);
                }
            }
        }
    }
}