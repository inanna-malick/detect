use nom::character::complete::*;
use nom::combinator::{all_consuming, cut, map_res};
use nom::error::{VerboseError, VerboseErrorKind};
use nom::Parser;
use nom::{branch::*, bytes::complete::tag};
use nom_locate::LocatedSpan;
use nom_recursive::{recursive_parser, RecursiveInfo};

use crate::expr::{ContentPredicate, Expr, MetadataPredicate, NamePredicate};
use crate::predicate::{NumericalOp, Op, Predicate, RawPredicate, Selector};

// Input type must implement trait HasRecursiveInfo
// nom_locate::LocatedSpan<T, RecursiveInfo> implements it.
type Span<'a> = LocatedSpan<&'a str, RecursiveInfo>;

// type IResult<A, B> = IResult<A, B, VerboseError<A>>;
pub type IResult<I, O, E = VerboseError<I>> = Result<(I, O), nom::Err<E>>;

pub fn expr(
    s: Span,
) -> IResult<Span, Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>> {
    all_consuming(_expr)(s)
}

fn _expr(
    s: Span,
) -> IResult<Span, Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>> {
    let predicate = map_res(raw_predicate, |p| p.parse()).map(Expr::Predicate);
    alt((parens, and, or, not, cut(predicate)))(s)
}

#[recursive_parser]
fn not(
    s: Span,
) -> IResult<Span, Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>> {
    let (s, _) = tag("!")(s)?;

    let predicate = map_res(raw_predicate, |p| p.parse()).map(Expr::Predicate);
    let (s, e) = alt((parens, cut(predicate)))(s)?;

    let ret = Expr::not(e);

    Ok((s, ret))
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
    let data = "@name ~= test || @size == 5 && (@name == test || @name == test)";

    let data: LocatedSpan<&str, RecursiveInfo> = LocatedSpan::new_extra(data, RecursiveInfo::new());

    let ret = expr(data);

    match ret {
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
            println!(
                "verbose errors - `root::<VerboseError>(data)`:\n{}",
                convert_error(data, e)
            );
        }
        Ok(x) => {
            println!("{:?}", x.1);
        }
        _ => {}
    }
}

// copy of nom's convert_error specialized to nom_locate span type, fixes type error on Deref
pub fn convert_error(input: Span, e: VerboseError<Span>) -> nom::lib::std::string::String {
    use nom::lib::std::fmt::Write;
    use nom::Offset;

    let mut result = nom::lib::std::string::String::new();

    for (i, (substring, kind)) in e.errors.iter().enumerate() {
        let offset = input.offset(substring);

        if input.is_empty() {
            match kind {
                VerboseErrorKind::Char(c) => {
                    write!(&mut result, "{}: expected '{}', got empty input\n\n", i, c)
                }
                VerboseErrorKind::Context(s) => {
                    write!(&mut result, "{}: in {}, got empty input\n\n", i, s)
                }
                VerboseErrorKind::Nom(e) => {
                    write!(&mut result, "{}: in {:?}, got empty input\n\n", i, e)
                }
            }
        } else {
            let prefix = &input.as_bytes()[..offset];

            // Count the number of newlines in the first `offset` bytes of input
            let line_number = prefix.iter().filter(|&&b| b == b'\n').count() + 1;

            // Find the line that includes the subslice:
            // Find the *last* newline before the substring starts
            let line_begin = prefix
                .iter()
                .rev()
                .position(|&b| b == b'\n')
                .map(|pos| offset - pos)
                .unwrap_or(0);

            // Find the full line after that newline
            let line = input[line_begin..]
                .lines()
                .next()
                .unwrap_or(&input[line_begin..])
                .trim_end();

            // The (1-indexed) column number is the offset of our substring into that line
            let column_number = line.offset(substring) + 1;

            match kind {
                VerboseErrorKind::Char(c) => {
                    if let Some(actual) = substring.chars().next() {
                        write!(
                            &mut result,
                            "{i}: at line {line_number}:\n\
               {line}\n\
               {caret:>column$}\n\
               expected '{expected}', found {actual}\n\n",
                            i = i,
                            line_number = line_number,
                            line = line,
                            caret = '^',
                            column = column_number,
                            expected = c,
                            actual = actual,
                        )
                    } else {
                        write!(
                            &mut result,
                            "{i}: at line {line_number}:\n\
               {line}\n\
               {caret:>column$}\n\
               expected '{expected}', got end of input\n\n",
                            i = i,
                            line_number = line_number,
                            line = line,
                            caret = '^',
                            column = column_number,
                            expected = c,
                        )
                    }
                }
                VerboseErrorKind::Context(s) => write!(
                    &mut result,
                    "{i}: at line {line_number}, in {context}:\n\
             {line}\n\
             {caret:>column$}\n\n",
                    i = i,
                    line_number = line_number,
                    context = s,
                    line = line,
                    caret = '^',
                    column = column_number,
                ),
                VerboseErrorKind::Nom(e) => write!(
                    &mut result,
                    "{i}: at line {line_number}, in {nom_err:?}:\n\
             {line}\n\
             {caret:>column$}\n\n",
                    i = i,
                    line_number = line_number,
                    nom_err = e,
                    line = line,
                    caret = '^',
                    column = column_number,
                ),
            }
        }
        // Because `write!` to a `String` is infallible, this `unwrap` is fine.
        .unwrap();
    }

    result
}
