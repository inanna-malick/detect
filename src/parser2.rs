use nom::character::complete::*;
use nom::combinator::{all_consuming, cut, map_res};
use nom::error::{Error, ParseError, VerboseError};
use nom::multi::separated_list1;
use nom::sequence::delimited;
use nom::{branch::*, bytes::complete::tag};
use nom::{IResult, Parser};
use nom_locate::LocatedSpan;
use nom_recursive::{recursive_parser, RecursiveInfo};
use regex::escape;

use crate::expr::{ContentPredicate, Expr, MetadataPredicate, NamePredicate};
use crate::predicate::{NumericalOp, Op, Predicate, RawPredicate, Selector};

// Input type must implement trait HasRecursiveInfo
// nom_locate::LocatedSpan<T, RecursiveInfo> implements it.
type Span<'a> = LocatedSpan<&'a str, RecursiveInfo>;

// type IResult<A, B> = IResult<A, B, VerboseError<A>>;
// pub type IResult<I, O, E = VerboseError<I>> = Result<(I, O), nom::Err<E>>;

pub fn expr(
    s: Span,
) -> IResult<Span, Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>> {
    all_consuming(_expr)(s)
}

fn _expr(
    s: Span,
) -> IResult<Span, Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>> {
    let predicate = map_res(raw_predicate, |p| p.parse()).map(Expr::Predicate);
    alt((parens, or, and, cut(predicate)))(s)
}

#[recursive_parser]
fn and(
    s: Span,
) -> IResult<Span, Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>> {
    let (s, x) = _expr(s)?;
    let (s, _) = space0(s)?;
    let (s, _) = tag("&&")(s)?;
    let (s, _) = space0(s)?;
    let (s, y) = _expr(s)?;

    let ret = Expr::and(x, y);

    Ok((s, ret))
}

#[recursive_parser]
fn parens(
    s: Span,
) -> IResult<Span, Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>> {
    let (s, _) = char('(')(s)?;
    let (s, _) = space0(s)?;
    let (s, e) = _expr(s)?;
    let (s, _) = space0(s)?;
    let (s, _) = char(')')(s)?;

    Ok((s, e))
}

// Apply recursive_parser by custom attribute
#[recursive_parser]
fn or(
    s: Span,
) -> IResult<Span, Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>> {
    let (s, x) = _expr(s)?;
    let (s, _) = space1(s)?;
    let (s, _) = tag("||")(s)?;
    let (s, _) = space1(s)?;
    let (s, y) = _expr(s)?;

    let ret = Expr::or(x, y);

    Ok((s, ret))
}

fn raw_predicate2(s: Span) -> IResult<Span, Expr<String>> {
    let (s, ret) = alpha1(s)?;
    Ok((s, Expr::Predicate(ret.to_string())))
}

fn raw_predicate(s: Span) -> IResult<Span, RawPredicate> {
    let (s, lhs) = selector(s)?;
    let (s, _) = space1(s)?;
    let (s, op) = operator(s)?;
    let (s, _) = space1(s)?;
    let (s, rhs) = alphanumeric1(s)?;
    let ret = RawPredicate {
        lhs,
        rhs: rhs.to_string(),
        op,
    };
    Ok((s, ret))
}
// todo: used escaped to get escaped string values

fn selector(s: Span) -> IResult<Span, Selector> {
    let (s, _) = char('@')(s)?;
    let mut selector = alt((
        alt((tag("name"), tag("filename"))).map(|_| Selector::FileName),
        alt((tag("path"), tag("filepath"))).map(|_| Selector::FilePath),
        alt((tag("extension"), tag("ext"))).map(|_| Selector::Extension),
        alt((tag("size"), tag("filesize"))).map(|_| Selector::Size),
        alt((tag("type"), tag("filetype"))).map(|_| Selector::EntityType),
        alt((tag("contents"), tag("file"))).map(|_| Selector::Contents),
    ));

    let (s, op) = selector(s)?;

    Ok((s, op))
}

fn operator(s: Span) -> IResult<Span, Op> {
    let mut operator = {
        use NumericalOp::*;
        use Op::*;
        alt((
            tag("contains").map(|_| Contains),
            tag("~=").map(|_| Matches),
            tag("==").map(|_| Equality),
            tag("<").map(|_| NumericComparison(Less)),
            tag(">").map(|_| NumericComparison(Greater)),
            tag("=<").map(|_| NumericComparison(LessOrEqual)),
            tag("=>").map(|_| NumericComparison(GreaterOrEqual)),
        ))
    };

    let (s, op) = operator(s)?;

    Ok((s, op))
}

#[test]
fn test() {
    // let mut or = separated_list1(ws(tag::<&str, &str, Error<_>>("||")), alpha0);

    // let ret = x("a || b || c && d || e");
    // println!("{:?}", ret.unwrap());

    let ret = expr(LocatedSpan::new_extra(
        "@name ~= test || @size == 5 && (@name == test || @name == test)",
        RecursiveInfo::new(),
    ));

    println!("{:?}", ret.unwrap().1);
    // if let nom::Err::Failure(errs) = ret.err().unwrap() {
    //     for e in errs.errors.iter() {

    //         println!("{:?}", e);
    //     }

    // }
}