pub(crate) use crate::predicate::{ContentPredicate, MetadataPredicate, NamePredicate};
use itertools::*;
use recursion::map_layer::MapLayer;
use std::fmt::{Debug, Display};

use super::Expr;

/// short-lived single layer of a filesystem entity matcher expression, used for
/// expressing recursive algorithms over a single layer of a borrowed Expr
pub enum ExprLayer<
    'a,
    Recurse,
    Name = NamePredicate,
    Metadata = MetadataPredicate,
    Contents = ContentPredicate,
> {
    // boolean literals
    KnownResult(bool),
    // boolean operators
    Operator(Operator<Recurse>),
    // borrowed predicates
    Name(&'a Name),
    Metadata(&'a Metadata),
    Contents(&'a Contents),
}

// having operator as a distinct type might seem a bit odd, but it lets us
// factor out the short circuiting logic
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
                    // we know they aren't _all_ true, but maybe some of them are. filter those out,
                    // and see if we can collapse this down to a single expression
                    let mut filtered: Vec<_> = ands
                        .into_iter()
                        .filter(|e| !matches!(e, KnownResult(true)))
                        .collect();
                    if filtered.len() == 1 {
                        filtered.remove(0)
                    } else {
                        And(filtered)
                    }
                }
            }
            Operator::Or(ors) => {
                if ors.iter().any(|b| matches!(b, KnownResult(true))) {
                    KnownResult(true)
                } else if ors.iter().all(|b| matches!(b, KnownResult(false))) {
                    KnownResult(false)
                } else {
                    // we know they aren't _all_ false, but maybe some of them are. filter those out,
                    // and see if we can collapse this down to a single expression
                    let mut filtered: Vec<_> = ors
                        .into_iter()
                        .filter(|e| !matches!(e, KnownResult(false)))
                        .collect();
                    if filtered.len() == 1 {
                        filtered.remove(0)
                    } else {
                        Or(filtered)
                    }
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

// for use in recursion visualizations
impl<'a, N: Display, M: Display, C: Display> Display for ExprLayer<'a, (), N, M, C> {
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
