use super::Expr;
use crate::predicate::Predicate;
use recursion::{Collapsible, MappableFrame, PartiallyApplied};
use std::fmt::Display;

/// short-lived single layer of a filesystem entity matcher expression, used for
/// expressing recursive algorithms over a single layer of a borrowed Expr
pub enum ExprFrame<X, P> {
    // TODO: replace 'P' with 'A', 'B', 'C'
    // borrowed predicate
    Predicate(P),
    // boolean operators
    Not(X),
    And(X, X),
    Or(X, X),
    // literal values
    Literal(bool),
}

impl<P> MappableFrame for ExprFrame<PartiallyApplied, P> {
    type Frame<X> = ExprFrame<X, P>;

    fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
        use ExprFrame::*;
        match input {
            Not(a) => Not(f(a)),
            And(a, b) => And(f(a), f(b)),
            Or(a, b) => Or(f(a), f(b)),
            Predicate(p) => Predicate(p),
            Literal(bool) => Literal(bool),
        }
    }
}

impl<'a, A, B, C> Collapsible for &'a Expr<A, B, C> {
    type FrameToken = ExprFrame<PartiallyApplied, Predicate<A, B, C>>;

    fn into_frame(self) -> ExprFrame<Self, Predicate<A, B, C>> {
        match self {
            Expr::Not(x) => ExprFrame::Not(x),
            Expr::And(a, b) => ExprFrame::And(a, b),
            Expr::Or(a, b) => ExprFrame::Or(a, b),
            Expr::Predicate(p) => ExprFrame::Predicate((*p).clone()),
            Expr::Literal(b) => ExprFrame::Literal(*b),
        }
    }
}

// for use in recursion visualizations
impl<P: Display> Display for ExprFrame<(), P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Not(_) => write!(f, "NOT"),
            Self::And(_, _) => {
                write!(f, "AND")
            }
            Self::Or(_, _) => {
                write!(f, "OR")
            }
            Self::Predicate(arg0) => write!(f, "{}", arg0),
            Self::Literal(arg0) => write!(f, "{}", arg0),
        }
    }
}
