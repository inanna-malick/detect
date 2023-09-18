use std::fmt::Display;

use crate::expr::frame::Operator;

use super::Expr;

pub enum ShortCircuit<X> {
    Known(bool),
    Unknown(X),
}

impl<X: Display> Display for ShortCircuit<X> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShortCircuit::Known(x) => write!(f, "known: {}", x),
            ShortCircuit::Unknown(x) => write!(f, "unknown: {}", x),
        }
    }
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
