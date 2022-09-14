use recursion::map_layer::MapLayer;

#[derive(Debug, Eq, PartialEq)]
pub enum Operator<Recurse> {
    Not(Recurse),
    And(Recurse, Recurse),
    Or(Recurse, Recurse),
}
use Operator::*;

impl Operator<Option<bool>> {
    pub(crate) fn eval(&self) -> Option<bool> {
        match self {
            And(Some(false), _) | And(_, Some(false)) => Some(false),
            And(Some(true), Some(true)) => Some(true),
            Or(Some(true), _) | Or(_, Some(true)) => Some(true),
            Or(Some(false), Some(false)) => Some(false),
            Not(Some(x)) => Some(!x),
            _ => None,
        }
    }
}

impl<X> Operator<X> {
    pub(crate) fn as_ref_op(&self) -> Operator<&X> {
        match self {
            Not(a) => Not(a),
            And(a, b) => And(a, b),
            Or(a, b) => Or(a, b),
        }
    }
}

impl<A, B> MapLayer<B> for Operator<A> {
    type Unwrapped = A;
    type To = Operator<B>;
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        match self {
            Not(a) => Not(f(a)),
            And(a, b) => And(f(a), f(b)),
            Or(a, b) => Or(f(a), f(b)),
        }
    }
}
