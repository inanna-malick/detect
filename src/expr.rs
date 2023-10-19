pub mod frame;
pub mod short_circuit;

use std::fmt::Display;

use crate::expr::frame::ExprFrame;
use crate::predicate::Predicate;
pub(crate) use crate::predicate::{ContentPredicate, MetadataPredicate, NamePredicate};
use recursion::CollapsibleExt;
use recursion_visualize::visualize::*;

use self::short_circuit::ShortCircuit;

/// Filesystem entity matcher expression with boolean logic and predicates
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

impl<A: Display> Display for Expr<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Not(a) => write!(f, "!{}", a),
            Self::And(a, b) => {
                write!(f, "{} && {}", a, b)
            }
            Self::Or(a, b) => {
                write!(f, "{} || {}", a, b)
            }
            Self::Predicate(arg0) => write!(f, "{}", arg0),
            Self::Literal(arg0) => write!(f, "{}", arg0),
        }
    }
}

impl<A: Display + Clone> Expr<A> {
    pub fn and(a: Self, b: Self) -> Self {
        Self::And(Box::new(a), Box::new(b))
    }
    pub fn or(a: Self, b: Self) -> Self {
        Self::Or(Box::new(a), Box::new(b))
    }
    pub fn not(a: Self) -> Self {
        Self::Not(Box::new(a))
    }

    pub fn map_predicate_v<B: Display + Clone>(
        &self,
        vopt: &mut Option<Viz>,
        header: String,
        text: String,
        f: impl Fn(A) -> ShortCircuit<B>,
    ) -> Expr<B> {
        let (res, viz) = self.collapse_frames_v(|e| match e {
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
        });

        if let Some(v) = vopt.take() {
            let vv = v.fuse(viz, header, text);
            *vopt = Some(vv);
        } else {
            *vopt = Some(viz.label(header, text))
        }

        res
    }

    //     // apply some potentially short circuiting transformation to all predicates in
    //     // this expression, with the goal of eventually eliminating all
    //     pub fn reduce_predicates_and_short_circuit_<B>(
    //         &self,
    //         f: impl Fn(A) -> ShortCircuit<B>,
    //     ) -> Expr<B> {
    //         self.collapse_frames(|e| match e {
    //             // apply 'f' to Predicate expressions
    //             ExprFrame::Predicate(p) => match f(p) {
    //                 ShortCircuit::Known(b) => Expr::Literal(b),
    //                 ShortCircuit::Unknown(p) => Expr::Predicate(p),
    //             },
    //             // reduce And expressions
    //             ExprFrame::And(Expr::Literal(false), _) => Expr::Literal(false),
    //             ExprFrame::And(_, Expr::Literal(false)) => Expr::Literal(false),
    //             ExprFrame::And(x, Expr::Literal(true)) => x,
    //             ExprFrame::And(Expr::Literal(true), x) => x,
    //             ExprFrame::And(a, b) => Expr::And(Box::new(a), Box::new(b)),
    //             // reduce Or expressions
    //             ExprFrame::Or(Expr::Literal(true), _) => Expr::Literal(true),
    //             ExprFrame::Or(_, Expr::Literal(true)) => Expr::Literal(true),
    //             ExprFrame::Or(x, Expr::Literal(false)) => x,
    //             ExprFrame::Or(Expr::Literal(false), x) => x,
    //             ExprFrame::Or(a, b) => Expr::Or(Box::new(a), Box::new(b)),
    //             // reduce Not expressions
    //             ExprFrame::Not(Expr::Literal(k)) => Expr::Literal(!k),
    //             ExprFrame::Not(x) => Expr::Not(Box::new(x)),
    //             // Literal expressions are unchanged
    //             ExprFrame::Literal(x) => Expr::Literal(x),
    //         })
    //     }

    //     // apply some potentially short circuiting transformation to all predicates in
    //     // this expression, with the goal of eventually eliminating all
    //     pub fn reduce_predicates_and_short_circuit<B>(
    //         &self,
    //         f: impl Fn(A) -> ShortCircuit<B>,
    //     ) -> Expr<B> {
    //         match self {
    //             // apply 'f' to Predicate expressions
    //             Expr::Predicate(p) => match f(p.clone()) {
    //                 ShortCircuit::Known(bool) => Expr::Literal(bool),
    //                 ShortCircuit::Unknown(p) => Expr::Predicate(p),
    //             },
    //             // reduce And expressions
    //             Expr::And(a, b) => match (&**a, &**b) {
    //                 (Expr::Literal(false), _) => Expr::Literal(false),
    //                 (_, Expr::Literal(false)) => Expr::Literal(false),
    //                 (x, Expr::Literal(true)) => x.reduce_predicates_and_short_circuit(f),
    //                 (Expr::Literal(true), x) => x.reduce_predicates_and_short_circuit(f),
    //                 (a, b) => Expr::And(
    //                     Box::new(a.reduce_predicates_and_short_circuit(&f)),
    //                     Box::new(b.reduce_predicates_and_short_circuit(&f)),
    //                 ),
    //             },
    //             // reduce Or expressions
    //             Expr::Or(a, b) => match (&**a, &**b) {
    //                 (Expr::Literal(true), _) => Expr::Literal(true),
    //                 (_, Expr::Literal(true)) => Expr::Literal(true),
    //                 (x, Expr::Literal(false)) => x.reduce_predicates_and_short_circuit(f),
    //                 (Expr::Literal(false), x) => x.reduce_predicates_and_short_circuit(f),
    //                 (a, b) => Expr::Or(
    //                     Box::new(a.reduce_predicates_and_short_circuit(&f)),
    //                     Box::new(b.reduce_predicates_and_short_circuit(&f)),
    //                 ),
    //             },
    //             // reduce Not expressions
    //             Expr::Not(a) => match &**a {
    //                 Expr::Literal(x) => Expr::Literal(!x),
    //                 x => Expr::Not(Box::new(x.reduce_predicates_and_short_circuit(&f))),
    //             },
    //             // Literal expressions are unchanged
    //             Expr::Literal(x) => Expr::Literal(*x),
    //         }
    //     }
}
