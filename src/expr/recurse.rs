use recursion_schemes::recursive::Recursive;
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

impl<P> Operator<ShortCircuit<Expr<P>>> {
    // TODO: docstrings
    pub fn attempt_short_circuit(self) -> ShortCircuit<Expr<P>> {
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


impl<'a, P: 'a> Recursive for &'a Expr<P> {
    type Frame<X> = ExprLayer<'a, X, P>;

    fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
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

    fn into_frame(self) -> Self::Frame<Self> {
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
                Operator::And(_, _) => {
                    write!(f, "AND")
                }
                Operator::Or(_, _) => {
                    write!(f, "OR")
                }
            },
            Self::Predicate(arg0) => write!(f, "{}", arg0),
        }
    }
}
