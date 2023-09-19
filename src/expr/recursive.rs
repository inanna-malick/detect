use std::fmt::Display;

use recursion_schemes::recursive::collapse::Collapsable;
use recursion_schemes::recursive::HasRecursiveFrame;
use recursion_visualize::visualize::CollapsableV;

use super::frame::{ExprFrame, Operator, PartiallyApplied};
use super::Expr;

impl<'a, P: 'a> HasRecursiveFrame for &'a Expr<P> {
    type FrameToken = ExprFrame<'a, PartiallyApplied, P>;
}

impl<'a, P: 'a> Collapsable for &'a Expr<P> {

    fn into_frame(self) -> ExprFrame<'a, Self, P> {
        match self {
            Expr::Not(x) => ExprFrame::Operator(Operator::Not(x)),
            Expr::And(a, b) => ExprFrame::Operator(Operator::And(a, b)),
            Expr::Or(a, b) => ExprFrame::Operator(Operator::Or(a, b)),
            Expr::Predicate(p) => ExprFrame::Predicate(p),
        }
    }
}

impl<'a, P: 'a + Display> CollapsableV for &'a Expr<P> {
}