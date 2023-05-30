pub(crate) use crate::predicate::{ContentPredicate, MetadataPredicate, NamePredicate};
use itertools::*;
use recursion_schemes::{
    functor::{Functor, PartiallyApplied},
    recursive::Recursive,
};
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
    And(Recurse, Recurse),
    Or(Recurse, Recurse),
}

pub trait ThreeValued {
    fn as_bool(&self) -> Option<bool>;
    fn lift_bool(b: bool) -> Self;

    fn is_false(&self) -> bool {
        self.as_bool().is_some_and(|x| !x)
    }

    fn is_true(&self) -> bool {
        self.as_bool().is_some_and(|x| x)
    }
}

impl<A, B, C> From<Operator<Self>> for Expr<A, B, C> {
    fn from(value: Operator<Expr<A, B, C>>) -> Self {
        match value {
            Operator::Not(x) => Self::Not(Box::new(x)),
            Operator::And(a, b) => Self::And(Box::new(a), Box::new(b)),
            Operator::Or(a, b) => Self::Or(Box::new(a), Box::new(b)),
        }
    }
}

pub enum ShortCircuit<X> {
    Known(bool),
    Unknown(X),
}

impl<X> ShortCircuit<X> {
    fn known(&self) -> Option<bool> {
        match self {
            ShortCircuit::Known(k) => Some(*k),
            ShortCircuit::Unknown(_) => None,
        }
    }
}

impl<A, B, C> Operator<ShortCircuit<Expr<A, B, C>>> {
    pub fn attempt_short_circuit(self) -> ShortCircuit<Expr<A, B, C>> {
        // use Expr::*;
        match self {
            Operator::And(a, b) => {
                use ShortCircuit::*;
                match (a, b) {
                    (Known(false), _) => Known(false),
                    (_, Known(false)) => Known(false),
                    (x, Known(true)) => x,
                    (Known(true), x) => x,
                    (Unknown(a), Unknown(b)) => Unknown(Expr::and(a, b)),
                }
            }
            Operator::Or(a, b) => {
                use ShortCircuit::*;
                match (a, b) {
                    (Known(true), _) => Known(true),
                    (_, Known(true)) => Known(true),
                    (x, Known(false)) => x,
                    (Known(false), x) => x,
                    (Unknown(a), Unknown(b)) => Unknown(Expr::or(a, b)),
                }
            }
            Operator::Not(x) => {
                use ShortCircuit::*;
                match x {
                    Known(k) => Known(!k),
                    Unknown(u) => Unknown(Expr::not(u)),
                }
            }
        }
    }
}

impl<'a, N: 'a, M: 'a, C: 'a> Functor for ExprLayer<'a, PartiallyApplied, N, M, C> {
    type Layer<X> = ExprLayer<'a, X, N, M, C>;

    fn fmap<F, A, B>(input: Self::Layer<A>, mut f: F) -> Self::Layer<B>
    where
        F: FnMut(A) -> B,
    {
        use self::Operator::*;
        use ExprLayer::*;
        match input {
            Operator(o) => Operator(match o {
                Not(a) => Not(f(a)),
                And(a, b) => And(f(a), f(b)),
                Or(a, b) => Or(f(a), f(b)),
            }),
            Name(x) => Name(x),
            Metadata(x) => Metadata(x),
            Contents(x) => Contents(x),
        }
    }
}

impl<'a, N: 'a, M: 'a, C: 'a> Recursive for &'a Expr<N, M, C> {
    type FunctorToken = ExprLayer<'a, PartiallyApplied, N, M, C>;

    fn into_layer(self) -> <Self::FunctorToken as Functor>::Layer<Self> {
        match self {
            Expr::Not(x) => ExprLayer::Operator(Operator::Not(x)),
            Expr::And(a, b) => ExprLayer::Operator(Operator::And(a, b)),
            Expr::Or(a, b) => ExprLayer::Operator(Operator::Or(a, b)),
            Expr::Name(n) => ExprLayer::Name(n),
            Expr::Metadata(m) => ExprLayer::Metadata(m),
            Expr::Contents(c) => ExprLayer::Contents(c),
        }
    }
}

// for use in recursion visualizations
impl<'a, N: Display, M: Display, C: Display> Display for ExprLayer<'a, (), N, M, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Operator(o) => match o {
                Operator::Not(_) => write!(f, "NOT"),
                Operator::And(a, b) => {
                    write!(f, "AND")
                }
                Operator::Or(a, b) => {
                    write!(f, "OR")
                }
            },
            Self::Name(arg0) => write!(f, "{}", arg0),
            Self::Metadata(arg0) => write!(f, "{}", arg0),
            Self::Contents(arg0) => write!(f, "{}", arg0),
        }
    }
}
