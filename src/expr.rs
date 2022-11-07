pub mod recurse;

pub(crate) use crate::predicate::{ContentPredicate, MetadataPredicate, NamePredicate};
use itertools::*;
use std::fmt::Display;

use self::recurse::{ExprLayer, Operator};

/// Filesystem entity matcher expression, with branches for matchers on
/// - file name
/// - file metadata
/// - file contents
pub enum Expr<Name, Metadata, Content> {
    // literal boolean values
    KnownResult(bool),
    // boolean operators
    Not(Box<Self>),
    And(Vec<Self>),
    Or(Vec<Self>),
    // predicates
    Name(Name),
    Metadata(Metadata),
    Contents(Content),
}

/// A filesystem entity matcher expression that owns its predicates
pub type OwnedExpr<Name = NamePredicate, Metadata = MetadataPredicate, Content = ContentPredicate> =
    Expr<Name, Metadata, Content>;

/// A filesystem entity matcher expression with borrowed predicates
pub type BorrowedExpr<
    'a,
    Name = &'a NamePredicate,
    Metadata = &'a MetadataPredicate,
    Content = &'a ContentPredicate,
> = Expr<Name, Metadata, Content>;

impl<N: Display, M: Display, C: Display> Display for Expr<N, M, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Not(x) => write!(f, "!{}", x),
            Self::And(xs) => {
                let xs: String =
                    intersperse(xs.iter().map(|x| format!("{}", x)), " && ".to_string()).collect();
                write!(f, "{}", xs)
            }
            Self::Or(xs) => {
                let xs: String =
                    Itertools::intersperse(xs.iter().map(|x| format!("{}", x)), " || ".to_string())
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
