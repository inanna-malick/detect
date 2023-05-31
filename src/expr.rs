pub mod recurse;

use crate::predicate::Predicate;
pub(crate) use crate::predicate::{ContentPredicate, MetadataPredicate, NamePredicate};
use itertools::*;
use std::fmt::Display;

use self::recurse::{ExprLayer, Operator};

/// Filesystem entity matcher expression with boolean logic and predicates
pub enum Expr<P = Predicate> {
    // boolean operators
    Not(Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    // predicates
    Predicate(P),
}

impl<P> Expr<P> {
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

// /// A filesystem entity matcher expression that owns its predicates
// pub type OwnedExpr<Name = NamePredicate, Metadata = MetadataPredicate, Content = ContentPredicate> =
//     Expr<Name, Metadata, Content>;

// /// A filesystem entity matcher expression with borrowed predicates
// pub type BorrowedExpr<
//     'a,
//     Name = &'a NamePredicate,
//     Metadata = &'a MetadataPredicate,
//     Content = &'a ContentPredicate,
// > = Expr<Name, Metadata, Content>;

impl<P: Display> Display for Expr<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Not(x) => write!(f, "!{}", x),
            Self::And(a, b) => {
                write!(f, "{} && {}", a, b)
            }
            Self::Or(a, b) => {
                write!(f, "{} || {}", a, b)
            }
            Self::Predicate(arg0) => write!(f, "{}", arg0),
        }
    }
}
