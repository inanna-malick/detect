pub(crate) use crate::predicate::{ContentPredicate, MetadataPredicate, NamePredicate};
use itertools::*;
use recursion_schemes::{
    functor::{Functor, PartiallyApplied},
    recursive::Recursive,
};
use std::fmt::{Debug, Display};

use super::Expr;

/// short-lived single layer of a filesystem entity matcher expression, used for
/// expressing recursive algorithms over a single layer of a borrowed Expr
pub enum ExprLayer<
    'a,
    Recurse,
    Name = NamePredicate,
    Metadata = MetadataPredicate,
    Contents = ContentPredicate,
> {
    // boolean literals
    KnownResult(bool),
    // boolean operators
    Operator(Operator<Recurse>),
    // borrowed predicates
    Name(&'a Name),
    Metadata(&'a Metadata),
    Contents(&'a Contents),
}

// having operator as a distinct type might seem a bit odd, but it lets us
// factor out the short circuiting logic
#[derive(Debug, Eq, PartialEq)]
pub enum Operator<Recurse> {
    Not(Recurse),
    And(Vec<Recurse>),
    Or(Vec<Recurse>),
}

pub trait ThreeValued {
    fn as_bool(&self) -> Option<bool>;
    fn lift_bool(b: bool) -> Self;

    fn is_false(&self) -> bool {
        self.as_bool().is_some_and(|x| !x)
    }

    fn is_true(&self) -> bool {
        self.as_bool().is_some_and(|x| x)
    }
}

impl<A, B, C> ThreeValued for Expr<A, B, C> {
    fn as_bool(&self) -> Option<bool> {
        match self {
            Expr::KnownResult(b) => Some(*b),
            _ => None,
        }
    }

    fn lift_bool(b: bool) -> Self {
        Expr::KnownResult(b)
    }
}

impl<A, B, C> From<Operator<Self>> for Expr<A, B, C> {
    fn from(value: Operator<Expr<A, B, C>>) -> Self {
        match value {
            Operator::Not(x) => Self::Not(Box::new(x)),
            Operator::And(xs) => Self::And(xs),
            Operator::Or(xs) => Self::Or(xs),
        }
    }
}

impl<X: ThreeValued + From<Operator<X>>> Operator<X> {
    pub fn attempt_short_circuit(self) -> X {
        // use Expr::*;
        match &self {
            Operator::And(ands) => {
                if ands.iter().any(ThreeValued::is_false) {
                    ThreeValued::lift_bool(false)
                } else if ands.iter().all(ThreeValued::is_true) {
                    ThreeValued::lift_bool(true)
                } else {
                    X::from(self)
                }
            }
            Operator::Or(ors) => {
                if ors.iter().any(ThreeValued::is_false) {
                    ThreeValued::lift_bool(true)
                } else if ors.iter().all(ThreeValued::is_true) {
                    ThreeValued::lift_bool(false)
                } else {
                    X::from(self)
                }
            }
            Operator::Not(x) => match x.as_bool() {
                Some(b) => ThreeValued::lift_bool(!b),
                None => X::from(self),
            },
        }
    }
}

impl<'a, N: 'a, M: 'a, C: 'a> Functor for ExprLayer<'a, PartiallyApplied, N, M, C> {
    type Layer<X> = ExprLayer<'a, X, N, M, C>;

    fn fmap<F, A, B>(input: Self::Layer<A>, mut f: F) -> Self::Layer<B>
    where
        F: FnMut(A) -> B,
    {
        use self::Operator::*;
        use ExprLayer::*;
        match input {
            Operator(o) => Operator(match o {
                Not(a) => Not(f(a)),
                And(xs) => And(xs.into_iter().map(f).collect()),
                Or(xs) => Or(xs.into_iter().map(f).collect()),
            }),
            KnownResult(k) => KnownResult(k),
            Name(x) => Name(x),
            Metadata(x) => Metadata(x),
            Contents(x) => Contents(x),
        }
    }
}

impl<'a, N: 'a, M: 'a, C: 'a> Recursive for &'a Expr<N, M, C> {
    type FunctorToken = ExprLayer<'a, PartiallyApplied, N, M, C>;

    fn into_layer(self) -> <Self::FunctorToken as Functor>::Layer<Self> {
        match self {
            Expr::Not(x) => ExprLayer::Operator(Operator::Not(x)),
            Expr::And(xs) => ExprLayer::Operator(Operator::And(xs.iter().collect())),
            Expr::Or(xs) => ExprLayer::Operator(Operator::Or(xs.iter().collect())),
            Expr::KnownResult(b) => ExprLayer::KnownResult(*b),
            Expr::Name(n) => ExprLayer::Name(n),
            Expr::Metadata(m) => ExprLayer::Metadata(m),
            Expr::Contents(c) => ExprLayer::Contents(c),
        }
    }
}

// for use in recursion visualizations
impl<'a, N: Display, M: Display, C: Display> Display for ExprLayer<'a, (), N, M, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Operator(o) => match o {
                Operator::Not(_) => write!(f, "!_"),
                Operator::And(xs) => {
                    let xs: String =
                        Itertools::intersperse(xs.iter().map(|_| "_"), " && ").collect();
                    write!(f, "{}", xs)
                }
                Operator::Or(xs) => {
                    let xs: String =
                        Itertools::intersperse(xs.iter().map(|_| "_"), " || ").collect();
                    write!(f, "{}", xs)
                }
            },
            Self::KnownResult(b) => {
                write!(f, "{}", b)
            }
            Self::Name(arg0) => write!(f, "{}", arg0),
            Self::Metadata(arg0) => write!(f, "{}", arg0),
            Self::Contents(arg0) => write!(f, "{}", arg0),
        }
    }
}
