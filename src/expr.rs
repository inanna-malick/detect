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
    // boolean operators
    Not(Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    // predicates
    Name(Name),
    Metadata(Metadata),
    Contents(Content),
}

impl<A, B, C> Expr<A, B, C> {
    pub fn and(a: Self, b: Self) -> Self {
        Self::And(Box::new(a), Box::new(b))
    }
    pub fn or(a: Self, b: Self) -> Self {
        Self::Or(Box::new(a), Box::new(b))
    }
    pub fn not(a: Self) -> Self {
        Self::Not(Box::new(a))
    }
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
            Self::And(a, b) => {
                write!(f, "{} && {}", a, b)
            }
            Self::Or(a, b) => {
                write!(f, "{} || {}", a, b)
            }
            Self::Name(arg0) => write!(f, "{}", arg0),
            Self::Metadata(arg0) => write!(f, "{}", arg0),
            Self::Contents(arg0) => write!(f, "{}", arg0),
        }
    }
}
