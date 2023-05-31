pub(crate) use crate::predicate::{ContentPredicate, MetadataPredicate, NamePredicate};
use itertools::*;
use recursion_schemes::{
    functor::{Functor, PartiallyApplied},
    recursive::{Recursive, Base, BaseFunctor},
};
use std::fmt::{Debug, Display};

use super::Expr;

/// short-lived single layer of a filesystem entity matcher expression, used for
/// expressing recursive algorithms over a single layer of a borrowed Expr
pub enum ExprLayer<'a, Recurse, P> {
    // boolean operators
    Operator(Operator<Recurse>),
    // borrowed predicate
    Predicate(&'a P),
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

impl<P> From<Operator<Self>> for Expr<P> {
    fn from(value: Operator<Expr<P>>) -> Self {
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

impl<P> Operator<ShortCircuit<Expr<P>>> {
    pub fn attempt_short_circuit(self) -> ShortCircuit<Expr<P>> {
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

impl<'a, P: 'a> Functor for ExprLayer<'a, PartiallyApplied, P> {

    type Layer<X> = ExprLayer<'a, X, P>;

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

            Predicate(p) => Predicate(p),
        }
    }
}

impl<'a, P: 'a> Base for &'a Expr<P> {
    type MappableFrame = ExprLayer<'a, PartiallyApplied, P>;
}

impl<'a, P: 'a> Recursive for &'a Expr<P> {

    fn into_layer(self) -> BaseFunctor<Self, Self> {
        match self {
            Expr::Not(x) => ExprLayer::Operator(Operator::Not(x)),
            Expr::And(a, b) => ExprLayer::Operator(Operator::And(a, b)),
            Expr::Or(a, b) => ExprLayer::Operator(Operator::Or(a, b)),
            Expr::Predicate(p) => ExprLayer::Predicate(p),
        }
    }
}

// for use in recursion visualizations
impl<'a, P: Display> Display for ExprLayer<'a, (), P> {
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
            Self::Predicate(arg0) => write!(f, "{}", arg0),
        }
    }
}
