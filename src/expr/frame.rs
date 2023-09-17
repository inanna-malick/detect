use recursion_schemes::frame::MappableFrame;
use std::fmt::{Debug, Display};

/// short-lived single layer of a filesystem entity matcher expression, used for
/// expressing recursive algorithms over a single layer of a borrowed Expr
pub enum ExprFrame<'a, X, P> {
    // boolean operators
    Operator(Operator<X>),
    // borrowed predicate
    Predicate(&'a P),
}

// having operator as a distinct type might seem a bit odd, but it lets us
// factor out the short circuiting logic
#[derive(Debug, Eq, PartialEq)]
pub enum Operator<Recurse> {
    Not(Recurse),
    And(Recurse, Recurse),
    Or(Recurse, Recurse),
}
pub enum PartiallyApplied {}

impl<'a, P: 'a> MappableFrame for ExprFrame<'a, PartiallyApplied, P> {
    type Frame<X> = ExprFrame<'a, X, P>;

    fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
        use self::Operator::*;
        use ExprFrame::*;
        match input {
            Operator(o) => Operator(match o {
                Not(a) => Not(f(a)),
                And(a, b) => And(f(a), f(b)),
                Or(a, b) => Or(f(a), f(b)),
            }),

            Predicate(p) => Predicate(p),
        }
    }
}

// for use in recursion visualizations
impl<'a, P: Display> Display for ExprFrame<'a, (), P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Operator(o) => match o {
                Operator::Not(_) => write!(f, "NOT"),
                Operator::And(_, _) => {
                    write!(f, "AND")
                }
                Operator::Or(_, _) => {
                    write!(f, "OR")
                }
            },
            Self::Predicate(arg0) => write!(f, "{}", arg0),
        }
    }
}
