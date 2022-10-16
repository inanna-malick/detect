pub(crate) use crate::matcher::{ContentsMatcher, MetadataMatcher, NameMatcher};
pub(crate) use crate::operator::Operator;
use itertools::*;
use recursion::map_layer::MapLayer;
use recursion::map_layer::Project;
use std::fmt::{Debug, Display};

/// Filesystem entity matcher expression, with branches for matchers on
/// - file name
/// - file metadata
/// - file contents
// #[derive(Debug)]
pub enum Expr<Name, Metadata, Contents> {
    Operator(Box<Operator<Self>>),
    KnownResult(bool),
    Name(Name),
    Metadata(Metadata),
    Contents(Contents),
}

// YES! simplifies ownership model immensely
pub type OwnedExpr<Name = NameMatcher, Metadata = MetadataMatcher, Contents = ContentsMatcher> =
    Expr<Name, Metadata, Contents>;
pub type BorrowedExpr<
    'a,
    Name = &'a NameMatcher,
    Metadata = &'a MetadataMatcher,
    Contents = &'a ContentsMatcher,
> = Expr<Name, Metadata, Contents>;

impl<N: Display, M: Display, C: Display> Debug for Expr<N, M, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Operator(o) => match o.as_ref() {
                Operator::Not(x) => write!(f, "!{:?}", x),
                Operator::And(xs) => {
                    let xs: String =
                        intersperse(xs.iter().map(|x| format!("{:?}", x)), " && ".to_string())
                            .collect();
                    write!(f, "{}", xs)
                }
                Operator::Or(xs) => {
                    let xs: String = Itertools::intersperse(
                        xs.iter().map(|x| format!("{:?}", x)),
                        " || ".to_string(),
                    )
                    .collect();
                    write!(f, "{}", xs)
                }
            },
            Self::KnownResult(b) => {
                write!(f, "{}", b)
            }
            Self::Name(arg0) => write!(f, "{}", arg0),
            Self::Metadata(arg0) => write!(f, "{}", arg0),
            Self::Contents(arg0) => write!(f, "{}", arg0),
        }
    }
}

/// short-lived single layer of a filesystem entity matcher expression, used for
/// expressing recursive algorithms over a single layer of a borrowed Expr
// #[derive(Debug)]
pub enum ExprLayer<
    'a,
    Recurse,
    Name = NameMatcher,
    Metadata = MetadataMatcher,
    Contents = ContentsMatcher,
> {
    Operator(Operator<Recurse>),
    KnownResult(bool),
    Name(&'a Name),
    Metadata(&'a Metadata),
    Contents(&'a Contents),
}

impl<'a, N: Display, M: Display, C: Display> Debug for ExprLayer<'a, (), N, M, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Operator(o) => match o {
                Operator::Not(_) => write!(f, "!_"),
                Operator::And(xs) => {
                    let xs: String =
                        Itertools::intersperse(xs.iter().map(|_| "_"), " && ").collect();
                    write!(f, "{}", xs)
                }
                Operator::Or(xs) => {
                    let xs: String =
                        Itertools::intersperse(xs.iter().map(|_| "_"), " || ").collect();
                    write!(f, "{}", xs)
                }
            },
            Self::KnownResult(b) => {
                write!(f, "{}", b)
            }
            Self::Name(arg0) => write!(f, "{}", arg0),
            Self::Metadata(arg0) => write!(f, "{}", arg0),
            Self::Contents(arg0) => write!(f, "{}", arg0),
        }
    }
}

impl<'a, Name, Meta, Content, A, B> MapLayer<B> for ExprLayer<'a, A, Name, Meta, Content> {
    type Unwrapped = A;
    type To = ExprLayer<'a, B, Name, Meta, Content>;
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To {
        use ExprLayer::*;
        match self {
            Operator(o) => Operator(o.map_layer(f)),
            KnownResult(k) => KnownResult(k),
            Name(x) => Name(x),
            Metadata(x) => Metadata(x),
            Contents(x) => Contents(x),
        }
    }
}

impl<'a, S1: 'a, S2: 'a, S3: 'a> Project for &'a Expr<S1, S2, S3> {
    type To = ExprLayer<'a, Self, S1, S2, S3>;

    // project into ExprLayer
    fn project(self) -> Self::To {
        match self {
            Expr::Operator(o) => ExprLayer::Operator(match o.as_ref() {
                Operator::Not(x) => Operator::Not(x),
                Operator::And(xs) => Operator::And(xs.iter().collect()),
                Operator::Or(xs) => Operator::Or(xs.iter().collect()),
            }),
            Expr::KnownResult(b) => ExprLayer::KnownResult(*b),
            Expr::Name(n) => ExprLayer::Name(n),
            Expr::Metadata(m) => ExprLayer::Metadata(m),
            Expr::Contents(c) => ExprLayer::Contents(c),
        }
    }
}
