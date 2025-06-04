#[cfg(test)]
mod tests {
    use detect::parser::parse_query;
    use detect::query::*;

    #[test]
    fn test_implicit_searches() {
        // For single patterns, they get wrapped in Expression(Atom(Query(Implicit(...))))
        // This is expected behavior from the grammar
        
        // Bare word
        let q = parse_query("TODO").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Bare(s)))) => {
                    assert_eq!(s, "TODO");
                }
                _ => panic!("Expected bare word pattern, got {:?}", expr),
            },
            _ => panic!("Expected expression, got {:?}", q),
        }

        // Glob pattern
        let q = parse_query("*.rs").unwrap();
        match q {
            Query::Expression(expr) => match expr.as_ref() {
                Expression::Atom(Atom::Query(Query::Implicit(Pattern::Glob(s)))) => {
                    assert_eq!(s, "*.rs");
                }
                _ => panic!("Expected glob pattern, got {:?}", expr),
            },
            _ => panic!("Expected expression, got {:?}", q),
        }
    }

    #[test]
    fn test_filtered_searches() {
        // Type with size filter
        let q = parse_query("rust >1MB").unwrap();
        match q {
            Query::Filtered { base, filters } => {
                assert!(matches!(base, FilterBase::Type(FileType::Rust)));
                assert_eq!(filters.len(), 1);
                assert!(matches!(&filters[0], Filter::Size(SizeOp::Greater, v, SizeUnit::Megabytes) if *v == 1.0));
            }
            _ => panic!("Expected filtered query"),
        }

        // Pattern with multiple filters
        let q = parse_query("*.rs >100KB modified:today").unwrap();
        match q {
            Query::Filtered { base, filters } => {
                assert!(matches!(base, FilterBase::Pattern(Pattern::Glob(s)) if s == "*.rs"));
                assert_eq!(filters.len(), 2);
            }
            _ => panic!("Expected filtered query"),
        }
    }

    #[test]
    fn test_expressions() {
        // Simple AND
        let q = parse_query("*.rs && TODO").unwrap();
        match q {
            Query::Expression(expr) => {
                assert!(matches!(expr.as_ref(), Expression::And(_, _)));
            }
            _ => panic!("Expected expression"),
        }

        // With predicates
        let q = parse_query("name == \"test.rs\" or size > 1000").unwrap();
        match q {
            Query::Expression(expr) => {
                assert!(matches!(expr.as_ref(), Expression::Or(_, _)));
            }
            _ => panic!("Expected expression"),
        }
    }

    #[test]
    fn test_progressive_examples() {
        // Level 1: Simple patterns
        assert!(parse_query("README").is_ok());
        assert!(parse_query("*.md").is_ok());
        assert!(parse_query("/TODO/i").is_ok());

        // Level 2: Filters
        assert!(parse_query("rust TODO").is_ok());
        assert!(parse_query("*.py >10KB").is_ok());
        assert!(parse_query("image in:src").is_ok());

        // Level 3: Expressions
        assert!(parse_query("(*.rs or *.go) and size > 1MB").is_ok());
        assert!(parse_query("hidden or empty").is_ok());
        assert!(parse_query("contains(/unsafe/) and rust").is_ok());
    }
}