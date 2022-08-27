// #[macro_use]
// extern crate combine;
// use crate::{expr::*, operator::Operator};
use combine::parser::char::{char, letter, spaces, string};
use combine::{between, choice, many1, parser, sep_by, Parser, EasyParser, token, easy, attempt};
use combine::error::{ParseError, StdParseResult};
use combine::stream::{Stream, Positioned};
use combine::stream::position;

#[derive(Debug, PartialEq)]
pub enum Expr {
    Id(String),
    Array(Vec<Expr>),
    Parens(Box<Expr>),
    Pair(Box<Expr>, Box<Expr>)
}

// `impl Parser` can be used to create reusable parsers with zero overhead
fn expr_<Input>() -> impl Parser< Input, Output = Expr>
    where Input: Stream<Token = char>,
          // Necessary due to rust-lang/rust#24159
          Input::Error: ParseError<Input::Token, Input::Range, Input::Position>,
{


    // let predicate_parser = {
    //     let binary_parser = string("binary()").map(|_| MetadataPredicate::Binary);
    //     let symlink_parser = string("symlink()").map(|_| MetadataPredicate::Symlink);
    //     binary_parser
    //         .or(symlink_parser)
    //         .map(Expr::<ExprTree>::Predicate)
    //         .skip(spaces())
    // }
    // .map(mt);

    let word = many1(letter());
    // TODO: full regex support later, also have support for arbitrary search strings I guess (or more likely use someone else's regex parser lol)
    // TODO: figure out how to make it only close/open on _unescaped_ parens, especially re: close
    // let regex_predicate = (string("contains"), between(token('('), token(')'), word));

    // A parser which skips past whitespace.
    // Since we aren't interested in knowing that our expression parser
    // could have accepted additional whitespace between the tokens we also silence the error.
    let skip_spaces = || spaces().silent();

    //Creates a parser which parses a char and skips any trailing whitespace
    let lex_char = |c| char(c).skip(skip_spaces());

    let comma_list = sep_by(expr(), lex_char(','));
    let array = between(lex_char('['), lex_char(']'), comma_list);


    let parens = (lex_char('('),
                expr(),
                lex_char(')'))
                   .map(|(_, e ,_)| Expr::Parens(Box::new(e)));

    //We can use tuples to run several parsers in sequence
    //The resulting type is a tuple containing each parsers output
    let pair = (lex_char('('),
                expr(),
                lex_char(','),
                expr(),
                lex_char(')'))
                   .map(|t| Expr::Pair(Box::new(t.1), Box::new(t.3)));

    choice((
        word.map(Expr::Id),
        array.map(Expr::Array),
        attempt(parens),
        attempt(pair),
    ))
        .skip(skip_spaces())
}

// As this expression parser needs to be able to call itself recursively `impl Parser` can't
// be used on its own as that would cause an infinitely large type. We can avoid this by using
// the `parser!` macro which erases the inner type and the size of that type entirely which
// lets it be used recursively.
//
// (This macro does not use `impl Trait` which means it can be used in rust < 1.26 as well to
// emulate `impl Parser`)
parser!{
    fn expr[Input]()(Input) -> Expr
    where [Input: Stream<Token = char>]
    {
        expr_()
    }
}

#[test]
fn test_test_test() {
    let result: Result<_, easy::ParseError<&str>> = expr()
        .easy_parse("[[], (hello, world), [(rust)]]");
    let expr = Expr::Array(vec![
          Expr::Array(Vec::new())
        , Expr::Pair(Box::new(Expr::Id("hello".to_string())),
                     Box::new(Expr::Id("world".to_string())))
        , Expr::Array(vec![Expr::Id("rust".to_string())])
    ]);
    assert_eq!(result.map(|x| x.0), Ok(expr));
}


// fn mt(e: Expr<ExprTree>) -> ExprTree {
//     ExprTree {
//         fs_ref: Box::new(e),
//     }
// }
