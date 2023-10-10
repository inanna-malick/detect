pub mod frame;
pub mod recursive;
pub mod short_circuit;

use crate::expr::frame::ExprFrame;
use crate::predicate::Predicate;
pub(crate) use crate::predicate::{ContentPredicate, MetadataPredicate, NamePredicate};
use recursion::CollapsibleExt;
use std::fmt::Display;

use self::short_circuit::ShortCircuit;

/// Filesystem entity matcher expression with boolean logic and predicates
pub enum Expr<Name = NamePredicate, Metadata = MetadataPredicate, Content = ContentPredicate> {
    // boolean operators
    Not(Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    // predicates
    Predicate(Predicate<Name, Metadata, Content>),
    // literal boolean values
    Literal(bool),
}

impl<A, B, C> Expr<A, B, C> {
    pub fn and(a: Self, b: Self) -> Self {
        Self::And(Box::new(a), Box::new(b))
    }
    pub fn or(a: Self, b: Self) -> Self {
        Self::Or(Box::new(a), Box::new(b))
    }
    pub fn not(a: Self) -> Self {
        Self::Not(Box::new(a))
    }

    pub fn map_predicate<A1, B1, C1>(
        &self,
        f: impl Fn(Predicate<A, B, C>) -> ShortCircuit<Predicate<A1, B1, C1>>,
    ) -> Expr<A1, B1, C1> {
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

    pub fn reduce(&self) -> Expr<A, B, C> {
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

// impl<P: Display> Display for Expr<P> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::Not(x) => write!(f, "!{}", x),
//             Self::And(a, b) => {
//                 write!(f, "{} && {}", a, b)
//             }
//             Self::Or(a, b) => {
//                 write!(f, "{} || {}", a, b)
//             }
//             Self::Predicate(arg0) => write!(f, "{}", arg0),
//         }
//     }
// }
