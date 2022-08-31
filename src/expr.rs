use recursion::map_layer::MapLayer;
use std::ops::Range;

use crate::operator::Operator;

#[derive(Debug, Eq, PartialEq)]
pub struct ExprTree {
    pub fs_ref: Box<Expr<ExprTree>>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Expr<Recurse> {
    Operator(Operator<Recurse>),
    Predicate(MetadataPredicate),
    RegexPredicate { regex: String },
}

impl<X> Expr<X> {
    pub(crate) fn as_ref_expr<'a>(&'a self) -> ExprRef<'a, &'a X> {
        match self {
            Expr::Operator(o) => ExprRef::Operator(o.as_ref_op()),
            // TODO: clone is fine but could be removed mb (also, why is Range not Copy (???) - literally 2x usize!)
            Expr::Predicate(mp) => ExprRef::Predicate(mp.clone()),
            Expr::RegexPredicate { regex } => ExprRef::RegexPredicate(RegexPredicate { regex }),
        }
    }
}

pub enum ExprRef<'a, Recurse> {
    Operator(Operator<Recurse>),
    Predicate(MetadataPredicate),
    RegexPredicate(RegexPredicate<'a>),
}

// TODO: check validity at construction time,
// &str should be known valid at parse time even if not compiled to Regex
pub struct RegexPredicate<'a> {
    pub(crate) regex: &'a str,
}

// from expr ref to expr ref
impl<'a, A, B> MapLayer<B> for ExprRef<'a, A> {
    type Unwrapped = A;
    type To = ExprRef<'a, B>;
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To {
        use ExprRef::*;
        match self {
            Operator(o) => Operator(o.map_layer(f)),
            Predicate(p) => Predicate(p),
            RegexPredicate(a) => RegexPredicate(a),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MetadataPredicate {
    Binary,
    Exec,
    Symlink,
    Size { allowed: Range<u64> },
}
