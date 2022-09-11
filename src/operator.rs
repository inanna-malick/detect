use recursion::map_layer::MapLayer;

use crate::expr::ExprTree;

#[derive(Debug, Eq, PartialEq)]
pub enum Operator<Recurse> {
    Not(Recurse),
    And(Vec<Recurse>),
    Or(Vec<Recurse>),
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

impl<A, B, C> Operator<ExprTree<A, B, C>> {
    pub(crate) fn short_circuit(&self) -> Option<bool> {
        match self {
            Operator::And(ands)
                if ands
                    .iter()
                    .any(|b: &ExprTree<_, _, _>| b.known() == Some(false)) =>
            {
                Some(false)
            }
            Operator::Or(xs)
                if xs
                    .iter()
                    .any(|b: &ExprTree<_, _, _>| b.known() == Some(true)) =>
            {
                Some(true)
            }
            x => match x.known() {
                None => None,
                Some(o) => Some(o.eval()),
            },
        }
    }

    fn known(&self) -> Option<Operator<bool>> {
        match self {
            Operator::Not(a) => a.known().map(Operator::Not),
            Operator::And(xs) => xs
                .iter()
                .map(|x| x.known())
                .collect::<Option<_>>()
                .map(Operator::And),
            Operator::Or(xs) => xs
                .iter()
                .map(|x| x.known())
                .collect::<Option<_>>()
                .map(Operator::Or),
        }
    }
}

impl Operator<bool> {
    fn eval(self) -> bool {
        use Operator::*;
        match self {
            Not(x) => !x,
            And(xs) => xs.into_iter().all(|b| b),
            Or(xs) => xs.into_iter().any(|b| b),
        }
    }
}

impl<X> Operator<X> {
    pub(crate) fn as_ref_op(&self) -> Operator<&X> {
        use Operator::*;
        match self {
            Not(a) => Not(a),
            And(xs) => And(xs.iter().collect()),
            Or(xs) => Or(xs.iter().collect()),
        }
    }
}
