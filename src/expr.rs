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
pub enum Expr<
    Name = NamePredicate,
    Metadata = MetadataPredicate,
    Content = ContentPredicate,
    Process = ProcessPredicate,
> {
    // boolean operators
    Not(Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    // predicates
    Predicate(Predicate<Name, Metadata, Content, Process>),
    // literal boolean values
    Literal(bool),
}

impl<A, B, C, D> Expr<A, B, C, D> {
    pub fn and(a: Self, b: Self) -> Self {
        Self::And(Box::new(a), Box::new(b))
    }
    pub fn or(a: Self, b: Self) -> Self {
        Self::Or(Box::new(a), Box::new(b))
    }
    pub fn not(a: Self) -> Self {
        Self::Not(Box::new(a))
    }
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

    pub fn map_predicate<A1, B1, C1, D1>(
        &self,
        f: impl Fn(Predicate<A, B, C, D>) -> ShortCircuit<Predicate<A1, B1, C1, D1>>,
    ) -> Expr<A1, B1, C1, D1> {
        self.collapse_frames(|e| match e {
            ExprFrame::Predicate(p) => match f(p) {
                ShortCircuit::Known(b) => Expr::Literal(b),
                ShortCircuit::Unknown(p) => Expr::Predicate(p),
            },
            ExprFrame::Not(a) => Expr::not(a),
            ExprFrame::And(a, b) => Expr::and(a, b),
            ExprFrame::Or(a, b) => Expr::or(a, b),
            ExprFrame::Literal(b) => Expr::Literal(b),
        })
    }

    pub fn reduce(&self) -> Expr<A, B, C, D> {
        self.collapse_frames(|e| match e {
            ExprFrame::And(a, b) => match (a, b) {
                (Expr::Literal(false), _) => Expr::Literal(false),
                (_, Expr::Literal(false)) => Expr::Literal(false),
                (x, Expr::Literal(true)) => x,
                (Expr::Literal(true), x) => x,
                (a, b) => Expr::and(a, b),
            },
            ExprFrame::Or(a, b) => match (a, b) {
                (Expr::Literal(true), _) => Expr::Literal(true),
                (_, Expr::Literal(true)) => Expr::Literal(true),
                (x, Expr::Literal(false)) => x,
                (Expr::Literal(false), x) => x,
                (a, b) => Expr::or(a, b),
            },
            ExprFrame::Not(x) => match x {
                Expr::Literal(k) => Expr::Literal(!k),
                x => Expr::not(x),
            },
            // leave literals and predicates unchanged
            ExprFrame::Predicate(p) => Expr::Predicate(p),
            ExprFrame::Literal(b) => Expr::Literal(b),
        })
    }
}

impl<A, B, C, D> Expr<A, B, C, D>
where
    A: Sync + Send + 'static,
    B: Sync + Send + 'static,
    C: Sync + Send + 'static,
    D: Sync + Send + 'static,
{
    pub async fn map_predicate_async<
        'a,
        A1: Send + Sync + 'static,
        B1: Send + Sync + 'static,
        C1: Send + Sync + 'static,
        D1: Send + Sync + 'static,
    >(
        &'a self,
        f: impl Fn(
                Predicate<A, B, C, D>,
            ) -> BoxFuture<'a, io::Result<ShortCircuit<Predicate<A1, B1, C1, D1>>>>
            + Send
            + Sync,
    ) -> std::io::Result<Expr<A1, B1, C1, D1>> {
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
                ExprFrame::Not(a) => ok(Expr::not(a)).boxed(),
                ExprFrame::And(a, b) => ok(Expr::and(a, b)).boxed(),
                ExprFrame::Or(a, b) => ok(Expr::or(a, b)).boxed(),
                ExprFrame::Literal(b) => ok(Expr::Literal(b)).boxed(),
            })
            .await?;

        Ok(res)
    }
}
