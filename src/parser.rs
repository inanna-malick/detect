// #[macro_use]
use combine::error::ParseError;
use combine::parser::char::{alpha_num, char, digit, spaces, string};
use combine::parser::combinator::recognize;
use combine::stream::Stream;
use combine::*;
use regex::Regex;

use crate::expr::*;
use crate::operator::Operator;

fn and_<Input>() -> impl Parser<Input, Output = ExprTree>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let skip_spaces = || spaces().silent();
    sep_by1(not().skip(skip_spaces()), string("&&").skip(skip_spaces())).map(|mut xs: Vec<_>| {
        if xs.len() > 1 {
            ExprTree::new(ExprLayer::Operator(Operator::And(xs)))
        } else {
            xs.pop().unwrap()
        }
    })
}

fn not_<Input>() -> impl Parser<Input, Output = ExprTree>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let skip_spaces = || spaces().silent();
    let lex_char = |c| char(c).skip(skip_spaces());

    choice((
        (lex_char('!'), base()).map(|(_, x)| ExprTree::new(ExprLayer::Operator(Operator::Not(x)))),
        base(),
    ))
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
                ExprTree::new(ExprLayer::Operator(Operator::Or(xs)))
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

    let regex = || many1(alpha_num()).map(|s: String| Regex::new(&s).unwrap());
    let contains_predicate =
        (string("contains("), regex(), lex_char(')')).map(|(_, s, _)| ContentsMatcher::Regex(s));
    let contents_predicate = choice((
        attempt(contains_predicate),
        string("utf8()").map(|_| ContentsMatcher::Utf8),
    ))
    .map(|p| ExprTree::new(ExprLayer::Contents(p)));

    let filename_predicate = (string("filename("), regex(), lex_char(')'))
        .map(|(_, s, _)| ExprTree::new(ExprLayer::Name(NameMatcher::Regex(s))));

    // TODO: parser for MB/GB postfixes, but we can start with exact numeral sizes
    let size_predicate = (string("size("), num(), string(".."), num(), lex_char(')'))
        .map(|(_, d1, _, d2, _)| MetadataMatcher::Filesize(d1..d2));

    // I don't think order matters here, inside the choice combinator? idk
    choice((
        attempt(contents_predicate),
        attempt(filename_predicate),
        attempt(size_predicate).map(|x| ExprTree::new(ExprLayer::Metadata(x))),
        parens,
    ))
    .skip(skip_spaces())
}

// base expr: choice between predicates and parens (recursing back to or) WITH OR
// and expr: not a choice - sep_by_1 w/ base expr
// or expr: not a choice - sep_by_1 w/ and expr

// mutually recursive ordering defines precedence

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
    fn not[Input]()(Input) -> ExprTree
    where [Input: Stream<Token = char>]
    {
        not_()
    }
}

parser! {
    fn base[Input]()(Input) -> ExprTree
    where [Input: Stream<Token = char>]
    {
        base_()
    }
}
