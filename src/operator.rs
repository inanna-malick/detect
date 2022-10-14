use crate::expr::Expr;
use recursion::map_layer::MapLayer;

#[derive(Debug, Eq, PartialEq)]
pub enum Operator<Recurse> {
    Not(Recurse),
    And(Vec<Recurse>),
    Or(Vec<Recurse>),
}

impl<A, B, C> Operator<Expr<A, B, C>> {
    pub(crate) fn eval(&self) -> Option<bool> {
        match self {
            Operator::And(ands) => {
                if ands.iter().any(|b| b.known() == Some(false)) {
                    Some(false)
                } else if ands.iter().all(|b| b.known() == Some(true)) {
                    Some(true)
                } else {
                    None
                }
            }
            Operator::Or(ors) => {
                if ors.iter().any(|b| b.known() == Some(true)) {
                    Some(true)
                } else if ors.iter().all(|b| b.known() == Some(false)) {
                    Some(false)
                } else {
                    None
                }
            }
            Operator::Not(x) => x.known().map(|b| !b),
        }
    }
}

impl<X> Operator<Box<X>> {
    pub(crate) fn as_ref_op(&self) -> Operator<&X> {
        use Operator::*;
        match self {
            Not(a) => Not(a),
            And(xs) => And(xs.iter().map(Box::as_ref).collect()),
            Or(xs) => Or(xs.iter().map(Box::as_ref).collect()),
        }
    }
}

impl<A, B> MapLayer<B> for Operator<A> {
    type Unwrapped = A;
    type To = Operator<B>;
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        use Operator::*;
        match self {
            Not(a) => Not(f(a)),
            And(xs) => And(xs.into_iter().map(f).collect()),
            Or(xs) => Or(xs.into_iter().map(f).collect()),
        }
    }
}
