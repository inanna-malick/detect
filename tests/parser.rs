use detect::predicate::{NamePredicate, StringMatcher};
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

    let examples: Vec<(&'static str, _)> = vec![
        (
            "@name == foo && @path ~= bar",
            Expr::and( filename("foo".to_string()), filepath("bar")),
        ),
        (
            // test confirming that '&&' binds more tightly than ||, a || (b && c)
            "@filename == a || @filepath ~= b && @filepath ~= c",
            Expr::or(filename("a".to_string()), Expr::and(filepath("b"), filepath("c"))),
        ),
        // test fails, and is outer binding - c && (b || a)
        // (
        //     // test confirming that '&&' binds more tightly than || in reverse order
        //     "@filepath ~= c && @filepath ~= b || @filename == a",
        //     Expr::or( Expr::and(filepath("c"), filepath("b")), filename("a".to_string()),),
        // ),
    ];

    for (input, expected) in examples.into_iter() {
        assert_eq!(detect::parse(input).unwrap(), expected)
    }
}
