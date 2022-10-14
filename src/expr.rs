pub(crate) use crate::matcher::{ContentsMatcher, MetadataMatcher, NameMatcher};
pub(crate) use crate::operator::Operator;
use recursion::map_layer::MapLayer;
use recursion::map_layer::Project;

/// Filesystem entity matcher expression, with branches for matchers on
/// - file name
/// - file metadata
/// - file contents
#[derive(Debug)]
pub enum Expr<Name = NameMatcher, Metadata = MetadataMatcher, Contents = ContentsMatcher> {
    Operator(Operator<Box<Self>>),
    KnownResult(bool),
    Name(Name),
    Metadata(Metadata),
    Contents(Contents),
}


/// NOTE: a projection, not a component of Expr
/// Single layer of a filesystem entity matcher expression, with branches for matchers on
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

impl<Name, Meta, Content, A, B> MapLayer<B> for ExprLayer<A, Name, Meta, Content> {
    type Unwrapped = A;
    type To = ExprLayer<B, Name, Meta, Content>;
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

impl<'a, S1: 'a, S2: 'a, S3: 'a> Project for &'a Expr<S1, S2, S3> {
    type To = ExprLayer<Self, &'a S1, &'a S2, &'a S3>;

    fn project(self) -> Self::To {
        self.as_ref()
    }
}



// borrowed vs owned nature extremely unclear here
impl<N, M, C> Expr<N, M, C> {
    // coproject - TODO rename, MB: make public API?
    // TOD: in use the things in the exprlayer are all borrowed so it looks like parameterizing over owned vs borrowed
    pub(crate) fn new(e: ExprLayer<Self, N, M, C>) -> Self {
        use ExprLayer::*;
        match e {
            Operator(o) => Self::Operator(o.map_layer(Box::new)),
            KnownResult(k) => Self::KnownResult(k),
            Name(n) => Self::Name(n),
            Metadata(m) => Self::Metadata(m),
            Contents(c) => Self::Contents(c),
        }
    }

    pub(crate) fn known(&self) -> Option<bool> {
        match *self {
            Self::KnownResult(b) => Some(b),
            _ => None,
        }
    }

    // TODO: rename or move into project
    // this looks like everything being borrowed
    pub(crate) fn as_ref(&self) -> ExprLayer<&Self, &N, &M, &C> {
        use ExprLayer::*;
        match self {
            Self::Operator(o) => Operator(o.as_ref_op()),
            Self::KnownResult(b) => KnownResult(*b),
            Self::Name(n) => Name(n),
            Self::Metadata(m) => Metadata(m),
            Self::Contents(c) => Contents(c),
        }
    }
}
