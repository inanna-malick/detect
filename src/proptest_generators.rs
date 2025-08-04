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

    fn valid_string() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_./\\-+@#$%^&*~\\[\\]{}()|?,]*"
    }

    fn valid_regex() -> impl Strategy<Value = String> {
        prop_oneof![
            Just(".*".to_string()),
            Just("^test.*$".to_string()),
            Just("[0-9]+".to_string()),
            Just("[a-zA-Z0-9]+".to_string()),
            Just("foo.*bar".to_string()),
            Just("test[0-9]+".to_string()),
            Just("^[a-zA-Z]+$".to_string()),
            Just(".+txt$".to_string()),
        ]
    }

    fn arb_string_matcher() -> impl Strategy<Value = StringMatcher> {
        prop_oneof![
            valid_regex().prop_filter_map("valid regex", |s| { StringMatcher::regex(&s).ok() }),
            valid_string().prop_map(StringMatcher::Equals),
            valid_string().prop_map(StringMatcher::NotEquals),
            valid_string().prop_map(StringMatcher::Contains),
            prop::collection::hash_set(valid_string(), 1..5).prop_map(StringMatcher::In),
        ]
    }

    fn arb_bound() -> impl Strategy<Value = Bound> {
        prop_oneof![
            (0u64..1000).prop_map(|start| Bound::Left(start..)),
            (1u64..1000).prop_map(|end| Bound::Right(..end)),
        ]
    }

    fn arb_number_matcher() -> impl Strategy<Value = NumberMatcher> {
        prop_oneof![
            arb_bound().prop_map(NumberMatcher::In),
            (0u64..1000000).prop_map(NumberMatcher::Equals),
            (0u64..1000000).prop_map(NumberMatcher::NotEquals),
        ]
    }

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

    fn arb_content_predicate() -> impl Strategy<Value = StreamingCompiledContentPredicate> {
        valid_regex().prop_filter_map("valid content predicate", |s| {
            StreamingCompiledContentPredicate::new(s).ok()
        })
    }

    fn arb_predicate() -> impl Strategy<
        Value = Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>,
    > {
        prop_oneof![
            arb_name_predicate().prop_map(Predicate::name),
            arb_metadata_predicate().prop_map(Predicate::meta),
            arb_content_predicate().prop_map(Predicate::contents),
        ]
    }

    fn arb_expr() -> impl Strategy<
        Value = Expr<
            Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>,
        >,
    > {
        let leaf = arb_predicate().prop_map(Expr::Predicate);

        leaf.prop_recursive(
            8,   // depth: up to 8 levels deep
            256, // desired_size: target ~256 nodes maximum
            10,  // expected_branch_size: tuning parameter
            |inner| {
                prop_oneof![
                    2 => inner.clone()
                        .prop_map(|e| Expr::negate(e)),

                    1 => (inner.clone(), inner.clone())
                        .prop_map(|(a, b)| Expr::and(a, b)),

                    1 => (inner.clone(), inner.clone())
                        .prop_map(|(a, b)| Expr::or(a, b)),
                ]
            },
        )
    }

    proptest! {
        #[test]
        fn test_expr_round_trip(expr in arb_expr()) {
            let expr_str = expr.to_string();

            match parse_expr(&expr_str) {
                Ok(parsed_expr) => {
                    prop_assert_eq!(parsed_expr, expr)
                }
                Err(e) => {
                    prop_assert!(false, "Failed to parse {expr:?} -> `{expr_str}`: {}", e);
                }
            }
        }
    }
}
