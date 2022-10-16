use recursion::map_layer::MapLayer;
use std::fmt::Debug;

use super::Expr;

#[derive(Debug, Eq, PartialEq)]
pub enum Operator<Recurse> {
    Not(Recurse),
    And(Vec<Recurse>),
    Or(Vec<Recurse>),
}

impl<A, B, C> Operator<Expr<A, B, C>> {
    pub fn attempt_short_circuit(self) -> Expr<A, B, C> {
        match self {
            Operator::And(ands) => {
                if ands.iter().any(|b| matches!(b, Expr::KnownResult(false))) {
                    Expr::KnownResult(false)
                } else if ands.iter().all(|b| matches!(b, Expr::KnownResult(true))) {
                    Expr::KnownResult(true)
                } else {
                    Expr::And(ands)
                }
            }
            Operator::Or(ors) => {
                if ors.iter().any(|b| matches!(b, Expr::KnownResult(true))) {
                    Expr::KnownResult(true)
                } else if ors.iter().all(|b| matches!(b, Expr::KnownResult(false))) {
                    Expr::KnownResult(false)
                } else {
                    Expr::Or(ors)
                }
            }
            Operator::Not(x) => match x {
                Expr::KnownResult(b) => Expr::KnownResult(!b),
                _ => Expr::Not(Box::new(x)),
            },
        }
    }
}

impl<A, B> MapLayer<B> for Operator<A> {
    type Unwrapped = A;
    type To = Operator<B>;
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        use Operator::*;
        match self {
            Not(a) => Not(f(a)),
            And(xs) => And(xs.into_iter().map(f).collect()),
            Or(xs) => Or(xs.into_iter().map(f).collect()),
        }
    }
}
