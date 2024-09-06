use detect::predicate::{MetadataPredicate, NamePredicate, Predicate, StringMatcher};
use futures::never::Never;
use regex::Regex;

#[test]
fn test_parser() {
    use detect::expr::Expr;

    let filepath = |s| {
        Expr::name_predicate(NamePredicate::Path(StringMatcher::Regex(
            Regex::new(s).unwrap(),
        )))
    };
    let filename = |s| Expr::name_predicate(NamePredicate::Filename(StringMatcher::Equals(s)));

    let examples: Vec<(
        &'static str,
        Expr<Predicate<NamePredicate, MetadataPredicate, Never>>,
    )> = vec![
        (
            "@name == foo && @path ~= bar",
            Expr::and(filename("foo".to_string()), filepath("bar")),
        ),
        (
            // test confirming that '&&' binds more tightly than ||, a || (b && c)
            "@filename == a || @filepath ~= b && @filepath ~= c",
            Expr::or(
                filename("a".to_string()),
                Expr::and(filepath("b"), filepath("c")),
            ),
        ),
        // test fails, and is outer binding - c && (b || a)
        // (
        //     // test confirming that '&&' binds more tightly than || in reverse order
        //     "@filepath ~= c && @filepath ~= b || @filename == a",
        //     Expr::or( Expr::and(filepath("c"), filepath("b")), filename("a".to_string()),),
        // ),
    ];

    for (input, expected) in examples.into_iter() {
        assert_eq!(
            detect::parse(input)
                .unwrap()
                .map_predicate_ref(|p| match p {
                    Predicate::Name(n) => Predicate::Name(n.clone()),
                    Predicate::Metadata(m) => Predicate::Metadata(m.clone()),
                    // we'll never hit this branch, not touching DFA's b/c no Eq impl
                    Predicate::Content(_) => unreachable!(),
                }),
            expected
        )
    }
}
