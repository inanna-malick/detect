use recursion::{
    map_layer::MapLayer,
    stack_machine_lazy::{unfold_and_fold_short_circuit, ShortCircuit},
};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

fn eval<'a>(e: FSExprRefBacked<'a>) -> bool {
    unfold_and_fold_short_circuit::<FSExprRefBacked<'a>, bool, _, _>(
        e,
        expand_layer,
        eval_layer,
    )
}

fn expand_layer<'a>(
    e: FSExprRefBacked<'a>,
) -> FilesetExpr<(FSExprRefBacked<'a>, Option<ShortCircuit<bool>>)> {
    // e.fs_ref.map_layer(f)
    todo!()
}

fn eval_layer(e: FilesetExpr<bool>) -> bool {
    match e {
        FilesetExpr::Operator(o) => eval_operator(o),
        FilesetExpr::Predicate(p) => eval_predicate(p),
    }
}

fn eval_operator(o: Operator<bool>) -> bool {
    match o {
        Operator::Not(x) => !x,
        Operator::And(x, y) => todo!(),
        Operator::Or(_, _) => todo!(),
        Operator::Sub(_, _) => todo!(),
    }
}

fn eval_predicate(p: MetadataPredicate) -> bool {
    match p {
        MetadataPredicate::Binary => todo!(),
        MetadataPredicate::Exec => todo!(),
        MetadataPredicate::Grep { regex } => todo!(),
        MetadataPredicate::Size { size } => todo!(),
        MetadataPredicate::Symlink => todo!(),
    }
}

type FSExprRefBacked<'a> = FilesetExpr<FileSetRef<'a>>;

pub struct FileSetRef<'a> {
    fs_ref: &'a FSExprRefBacked<'a>,
}

// from https://backend.bolt80.com/hgdoc/topic-filesets.html
pub enum FilesetExpr<Recurse> {
    Operator(Operator<Recurse>),
    Predicate(MetadataPredicate),
}

// first pass - just pretend everything has Clone
impl<A, B> MapLayer<B> for FilesetExpr<A> {
    type Unwrapped = A;
    type To = FilesetExpr<B>;
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, _f: F) -> Self::To {
        match self {
            FilesetExpr::Operator(_) => todo!(),
            FilesetExpr::Predicate(p) => FilesetExpr::Predicate(p),
        }
    }
}

// not x
//     Files not in x. Short form is ! x.
// x and y
//     The intersection of files in x and y. Short form is x & y.
// x or y
//     The union of files in x and y. There are two alternative short forms: x | y and x + y.
// x - y
//     Files in x but not in y.
pub enum Operator<Recurse> {
    Not(Recurse),
    And(Recurse, Recurse),
    Or(Recurse, Recurse),
    Sub(Recurse, Recurse),
}

// (PARTIAL SUBSET, removed 'hg' commands and a few other things too, for first pass)
// binary()
//     File that appears to be binary (contains NUL bytes).
// exec()
//     File that is marked as executable.
// grep(regex)
//     File contains the given regular expression.
// size(expression)
//     File size matches the given expression. Examples:
//         size('1k') - files from 1024 to 2047 bytes
//         size('< 20k') - files less than 20480 bytes
//         size('>= .5MB') - files at least 524288 bytes
//         size('4k - 1MB') - files from 4096 bytes to 1048576 bytes
// symlink()
//     File that is marked as a symlink.
pub enum MetadataPredicate {
    Binary,
    Exec,
    Grep { regex: String }, // TODO: parsed regex type
    Size { size: usize },   // TODO: range syntax (I think rust builtins will work, nice!)
    Symlink,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
