use detect::{parser::Token, predicate::{MetadataPredicate, NamePredicate, Predicate, StringMatcher}};
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
        (
            // test confirming that '&&' binds more tightly than || in reverse order
            "@filepath ~= c && (@filepath ~= b || @filename == a)",
            Expr::and(filepath("c"), Expr::or(filepath("b"), filename("a".to_string()),)),
        ),
        (
            // test confirming that '&&' binds more tightly than || in reverse order
            "@filepath ~= c && @filepath ~= b || @filename == a",
            Expr::or( Expr::and(filepath("c"), filepath("b")), filename("a".to_string()),),
        ),
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
            expected,
            "parser failed for {}", input
        )
    }
}


#[test]
fn test_parser_simple() {

    let filepath = |s| {
        Token::Predicate(Predicate::name(NamePredicate::Path(StringMatcher::Regex(
            Regex::new(s).unwrap(),
        ))))
    };

    let filename = |s: &str| Token::Predicate(Predicate::name(NamePredicate::Filename(StringMatcher::Equals(s.to_string()))));

    let examples: Vec<(
        &'static str,
        Vec<Token>,
    )> = {
        use Token::*;
        vec![
        (
            "@name == foo && @path ~= bar",
            vec![filename("foo"), And, filepath("bar")],
        ),
        (
            // test confirming that '&&' binds more tightly than ||, a || (b && c)
            "@filename == a || @filepath ~= b && @filepath ~= c",
            vec![filename("a"), Or, filepath("b"), And, filepath("c")],
        ),
        (
            // test confirming that '&&' binds more tightly than || in reverse order
            "@filepath ~= c && (@filepath ~= b || @filename == a)",
            vec![filepath("c"), And, OpenParen, filepath("b"), Or, filename("a"), CloseParen],
        ),
        (
            // test confirming that '&&' binds more tightly than || in reverse order
            "@filepath ~= c && @filepath ~= b || @filename == a",
            vec![filepath("c"), And, filepath("b"), Or, filename("a")],
        ),
    ]};

    for (input, expected) in examples.into_iter() {
        assert_eq!(
            detect::parse_tokens(input)
                .unwrap(),
            expected,
            "parser failed for {}", input
        )
    }
}