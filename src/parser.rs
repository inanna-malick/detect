// #[macro_use]
// extern crate combine;
// use crate::{expr::*, operator::Operator};
use combine::error::{ParseError, StdParseResult};
use combine::parser::char::{char, letter, spaces, string};
use combine::stream::position;
use combine::stream::{Positioned, Stream};
use combine::{
    attempt, between, choice, easy, many1, parser, sep_by, sep_by1, token, EasyParser, Parser,
};

use combine_language::{expression_parser, Assoc, Fixity};

#[derive(Debug, PartialEq)]
pub enum Expr {
    Id(String), // replace this with terminating predicate thingies
    Array(Vec<Expr>),
    Parens(Box<Expr>),
    Pair(Box<Expr>, Box<Expr>),
    And(Vec<Expr>),
    Or(Vec<Expr>),
}

fn and_<Input>() -> impl Parser<Input, Output = Expr>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let skip_spaces = || spaces().silent();
    sep_by1(base().skip(skip_spaces()), string("&&").skip(skip_spaces())).map(|mut xs: Vec<_>| {
        if xs.len() > 1 {
            Expr::And(xs)
        } else {
            xs.pop().unwrap()
        }
    })
}

fn or_<Input>() -> impl Parser<Input, Output = Expr>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let skip_spaces = || spaces().silent();
    sep_by1(and().skip(skip_spaces()), string("||").skip(skip_spaces()))
        .map(|xs| Expr::Or(xs))
        .skip(skip_spaces())
}

// `impl Parser` can be used to create reusable parsers with zero overhead
fn base_<Input>() -> impl Parser<Input, Output = Expr>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let word = many1(letter());

    // A parser which skips past whitespace.
    // Since we aren't interested in knowing that our expression parser
    // could have accepted additional whitespace between the tokens we also silence the error.
    let skip_spaces = || spaces().silent();
    let lex_char = |c| char(c).skip(skip_spaces());

    let parens = (lex_char('('), or(), lex_char(')')).map(|(_, e, _)| e);

    // I don't think order matters here, inside the choice combinator? idk
    choice((
        // attempt(and_operator),
        word.map(Expr::Id),
        // array.map(Expr::Array),
        parens,
        // attempt(pair),
        // attempt(binop),
    ))
    .skip(skip_spaces())
}

// base expr: choice between predicates and parens (recursing back to or) WITH OR
// and expr: not a choice - sep_by_1 w/ base expr
// or expr: not a choice - sep_by_1 w/ and expr

// mutually recursive ordering defines precedence

parser! {
    fn and[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        and_()
    }
}

parser! {
    fn or[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        or_()
    }
}

parser! {
    fn base[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        base_()
    }
}

#[test]
fn test_test_test() {
    let result: Result<_, easy::ParseError<&str>> =
        // or().easy_parse("");
        or().easy_parse("foo && bar && (foo || bar) && (foo && (bar || bar))");
        // or().easy_parse("a || b");

    let expr = Expr::Array(vec![
        Expr::Array(Vec::new()),
        Expr::Pair(
            Box::new(Expr::Id("hello".to_string())),
            Box::new(Expr::Id("world".to_string())),
        ),
        Expr::Array(vec![Expr::Id("rust".to_string())]),
    ]);
    assert_eq!(result.map(|x| x.0), Ok(expr));
}

// fn mt(e: Expr<ExprTree>) -> ExprTree {
//     ExprTree {
//         fs_ref: Box::new(e),
//     }
// }
