pub(crate) use crate::matcher::{ContentsMatcher, MetadataMatcher, NameMatcher};
pub(crate) use crate::operator::Operator;
use recursion::map_layer::MapLayer;
use recursion::map_layer::Project;

/// Generic expression type, with branches for matchers on
/// - file name
/// - file metadata
/// - file contents
#[derive(Debug)]
pub enum ExprLayer<
    Recurse,
    Name = NameMatcher,
    Metadata = MetadataMatcher,
    Contents = ContentsMatcher,
> {
    Operator(Operator<Recurse>),
    KnownResult(bool),
    Name(Name),
    Metadata(Metadata),
    Contents(Contents),
}

impl<Stage1, Stage2, Stage3, A, B> MapLayer<B> for ExprLayer<A, Stage1, Stage2, Stage3> {
    type Unwrapped = A;
    type To = ExprLayer<B, Stage1, Stage2, Stage3>;
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To {
        use ExprLayer::*;
        match self {
            Operator(o) => Operator(o.map_layer(f)),
            KnownResult(k) => KnownResult(k),
            Name(x) => Name(x),
            Metadata(x) => Metadata(x),
            Contents(x) => Contents(x),
        }
    }
}

#[derive(Debug)]
#[allow(clippy::type_complexity)] pub struct ExprTree<N = NameMatcher, M = MetadataMatcher, C = ContentsMatcher>(
    pub(crate) Box<ExprLayer<ExprTree<N, M, C>, N, M, C>>,
);

impl<'a, S1: 'a, S2: 'a, S3: 'a> Project for &'a ExprTree<S1, S2, S3> {
    type To = ExprLayer<Self, &'a S1, &'a S2, &'a S3>;

    fn project(self) -> Self::To {
        self.as_ref()
    }
}

impl<N, M, C> ExprTree<N, M, C> {
    pub(crate) fn new(e: ExprLayer<ExprTree<N, M, C>, N, M, C>) -> Self {
        Self(Box::new(e))
    }

    pub(crate) fn known(&self) -> Option<bool> {
        match *self.0 {
            ExprLayer::KnownResult(b) => Some(b),
            _ => None,
        }
    }

    pub(crate) fn as_ref(&self) -> ExprLayer<&Self, &N, &M, &C> {
        use ExprLayer::*;
        match self.0.as_ref() {
            Operator(o) => Operator(o.as_ref_op()),
            KnownResult(b) => KnownResult(*b),
            Name(n) => Name(n),
            Metadata(m) => Metadata(m),
            Contents(c) => Contents(c),
        }
    }
}
