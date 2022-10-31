// #[macro_use]
use combine::error::ParseError;
use combine::parser::char::{char, digit, spaces, string};
use combine::stream::Stream;
use combine::*;
use regex::Regex;

use crate::expr::*;
use crate::predicate::Bound;

fn and_<Input>() -> impl Parser<Input, Output = OwnedExpr>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let skip_spaces = || spaces().silent();
    sep_by1(not().skip(skip_spaces()), string("&&").skip(skip_spaces())).map(|mut xs: Vec<_>| {
        if xs.len() > 1 {
            Expr::And(xs)
        } else {
            xs.pop().unwrap()
        }
    })
}

fn not_<Input>() -> impl Parser<Input, Output = OwnedExpr>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let skip_spaces = || spaces().silent();
    let lex_char = |c| char(c).skip(skip_spaces());

    choice((
        (lex_char('!'), base()).map(|(_, x)| Expr::Not(Box::new(x))),
        base(),
    ))
}

fn kb_mb_bound_<Input>() -> impl Parser<Input, Output = Bound>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let num = || {
        many1(digit()).map(|s: String| {
            // `bs` only contains digits which are ascii and thus UTF-8
            s.parse::<u64>().unwrap()
        })
    };

    let kb_mb_num = || {
        choice((
            attempt((num(), char('k'), char('b'))).map(|(n, _, _)| n * 1024),
            attempt((num(), char('m'), char('b'))).map(|(n, _, _)| n * 1024 * 1024),
            attempt(num()),
        ))
    };

    let full_range =
        (kb_mb_num(), string(".."), kb_mb_num()).map(|(x1, _, x2)| Bound::Full(x1..x2));
    let left_range = (kb_mb_num(), string("..")).map(|(x1, _)| Bound::Left(x1..));
    let right_range = (string(".."), kb_mb_num()).map(|(_, x2)| Bound::Right(..x2));
    choice((
        attempt(full_range),  // x1..x2
        attempt(left_range),  // x1..
        attempt(right_range), // ..x2
    ))
}

parser! {
    fn kb_mb_bound[Input]()(Input) -> Bound
    where [Input: Stream<Token = char>]
    {
        kb_mb_bound_()
    }
}

fn or_<Input>() -> impl Parser<Input, Output = OwnedExpr>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let skip_spaces = || spaces().silent();
    sep_by1(and().skip(skip_spaces()), string("||").skip(skip_spaces()))
        .map(|mut xs: Vec<_>| {
            if xs.len() > 1 {
                Expr::Or(xs)
            } else {
                xs.pop().unwrap()
            }
        })
        .skip(skip_spaces())
}

// `impl Parser` can be used to create reusable parsers with zero overhead
fn base_<Input>() -> impl Parser<Input, Output = OwnedExpr>
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

    let regex = || {
        many1(
            // TODO: should do this in a principled way that fully covers all allowed regex chars,
            //       instead I keep adding characters I want to use. shrug emoji.
            satisfy(|ch: char| ch.is_alphanumeric() || ch == '.' || ch == '_' || ch == ' ')
                .expected("letter or digit or ."),
        )
        .map(|s: String| Regex::new(&s).unwrap())
    };

    let contains_predicate =
        (string("contains("), regex(), char(')')).map(|(_, s, _)| ContentPredicate::Regex(s));
    let contents_predicate = choice((
        attempt(contains_predicate),
        string("utf8()").map(|_| ContentPredicate::Utf8),
    ))
    .map(Expr::Contents);

    let filename_predicate = (string("filename("), regex(), lex_char(')'))
        .map(|(_, s, _)| Expr::Name(NamePredicate::Regex(s)));

    let extension_predicate = (
        string("extension("),
        many1(
            satisfy(|ch: char| ch.is_alphanumeric() || ch == '.').expected("letter or digit or ."),
        ),
        lex_char(')'),
    )
        .map(|(_, s, _)| Expr::Name(NamePredicate::Extension(s)));

    let size_predicate = (string("size("), kb_mb_bound(), lex_char(')'))
        .map(|(_, range, _)| MetadataPredicate::Filesize(range));

    let metadata_predicate = choice((
        attempt(size_predicate),
        attempt(string("executable()").map(|_| MetadataPredicate::Executable())),
        // TODO: add file/symlink predicate branches
        attempt(string("dir()").map(|_| MetadataPredicate::Dir())),
    ))
    .map(Expr::Metadata);

    // I don't think order matters here, inside the choice combinator? idk
    choice((
        attempt(contents_predicate),
        attempt(filename_predicate),
        attempt(extension_predicate),
        attempt(metadata_predicate),
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
    pub fn or[Input]()(Input) -> OwnedExpr
    where [Input: Stream<Token = char>]
    {
        or_()
    }
}

parser! {
    fn and[Input]()(Input) -> OwnedExpr
    where [Input: Stream<Token = char>]
    {
        and_()
    }
}

parser! {
    fn not[Input]()(Input) -> OwnedExpr
    where [Input: Stream<Token = char>]
    {
        not_()
    }
}

parser! {
    fn base[Input]()(Input) -> OwnedExpr
    where [Input: Stream<Token = char>]
    {
        base_()
    }
}
