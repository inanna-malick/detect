// #[macro_use]
use combine::error::ParseError;
use combine::parser::char::{char, digit, space, spaces, string};
use combine::stream::Stream;
use combine::*;
use regex::Regex;



use crate::expr::*;
use crate::predicate::{Bound, Op, Predicate, RawPredicate, Selector};

// TODO: use nom instead of combine

fn and_<Input>(
) -> impl Parser<Input, Output = Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let skip_spaces = || spaces().silent();
    sep_by1(not().skip(skip_spaces()), string("&&").skip(skip_spaces())).map(|mut xs: Vec<_>| {
        if xs.len() > 1 {
            let head = xs.pop().unwrap();
            xs.into_iter().fold(head, Expr::and)
        } else {
            // always at least one element if we're here
            xs.pop().unwrap()
        }
    })
}

fn not_<Input>(
) -> impl Parser<Input, Output = Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>>
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

// TODO use this, with 'in' operator (range)
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

fn or_<Input>(
) -> impl Parser<Input, Output = Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    let skip_spaces = || spaces().silent();
    sep_by1(and().skip(skip_spaces()), string("||").skip(skip_spaces()))
        .map(|mut xs: Vec<_>| {
            if xs.len() > 1 {
                let head = xs.pop().unwrap();
                xs.into_iter().fold(head, Expr::or)
            } else {
                xs.pop().unwrap()
            }
        })
        .skip(skip_spaces())
}

fn base_<Input>(
) -> impl Parser<Input, Output = Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>>
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

    // I don't think order matters here, inside the choice combinator? idk
    choice((
        attempt(raw_predicate()).map(Expr::Predicate),
        // attempt(filename_predicate),
        // attempt(filepath_predicate),
        // attempt(extension_predicate),
        // attempt(metadata_predicate),
        parens,
    ))
}

fn raw_predicate_<Input>(
) -> impl Parser<Input, Output = Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>
where
    Input: Stream<Token = char>,
    // Necessary due to rust-lang/rust#24159
    Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{
    // A parser which skips past whitespace.
    // Since we aren't interested in knowing that our expression parser
    // could have accepted additional whitespace between the tokens we also silence the error.
    let whitespace = || skip_many1(space());

    let selector = {
        choice((
            string("name").map(|_| Selector::Name),
            string("size").map(|_| Selector::Size),
        ))
    };

    let regex_str = || {
        many1(
            // TODO: should do this in a principled way that fully covers all allowed regex chars,
            //       instead I keep adding characters I want to use. shrug emoji.
            // TODO: another one of these for valid process stuff
            satisfy(|ch: char| {
                ch.is_alphanumeric() || ch == '.' || ch == '_' || ch == ' ' || ch == '-'
            })
            .expected("letter or digit or . _ or ' ' or -"), // TODO: clean this up idk
        )
    };

    let regex = || regex_str().map(|s: String| Regex::new(&s).unwrap());

    let operator = || {
        use Op::*;
        choice((
            attempt(string("contains").map(|_| Contains)),
            attempt(string("~=").map(|_| Matches)),
            //     Matches,  // '~=', 'matches'
            //     Equality, // '==', '=', 'is'
            // //     NumericComparison(NumericaComparisonOp),
            // // pub enum NumericaComparisonOp {
            //     Greater,        // '>'
            //     GreaterOrEqual, // >=
            //     LessOrEqual,    // <=
            //     Less,           // <
        ))
    };

    (selector, whitespace(), operator(), whitespace(), regex_str())
        .map(|(lhs, _, op, _, rhs)| RawPredicate { lhs, op, rhs })
        .then(|r| match r.parse() {
            Ok(x) => value(x).left(),
            // todo: idk why static str required, follow up later w/ eg format!("{:?}", e)
            Err(e) => unexpected_any("token").message("fixme").right(),
        })
}

// base expr: choice between predicates and parens (recursing back to or) WITH OR
// and expr: not a choice - sep_by_1 w/ base expr
// or expr: not a choice - sep_by_1 w/ and expr

// mutually recursive ordering defines precedence

// entry point
parser! {
    pub fn or[Input]()(Input) -> Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>
    where [Input: Stream<Token = char>]
    {
        or_()
    }
}

parser! {
    fn and[Input]()(Input) -> Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>
    where [Input: Stream<Token = char>]
    {
        and_()
    }
}

parser! {
    fn not[Input]()(Input) -> Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>
    where [Input: Stream<Token = char>]
    {
        not_()
    }
}

parser! {
    fn base[Input]()(Input) -> Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>
    where [Input: Stream<Token = char>]
    {
        base_()
    }
}

parser! {
    fn raw_predicate[Input]()(Input) -> Predicate<NamePredicate, MetadataPredicate, ContentPredicate>
    where [Input: Stream<Token = char>]
    {
        raw_predicate_()
    }
}
