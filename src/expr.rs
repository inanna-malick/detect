pub(crate) use crate::matcher::{ContentsMatcher, MetadataMatcher, NameMatcher};
pub(crate) use crate::operator::Operator;
use recursion::{map_layer::MapLayer, stack_machine_lazy::unfold_and_fold};

/// Generic expression type, with branches for matchers on
/// - file name
/// - file metadata
/// - file contents
#[derive(Debug)]
pub(crate) enum Expr<
    Recurse,
    Name = NameMatcher,
    Metadata = MetadataMatcher,
    Contents = ContentsMatcher,
> {
    Operator(Operator<Recurse>),
    KnownResult(bool),
    NameMatcher(Name),
    MetadataMatcher(Metadata),
    ContentsMatcher(Contents),
}

/// abstracts over building and running a stack machine to
/// process some stage of expression evaluation with short-circuiting
/// where applicable - eg, And(false, *, *), Or(*, *, true, *)
pub(crate) fn run_stage<'a, NameA, MetadataA, ContentsA, NameB, MetadataB, ContentsB, F1, F2, F3>(
    e: &'a ExprTree<NameA, MetadataA, ContentsA>,
    mut map_name: F1,
    mut map_metadata: F2,
    mut map_contents: F3,
) -> ExprTree<NameB, MetadataB, ContentsB>
where
    F1: FnMut(&'a NameA) -> ExprLayer<NameB, MetadataB, ContentsB>,
    F2: FnMut(&'a MetadataA) -> ExprLayer<NameB, MetadataB, ContentsB>,
    F3: FnMut(&'a ContentsA) -> ExprLayer<NameB, MetadataB, ContentsB>,
{
    unfold_and_fold(e, ExprTree::as_ref, |layer| {
        ExprTree(Box::new(match layer {
            Expr::Operator(x) => match x.eval() {
                None => Expr::Operator(x),
                Some(k) => Expr::KnownResult(k),
            },
            Expr::KnownResult(x) => Expr::KnownResult(x),
            Expr::NameMatcher(s1) => map_name(s1),
            Expr::MetadataMatcher(s2) => map_metadata(s2),
            Expr::ContentsMatcher(s3) => map_contents(s3),
        }))
    })
}

impl<Stage1, Stage2, Stage3, A, B> MapLayer<B> for Expr<A, Stage1, Stage2, Stage3> {
    type Unwrapped = A;
    type To = Expr<B, Stage1, Stage2, Stage3>;
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To {
        use Expr::*;
        match self {
            Operator(o) => Operator(o.map_layer(f)),
            KnownResult(k) => KnownResult(k),
            NameMatcher(x) => NameMatcher(x),
            MetadataMatcher(x) => MetadataMatcher(x),
            ContentsMatcher(x) => ContentsMatcher(x),
        }
    }
}

type ExprLayer<A, B, C> = Expr<ExprTree<A, B, C>, A, B, C>;
#[derive(Debug)]
pub struct ExprTree<Name = NameMatcher, Metadata = MetadataMatcher, Contents = ContentsMatcher>(
    pub(crate) Box<ExprLayer<Name, Metadata, Contents>>,
);

impl<S1, S2, S3> ExprTree<S1, S2, S3> {
    pub(crate) fn new(e: ExprLayer<S1, S2, S3>) -> Self {
        Self(Box::new(e))
    }

    pub(crate) fn known(&self) -> Option<bool> {
        match *self.0 {
            Expr::KnownResult(b) => Some(b),
            _ => None,
        }
    }

    pub(crate) fn as_ref(&self) -> Expr<&Self, &S1, &S2, &S3> {
        match self.0.as_ref() {
            Expr::Operator(o) => Expr::Operator(o.as_ref_op()),
            Expr::KnownResult(b) => Expr::KnownResult(*b),
            Expr::NameMatcher(n) => Expr::NameMatcher(n),
            Expr::MetadataMatcher(m) => Expr::MetadataMatcher(m),
            Expr::ContentsMatcher(c) => Expr::ContentsMatcher(c),
        }
    }
}
