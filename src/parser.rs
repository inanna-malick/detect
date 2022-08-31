// #[macro_use]
// extern crate combine;
// use crate::{expr::*, operator::Operator};
use combine::error::{ParseError, StdParseResult, UnexpectedParse};
use combine::parser::char::{char, digit, letter, spaces, string};
use combine::parser::combinator::recognize;
use combine::stream::{Positioned, Stream};
use combine::{choice, easy, parser, sep_by1, skip_many1, token, EasyParser, Parser, attempt};

use crate::expr::{Expr, ExprTree, MetadataPredicate};
use crate::operator::Operator;

// #[derive(Debug, PartialEq)]
// pub enum Expr {
//     Id(String), // replace this with terminating predicate thingies
//     Array(Vec<Expr>),
//     Parens(Box<Expr>),
//     Pair(Box<Expr>, Box<Expr>),
//     And(Vec<Expr>),
//     Or(Vec<Expr>),
// }

fn and_<Input>() -> impl Parser<Input, Output = ExprTree>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let skip_spaces = || spaces().silent();
    sep_by1(base().skip(skip_spaces()), string("&&").skip(skip_spaces())).map(|mut xs: Vec<_>| {
        if xs.len() > 1 {
            ExprTree {
                fs_ref: Box::new(Expr::Operator(Operator::And(xs))),
            }
        } else {
            xs.pop().unwrap()
        }
    })
}

fn or_<Input>() -> impl Parser<Input, Output = ExprTree>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let skip_spaces = || spaces().silent();
    sep_by1(and().skip(skip_spaces()), string("||").skip(skip_spaces()))
        .map(|mut xs: Vec<_>| {
            if xs.len() > 1 {
                ExprTree {
                    fs_ref: Box::new(Expr::Operator(Operator::Or(xs))),
                }
            } else {
                xs.pop().unwrap()
            }
        })
        .skip(skip_spaces())
}

// `impl Parser` can be used to create reusable parsers with zero overhead
fn base_<Input>() -> impl Parser<Input, Output = ExprTree>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    // A parser which skips past whitespace.
    // Since we aren't interested in knowing that our expression parser
    // could have accepted additional whitespace between the tokens we also silence the error.
    let skip_spaces = || spaces().silent();
    let lex_char = |c| char(c).skip(skip_spaces());

    let parens = (lex_char('('), or(), lex_char(')')).map(|(_, e, _)| e);

    let num = || {
        recognize(skip_many1(digit()))
            // .and_then(|s: String| {
            .map(|s: String| {
                // `bs` only contains digits which are ascii and thus UTF-8
                s.parse::<u64>().unwrap() // TODO: figure out andthen/error parsing, this will only trigger w/ ints greater than usize
            })
    };

    // starting with hardcoded size, I think. also, TODO: parser for MB/GB postfixes, but we can start with exact numeral sizes
    let size_predicate = (string("size("), num(), string(".."), num(), lex_char(')'))
        .map(|(_, d1, _, d2, _)| MetadataPredicate::Size { allowed: d1..d2 });

    let predicate = choice((
        attempt(string("binary()").map(|_| MetadataPredicate::Binary)),
        attempt(string("symlink()").map(|_| MetadataPredicate::Symlink)),
        attempt(string("exec()").map(|_| MetadataPredicate::Exec)),
        attempt(size_predicate),
        // TODO: content search predicate implies file (could later expand to binary search and etc) so if that's present filter on type there too
        // also, decide on correct name - eg might _not_ be file
        // string("file()").map(|_| MetadataPredicate::Exec),
        // TODO: add this I think?
        // string("dir()").map(|_| MetadataPredicate::Exec),
    ));

    // I don't think order matters here, inside the choice combinator? idk
    choice((
        // attempt(and_operator),
        predicate.map(|x| ExprTree {
            fs_ref: Box::new(Expr::Predicate(x)),
        }),
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

// pub fn expr(s: &str) -> impl Parser<> {
//     let e = or().easy_parse(s)?;
//     Ok(e.0) // TODO: assert no remaining string left to parse
// }

// entry point
parser! {
    pub fn or[Input]()(Input) -> ExprTree
    where [Input: Stream<Token = char>]
    {
        or_()
    }
}

parser! {
    fn and[Input]()(Input) -> ExprTree
    where [Input: Stream<Token = char>]
    {
        and_()
    }
}

parser! {
    fn base[Input]()(Input) -> ExprTree
    where [Input: Stream<Token = char>]
    {
        base_()
    }
}

#[test]
fn test_test_test() {
    let result: Result<_, easy::ParseError<&str>> =
        // or().easy_parse("");
        or().easy_parse("size(1..2)");
    // or().easy_parse("a || b");

    let expected = ExprTree {
        fs_ref: Box::new(Expr::Predicate(MetadataPredicate::Size { allowed: 1..2 })),
    };

    assert_eq!(result.map(|x| x.0), Ok(expected));
}

// fn mt(e: Expr<ExprTree>) -> ExprTree {
//     ExprTree {
//         fs_ref: Box::new(e),
//     }
// }
