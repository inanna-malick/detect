use futures::future::BoxFuture;
use recursion::map_layer::MapLayer;
use std::io;

#[derive(Debug)]
pub enum Operator<Recurse> {
    Not(Recurse),
    And(Recurse, Recurse),
    Or(Recurse, Recurse),
}

impl<A, B> MapLayer<B> for Operator<A> {
    type Unwrapped = A;
    type To = Operator<B>;
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        use Operator::*;
        match self {
            Not(a) => Not(f(a)),
            And(a, b) => And(f(a), f(b)),
            Or(a, b) => Or(f(a), f(b)),
        }
    }
}

impl<'a> Operator<BoxFuture<'a, io::Result<bool>>> {
    pub(crate) async fn eval_async(self) -> io::Result<bool> {
        use Operator::*;
        match self {
            Not(a) => Ok(!a.await?),
            And(a, b) => {
                let a = a.await?;
                let b = b.await?;
                Ok(a && b)
            }
            Or(a, b) => {
                let a = a.await?;
                let b = b.await?;
                Ok(a || b)
            }
        }
    }
}

impl Operator<bool> {
    pub(crate) fn eval(self) -> bool {
        use Operator::*;
        match self {
            Not(x) => !x,
            And(x, y) => x && y,
            Or(x, y) => x || y,
        }
    }
}

impl<X> Operator<X> {
    pub(crate) fn as_ref_op(&self) -> Operator<&X> {
        use Operator::*;
        match self {
            Not(a) => Not(a),
            And(a, b) => And(a, b),
            Or(a, b) => Or(a, b),
        }
    }
}
