pub mod frame;
pub mod short_circuit;

use std::{fmt::Display, sync::Arc};

use crate::expr::frame::ExprFrame;
use crate::predicate::Predicate;
pub(crate) use crate::predicate::{MetadataPredicate, NamePredicate};
use frame::MapPredicateRef;
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

impl<P: Display> Display for Expr<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Not(e) => {
                write!(f, "!")?;
                // Add parentheses for complex expressions
                match e.as_ref() {
                    Expr::And(_, _) | Expr::Or(_, _) => {
                        write!(f, "(")?;
                        e.fmt(f)?;
                        write!(f, ")")
                    }
                    _ => e.fmt(f),
                }
            }
            Expr::And(a, b) => {
                // Add parentheses for Or expressions to preserve precedence
                match a.as_ref() {
                    Expr::Or(_, _) => {
                        write!(f, "(")?;
                        a.fmt(f)?;
                        write!(f, ")")?;
                    }
                    _ => a.fmt(f)?,
                }
                write!(f, " && ")?;
                match b.as_ref() {
                    Expr::Or(_, _) => {
                        write!(f, "(")?;
                        b.fmt(f)?;
                        write!(f, ")")
                    }
                    _ => b.fmt(f),
                }
            }
            Expr::Or(a, b) => {
                a.fmt(f)?;
                write!(f, " || ")?;
                b.fmt(f)
            }
            Expr::Predicate(p) => p.fmt(f),
            Expr::Literal(x) => x.fmt(f),
        }
    }
}

impl<A, B, C> Expr<Predicate<A, B, C>> {
    pub fn name_predicate(x: A) -> Self {
        Self::Predicate(Predicate::Name(Arc::new(x)))
    }
    pub fn meta_predicate(x: B) -> Self {
        Self::Predicate(Predicate::Metadata(Arc::new(x)))
    }
    pub fn content_predicate(x: C) -> Self {
        Self::Predicate(Predicate::Content(x))
    }
}

impl<P> Expr<P> {
    pub fn map_predicate_ref<'a, B>(&'a self, f: impl Fn(&'a P) -> B) -> Expr<B> {
        MapPredicateRef(self).collapse_frames(|e| match e {
            // apply 'f' to Predicate expressions
            ExprFrame::Predicate(p) => Expr::Predicate(f(p)),
            ExprFrame::And(a, b) => Expr::and(a, b),
            ExprFrame::Or(a, b) => Expr::or(a, b),
            ExprFrame::Not(a) => Expr::negate(a),
            ExprFrame::Literal(x) => Expr::Literal(x),
        })
    }

    pub fn and(a: Self, b: Self) -> Self {
        Self::And(Box::new(a), Box::new(b))
    }
    pub fn or(a: Self, b: Self) -> Self {
        Self::Or(Box::new(a), Box::new(b))
    }
    pub fn negate(a: Self) -> Self {
        Self::Not(Box::new(a))
    }
}

impl<P: Clone> Expr<P> {
    pub fn map_predicate<B>(self, f: impl Fn(P) -> B) -> Expr<B> {
        self.collapse_frames(|e| match e {
            // apply 'f' to Predicate expressions
            ExprFrame::Predicate(p) => Expr::Predicate(f(p)),
            ExprFrame::And(a, b) => Expr::and(a, b),
            ExprFrame::Or(a, b) => Expr::or(a, b),
            ExprFrame::Not(a) => Expr::negate(a),
            ExprFrame::Literal(x) => Expr::Literal(x),
        })
    }

    pub fn map_predicate_err<E, B>(self, f: impl Fn(P) -> Result<B, E>) -> Result<Expr<B>, E> {
        self.collapse_frames(|e| match e {
            // apply 'f' to Predicate expressions
            ExprFrame::Predicate(p) => Ok(Expr::Predicate(f(p)?)),
            ExprFrame::And(a, b) => Ok(Expr::and(a?, b?)),
            ExprFrame::Or(a, b) => Ok(Expr::or(a?, b?)),
            ExprFrame::Not(a) => Ok(Expr::negate(a?)),
            ExprFrame::Literal(x) => Ok(Expr::Literal(x)),
        })
    }
}

impl<P: Clone> Expr<P> {
    pub fn reduce_predicate_and_short_circuit<B, X: Into<ShortCircuit<B>>>(
        &self,
        f: impl Fn(P) -> X,
    ) -> Expr<B> {
        self.collapse_frames(|e| match e {
            // apply 'f' to Predicate expressions
            ExprFrame::Predicate(p) => match f(p).into() {
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
}
