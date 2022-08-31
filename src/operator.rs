use futures::future::{join_all, try_join_all, BoxFuture};
use recursion::map_layer::MapLayer;
use std::io;

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

impl<'a> Operator<BoxFuture<'a, io::Result<bool>>> {
    pub(crate) async fn eval_async(self) -> io::Result<bool> {
        use Operator::*;
        match self {
            Not(a) => Ok(!a.await?),
            And(xs) => {
                let xs = try_join_all(xs).await?;
                Ok(xs.into_iter().all(|b| b))
            }
            Or(xs) => {
                let xs = try_join_all(xs).await?;
                Ok(xs.into_iter().any(|b| b))
            }
        }
    }
}

impl Operator<bool> {
    pub(crate) fn eval(self) -> bool {
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
