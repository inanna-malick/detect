pub mod frame;
pub mod short_circuit;

use std::{io, sync::Arc};

use crate::predicate::Predicate;
pub(crate) use crate::predicate::{ContentPredicate, MetadataPredicate, NamePredicate};
use crate::{expr::frame::ExprFrame, predicate::ProcessPredicate};
use futures::{future::BoxFuture, FutureExt, TryFutureExt};
use recursion::CollapsibleExt;

use self::short_circuit::ShortCircuit;

/// Filesystem entity matcher expression with boolean logic and predicates
#[derive(Debug, PartialEq, Eq)]
pub enum Expr<Predicate> {
    // boolean operators
    Not(Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    // predicates
    Predicate(Predicate),
    // literal boolean values
    Literal(bool),
}

impl<A, B, C, D> Expr<Predicate<A, B, C, D>> {
    pub fn name_predicate(x: A) -> Self {
        Self::Predicate(Predicate::Name(Arc::new(x)))
    }
    pub fn meta_predicate(x: B) -> Self {
        Self::Predicate(Predicate::Metadata(Arc::new(x)))
    }
    pub fn content_predicate(x: C) -> Self {
        Self::Predicate(Predicate::Content(Arc::new(x)))
    }
    pub fn process_predicate(x: D) -> Self {
        Self::Predicate(Predicate::Process(Arc::new(x)))
    }
}

impl<P: Clone> Expr<P> {
    pub fn reduce_predicate_and_short_circuit<B: Clone>(
        &self,
        f: impl Fn(P) -> ShortCircuit<B>,
    ) -> Expr<B> {
        self.collapse_frames(|e| match e {
            // apply 'f' to Predicate expressions
            ExprFrame::Predicate(p) => match f(p) {
                ShortCircuit::Known(b) => Expr::Literal(b),
                ShortCircuit::Unknown(p) => Expr::Predicate(p),
            },
            // reduce And expressions
            ExprFrame::And(Expr::Literal(false), _) => Expr::Literal(false),
            ExprFrame::And(_, Expr::Literal(false)) => Expr::Literal(false),
            ExprFrame::And(x, Expr::Literal(true)) => x,
            ExprFrame::And(Expr::Literal(true), x) => x,
            ExprFrame::And(a, b) => Expr::And(Box::new(a), Box::new(b)),
            // reduce Or expressions
            ExprFrame::Or(Expr::Literal(true), _) => Expr::Literal(true),
            ExprFrame::Or(_, Expr::Literal(true)) => Expr::Literal(true),
            ExprFrame::Or(x, Expr::Literal(false)) => x,
            ExprFrame::Or(Expr::Literal(false), x) => x,
            ExprFrame::Or(a, b) => Expr::Or(Box::new(a), Box::new(b)),
            // reduce Not expressions
            ExprFrame::Not(Expr::Literal(k)) => Expr::Literal(!k),
            ExprFrame::Not(x) => Expr::Not(Box::new(x)),
            // Literal expressions are unchanged
            ExprFrame::Literal(x) => Expr::Literal(x),
        })
    }

    pub fn and(a: Self, b: Self) -> Self {
        Self::And(Box::new(a), Box::new(b))
    }
    pub fn or(a: Self, b: Self) -> Self {
        Self::Or(Box::new(a), Box::new(b))
    }
    pub fn not(a: Self) -> Self {
        Self::Not(Box::new(a))
    }
}

impl<P> Expr<P>
where
    P: Clone + Sync + Send + 'static,
{
    pub async fn reduce_predicate_and_short_circuit_async<'a, B: Send + Sync + 'static>(
        &'a self,
        f: impl Fn(P) -> BoxFuture<'a, io::Result<ShortCircuit<B>>> + Send + Sync,
    ) -> std::io::Result<Expr<B>> {
        use futures::future::ok;
        use recursion::experimental::recursive::collapse::CollapsibleAsync;
        let res = self
            .collapse_frames_async(|e| match e {
                ExprFrame::Predicate(p) => f(p)
                    .map_ok(|x| match x {
                        ShortCircuit::Known(b) => Expr::Literal(b),
                        ShortCircuit::Unknown(p) => Expr::Predicate(p),
                    })
                    .boxed(),
                x => ok(match x {
                    // reduce And expressions
                    ExprFrame::And(Expr::Literal(false), _) => Expr::Literal(false),
                    ExprFrame::And(_, Expr::Literal(false)) => Expr::Literal(false),
                    ExprFrame::And(x, Expr::Literal(true)) => x,
                    ExprFrame::And(Expr::Literal(true), x) => x,
                    ExprFrame::And(a, b) => Expr::And(Box::new(a), Box::new(b)),
                    // reduce Or expressions
                    ExprFrame::Or(Expr::Literal(true), _) => Expr::Literal(true),
                    ExprFrame::Or(_, Expr::Literal(true)) => Expr::Literal(true),
                    ExprFrame::Or(x, Expr::Literal(false)) => x,
                    ExprFrame::Or(Expr::Literal(false), x) => x,
                    ExprFrame::Or(a, b) => Expr::Or(Box::new(a), Box::new(b)),
                    // reduce Not expressions
                    ExprFrame::Not(Expr::Literal(k)) => Expr::Literal(!k),
                    ExprFrame::Not(x) => Expr::Not(Box::new(x)),
                    // Literal expressions are unchanged
                    ExprFrame::Literal(x) => Expr::Literal(x),
                    ExprFrame::Predicate(_) => unreachable!("handled above"),
                })
                .boxed(),
            })
            .await?;

        Ok(res)
    }
}
