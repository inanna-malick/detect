
use recursion::Collapsible;
#[cfg(feature = "viz")]
use std::fmt::Display;
#[cfg(feature = "viz")]
use recursion_visualize::visualize::CollapsableV;

use super::frame::{ExprFrame, Operator, PartiallyApplied};
use super::Expr;

impl<'a, P: 'a> Collapsible for &'a Expr<P> {
    type FrameToken = ExprFrame<'a, PartiallyApplied, P>;

    fn into_frame(self) -> ExprFrame<'a, Self, P> {
        match self {
            Expr::Not(x) => ExprFrame::Operator(Operator::Not(x)),
            Expr::And(a, b) => ExprFrame::Operator(Operator::And(a, b)),
            Expr::Or(a, b) => ExprFrame::Operator(Operator::Or(a, b)),
            Expr::Predicate(p) => ExprFrame::Predicate(p),
        }
    }
}

#[cfg(feature = "viz")]
impl<'a, P: 'a + Display> CollapsableV for &'a Expr<P> {
}