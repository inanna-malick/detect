use super::Expr;
use crate::predicate::Predicate;
use futures::FutureExt;
use recursion::experimental::recursive::collapse::CollapsibleAsync;
use recursion::{
    experimental::frame::AsyncMappableFrame, Collapsible, MappableFrame, PartiallyApplied,
};
use std::fmt::Display;
use tokio::try_join;

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

async fn map_frame_async<'a, A, B, E, P>(
    input: ExprFrame<A, P>,
    f: impl Fn(A) -> futures::future::BoxFuture<'a, Result<B, E>> + Send + Sync + 'a,
) -> Result<ExprFrame<B, P>, E>
where
    E: Send + 'a,
    A: Send + 'a,
    B: Send + 'a,
{
    use ExprFrame::*;
    match input {
        Not(a) => Ok(Not(f(a).await?)),
        And(a, b) => {
            let (a, b) = try_join!(f(a), f(b))?;
            Ok(And(a, b))
        }
        Or(a, b) => {
            let (a, b) = try_join!(f(a), f(b))?;
            Ok(Or(a, b))
        }
        Predicate(p) => Ok(Predicate(p)),
        Literal(bool) => Ok(Literal(bool)),
    }
}

impl<P: Send + Sync + 'static> AsyncMappableFrame for ExprFrame<PartiallyApplied, P> {
    fn map_frame_async<'a, A, B, E>(
        input: Self::Frame<A>,
        f: impl Fn(A) -> futures::future::BoxFuture<'a, Result<B, E>> + Send + Sync + 'a,
    ) -> futures::future::BoxFuture<'a, Result<Self::Frame<B>, E>>
    where
        E: Send + 'a,
        A: Send + 'a,
        B: Send + 'a,
    {
        map_frame_async(input, f).boxed()
    }
}

impl<'a, P: Clone> Collapsible for &'a Expr<P> {
    type FrameToken = ExprFrame<PartiallyApplied, P>;

    fn into_frame(self) -> ExprFrame<Self, P> {
        match self {
            Expr::Not(x) => ExprFrame::Not(x),
            Expr::And(a, b) => ExprFrame::And(a, b),
            Expr::Or(a, b) => ExprFrame::Or(a, b),
            Expr::Predicate(p) => ExprFrame::Predicate((*p).clone()),
            Expr::Literal(b) => ExprFrame::Literal(*b),
        }
    }
}

impl<'a, P: Clone + Send + Sync + 'static> CollapsibleAsync for &'a Expr<P> {
    type AsyncFrameToken = ExprFrame<PartiallyApplied, P>;
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
