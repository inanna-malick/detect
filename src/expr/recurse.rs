pub(crate) use crate::matcher::{ContentsMatcher, MetadataMatcher, NameMatcher};
use itertools::*;
use recursion::map_layer::MapLayer;
use std::fmt::{Debug, Display};

use super::Expr;

/// short-lived single layer of a filesystem entity matcher expression, used for
/// expressing recursive algorithms over a single layer of a borrowed Expr
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

#[derive(Debug, Eq, PartialEq)]
pub enum Operator<Recurse> {
    Not(Recurse),
    And(Vec<Recurse>),
    Or(Vec<Recurse>),
}

impl<A, B, C> Operator<Expr<A, B, C>> {
    pub fn attempt_short_circuit(self) -> Expr<A, B, C> {
        use Expr::*;
        match self {
            Operator::And(ands) => {
                if ands.iter().any(|b| matches!(b, KnownResult(false))) {
                    KnownResult(false)
                } else if ands.iter().all(|b| matches!(b, KnownResult(true))) {
                    KnownResult(true)
                } else {
                    And(ands)
                }
            }
            Operator::Or(ors) => {
                if ors.iter().any(|b| matches!(b, KnownResult(true))) {
                    KnownResult(true)
                } else if ors.iter().all(|b| matches!(b, KnownResult(false))) {
                    KnownResult(false)
                } else {
                    Or(ors)
                }
            }
            Operator::Not(x) => match x {
                KnownResult(b) => KnownResult(!b),
                _ => Not(Box::new(x)),
            },
        }
    }
}

impl<'a, Name, Meta, Content, A, B> MapLayer<B> for ExprLayer<'a, A, Name, Meta, Content> {
    type Unwrapped = A;
    type To = ExprLayer<'a, B, Name, Meta, Content>;
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        use self::Operator::*;
        use ExprLayer::*;
        match self {
            Operator(o) => Operator(match o {
                Not(a) => Not(f(a)),
                And(xs) => And(xs.into_iter().map(f).collect()),
                Or(xs) => Or(xs.into_iter().map(f).collect()),
            }),
            KnownResult(k) => KnownResult(k),
            Name(x) => Name(x),
            Metadata(x) => Metadata(x),
            Contents(x) => Contents(x),
        }
    }
}

// TODO: this should probably be 'display', but the current impl of visualization machinery in 'recurse' wants Debug
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
