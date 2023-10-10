use std::fmt::Display;

use recursion::Collapsible;

use crate::predicate::Predicate;

use super::frame::{ExprFrame, PartiallyApplied};
use super::Expr;

// TODO: fold predicate wrapper into expr?

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
