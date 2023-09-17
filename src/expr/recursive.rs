use recursion_schemes::recursive::collapse::IntoRecursiveFrame;
use recursion_schemes::recursive::HasRecursiveFrame;

use super::frame::{ExprFrame, Operator, PartiallyApplied};
use super::Expr;

impl<'a, P: 'a> HasRecursiveFrame for &'a Expr<P> {
    type FrameToken = ExprFrame<'a, PartiallyApplied, P>;
}

impl<'a, P: 'a> IntoRecursiveFrame for &'a Expr<P> {
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
