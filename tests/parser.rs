use detect::predicate::NamePredicate;
use regex::Regex;

#[test]
fn test_parser() {
    use detect::expr::Expr;

    let filepath = |s| Expr::name_predicate(NamePredicate::Path(Regex::new(s).unwrap()));
    let filename = |s| Expr::name_predicate(NamePredicate::Filename(Regex::new(s).unwrap()));

    let examples: Vec<(&'static str, _)> = vec![
        (
            "filename(foo) && filepath(bar)",
            // note: order is reversed by parser? eh that's fine (TODO: or is it?)
            Expr::and(
                filepath("bar"),
                Expr::name_predicate(NamePredicate::Filename(Regex::new("foo").unwrap())),
            ),
        ),
        (
            // test confirming that '&&' binds more tightly than ||
            "filename(foo) || filepath(bar) && filepath(baz)",
            Expr::or(Expr::and(filepath("baz"), filepath("bar")), filename("foo")),
        ),
        (
            // test confirming that '&&' binds more tightly than || in reverse order
            "filepath(bar) && filepath(baz) || filename(foo)",
            Expr::or(filename("foo"), Expr::and(filepath("baz"), filepath("bar"))),
        ),
        // TODO: test for ! operator binding I guess?
    ];

    for (input, expected) in examples.into_iter() {
        assert_eq!(detect::parse(input).unwrap(), expected)
    }
}
