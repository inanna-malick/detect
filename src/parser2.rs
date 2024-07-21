use nom::character::complete::*;
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


pub fn expr(s: Span) -> IResult<Span, Expr<RawPredicate>> {
    alt((parens, or, and, raw_predicate))(s)
}

#[recursive_parser]
pub fn and(s: Span) -> IResult<Span, Expr<RawPredicate>> {
    let (s, x) = expr(s)?;
    let (s, _) = space1(s)?;
    let (s, _) = tag("&&")(s)?;
    let (s, _) = space1(s)?;
    let (s, y) = expr(s)?;

    let ret = Expr::and(x, y);

    Ok((s, ret))
}

#[recursive_parser]
pub fn parens(s: Span) -> IResult<Span, Expr<RawPredicate>> {
    let (s, _) = char('(')(s)?;
    let (s, _) = space0(s)?;
    let (s, e) = expr(s)?;
    let (s, _) = space0(s)?;
    let (s, _) = char(')')(s)?;


    Ok((s, e))
}


// Apply recursive_parser by custom attribute
#[recursive_parser]
pub fn or(s: Span) -> IResult<Span, Expr<RawPredicate>> {
    let (s, x) = expr(s)?;
    let (s, _) = space1(s)?;
    let (s, _) = tag("||")(s)?;
    let (s, _) = space1(s)?;
    let (s, y) = expr(s)?;

    let ret = Expr::or(x, y);

    Ok((s, ret))
}

pub fn raw_predicate(s: Span) -> IResult<Span, Expr<RawPredicate>> {
    let (s, lhs) = selector(s)?;
    let (s, _) = space1(s)?;
    let (s, op) = operator(s)?;
    let (s, _) = space1(s)?;
    let (s, rhs) = alpha1(s)?;
    let ret = RawPredicate{lhs, rhs: rhs.to_string(), op};
    Ok((s, Expr::Predicate(ret)))
}
// todo: used escaped to get escaped string values

pub fn selector(s: Span) -> IResult<Span, Selector> {
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


pub fn operator(s: Span) -> IResult<Span, Op> {
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
    let ret = expr(LocatedSpan::new_extra("@name ~= test || @path == foo && size > 999", RecursiveInfo::new()));
    println!("{:?}", ret.unwrap().1);
    let ret = expr(LocatedSpan::new_extra("(@name ~= test)", RecursiveInfo::new()));
    println!("{:?}", ret.unwrap().1);
}
