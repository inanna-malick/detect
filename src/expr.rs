pub mod recurse;

pub(crate) use crate::matcher::{ContentsMatcher, MetadataMatcher, NameMatcher};
use itertools::*;
use std::fmt::{Debug, Display};

/// Filesystem entity matcher expression, with branches for matchers on
/// - file name
/// - file metadata
/// - file contents
pub enum Expr<Name, Metadata, Contents> {
    // literal boolean values
    KnownResult(bool),
    // boolean operators
    Not(Box<Self>),
    And(Vec<Self>),
    Or(Vec<Self>),
    // predicates
    Name(Name),
    Metadata(Metadata),
    Contents(Contents),
}

/// A filesystem entity matcher expression that owns its predicates
pub type OwnedExpr<Name = NameMatcher, Metadata = MetadataMatcher, Contents = ContentsMatcher> =
    Expr<Name, Metadata, Contents>;

/// A filesystem entity matcher expression with borrowed predicates
pub type BorrowedExpr<
    'a,
    Name = &'a NameMatcher,
    Metadata = &'a MetadataMatcher,
    Contents = &'a ContentsMatcher,
> = Expr<Name, Metadata, Contents>;

impl<N: Display, M: Display, C: Display> Debug for Expr<N, M, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Not(x) => write!(f, "!{:?}", x),
            Self::And(xs) => {
                let xs: String =
                    intersperse(xs.iter().map(|x| format!("{:?}", x)), " && ".to_string())
                        .collect();
                write!(f, "{}", xs)
            }
            Self::Or(xs) => {
                let xs: String = Itertools::intersperse(
                    xs.iter().map(|x| format!("{:?}", x)),
                    " || ".to_string(),
                )
                .collect();
                write!(f, "{}", xs)
            }
            Self::KnownResult(b) => {
                write!(f, "{}", b)
            }
            Self::Name(arg0) => write!(f, "{}", arg0),
            Self::Metadata(arg0) => write!(f, "{}", arg0),
            Self::Contents(arg0) => write!(f, "{}", arg0),
        }
    }
}
