#![feature(box_patterns)]

use std::ops::Range;

use futures::{
    channel::oneshot::{self, Canceled},
    future::BoxFuture,
    FutureExt,
};
use recursion::{
    map_layer::{MapLayer, MapLayerRef},
    stack_machine_lazy::{unfold_and_fold, unfold_and_fold_short_circuit, ShortCircuit},
};
use regex::{Regex, RegexSet, RegexSetBuilder};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

// two passes, better types!
pub async fn eval_3<'a, File: FileLike>(f: &File, e: FileSetRef<'a>) -> std::io::Result<bool> {
    // intermediate state
    enum Intermediate<'a, Recurse> {
        Operator(Operator<Recurse>),
        KnownResult(bool), // pure code leading to result via previous stage of processing
        AsyncRegex(AsyncRegex<'a>), // async predicate, not yet run
    }

    // from expr to expr
    impl<'a, A, B> MapLayer<B> for Intermediate<'a, A> {
        type Unwrapped = A;
        type To = Intermediate<'a, B>;
        fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To {
            match self {
                Intermediate::Operator(o) => Intermediate::Operator(o.map_layer(f)),
                Intermediate::KnownResult(k) => Intermediate::KnownResult(k),
                Intermediate::AsyncRegex(a) => Intermediate::AsyncRegex(a),
            }
        }
    }

    // fixed point hack
    struct F<'a>(Box<Intermediate<'a, F<'a>>>);

    // run all the pure expr calcs we can here (TODO - use arena for optimzation; figure out more elegant repr)
    let x: F<'a> = unfold_and_fold::<
        FileSetRef<'a>,
        F<'a>,
        FilesetExprRef<'a, FileSetRef<'a>>,
        FilesetExprRef<'a, F<'a>>,
    >(
        e,
        |x| x.map_layer(|x| x),
        |layer| {
            F(Box::new(match layer {
                FilesetExprRef::Operator(x) => match x {
                    Operator::Not(F(box Intermediate::KnownResult(x))) => {
                        Intermediate::KnownResult(!x)
                    }
                    Operator::And(F(box Intermediate::KnownResult(false)), _)
                    | Operator::And(_, F(box Intermediate::KnownResult(false))) => {
                        Intermediate::KnownResult(false)
                    }
                    Operator::Or(F(box Intermediate::KnownResult(true)), _)
                    | Operator::Or(_, F(box Intermediate::KnownResult(true))) => {
                        Intermediate::KnownResult(true)
                    }
                    x => Intermediate::Operator(x),
                },
                FilesetExprRef::Predicate(p) => Intermediate::KnownResult(eval_predicate(f, p)),
                FilesetExprRef::AsyncPredicate(x) => Intermediate::AsyncRegex(x),
            }))
        },
    );

    // TODO: mutable - accepts set of regexes + listener, then runs async
    struct RegexWatcher<'a>(oneshot::Sender<std::io::Result<bool>>, AsyncRegex<'a>);
    // TODO: thing that grabs all the regexes and runs once so I don't need to etc
    let mut regexes: Vec<RegexWatcher<'a>> = vec![];

    // build but do not run future - we need to run our regex set against the file first
    let f = unfold_and_fold::<_, BoxFuture<'a, std::io::Result<bool>>, _, _>(
        x,
        |fseri| *fseri.0,
        |collapse| match collapse {
            Intermediate::Operator(o) => eval_operator_async(o).boxed(),
            Intermediate::KnownResult(b) => async move { Ok(b) }.boxed(),
            Intermediate::AsyncRegex(r) => {
                let (send, receive) = oneshot::channel();
                regexes.push(RegexWatcher(send, r));
                async move {
                    match receive.await {
                        Ok(msg) => msg,
                        Err(oneshot::Canceled) => todo!(),
                    }
                }
                .boxed()
            }
        },
    );

    // TODO: handle? precompile? can't really precompile b/c the set of regexes changes based on the run
    let regex_set = RegexSet::new(regexes.iter().map(|x| (x.1).regex)).unwrap();
    let contents: String = "".to_string(); // run streaming or w/e
    let matches: Vec<_> = regex_set.matches(&contents).into_iter().collect();
    for (idx, watcher) in regexes.into_iter().enumerate() {
        if matches.contains(&idx) {
            watcher.0.send(Ok(true)).unwrap();
        } else {
            watcher.0.send(Ok(false)).unwrap();
        }
    }

    f.await
}

// pause - should I be running this with result of a set of files or of a bool for a single file
// answer - yes, runs against single file (although could be optimized)
pub fn eval<'a, File: FileLike>(f: &File, e: FileSetRef<'a>) -> bool {
    unfold_and_fold_short_circuit::<
        FileSetRef<'a>,
        bool,
        FilesetExprRef<'a, (FileSetRef<'a>, Option<ShortCircuit<bool>>)>,
        FilesetExprRef<'a, bool>,
    >(e, expand_layer, |layer| eval_layer(f, layer))
}

fn expand_layer<'a>(
    e: FileSetRef<'a>,
) -> FilesetExprRef<'a, (FileSetRef<'a>, Option<ShortCircuit<bool>>)> {
    let short_circut = match &e.fs_ref {
        FilesetExpr::Operator(o) => match o {
            Operator::Not(_) => None,
            Operator::And(_, _) => Some(ShortCircuit {
                short_circuit_on: false,
                return_on_short_circuit: false,
            }),
            Operator::Or(_, _) => Some(ShortCircuit {
                short_circuit_on: true,
                return_on_short_circuit: true,
            }),
        },
        FilesetExpr::Predicate(_) => None,
    };

    e.map_layer(|l| {
        // TODO: remove clone after the rest is working
        (l, short_circut)
    })
}

fn eval_layer<File: FileLike>(f: &File, e: FilesetExprRef<bool>) -> bool {
    match e {
        FilesetExprRef::Operator(o) => eval_operator(o),
        FilesetExprRef::Predicate(p) => eval_predicate(f, p),
        FilesetExprRef::AsyncPredicate(_) => todo!(),
    }
}

async fn eval_operator_async<'a>(
    o: Operator<BoxFuture<'a, std::io::Result<bool>>>,
) -> std::io::Result<bool> {
    match o {
        Operator::Not(a) => Ok(!a.await?),
        Operator::And(a, b) => todo!(),
        Operator::Or(a, b) => todo!(),
    }
}

fn eval_operator(o: Operator<bool>) -> bool {
    match o {
        Operator::Not(x) => !x,
        Operator::And(x, y) => x && y,
        Operator::Or(x, y) => todo!(),
    }
}

async fn eval_async_predicate<'a, File: FileLike>(
    f: &File,
    p: AsyncRegex<'a>,
) -> std::io::Result<bool> {
    todo!()
}

fn eval_predicate<File: FileLike>(f: &File, p: MetadataPredicate) -> bool {
    match p {
        MetadataPredicate::Binary => todo!(),
        MetadataPredicate::Exec => todo!(),
        MetadataPredicate::Size { allowed: size } => todo!(),
        MetadataPredicate::Symlink => todo!(),
    }
}

#[derive(Copy, Clone)]
pub struct FileSetRef<'a> {
    fs_ref: &'a FilesetExpr<FileSetRef<'a>>,
}

// from https://backend.bolt80.com/hgdoc/topic-filesets.html
pub enum FilesetExpr<Recurse> {
    Operator(Operator<Recurse>),
    Predicate(MetadataPredicate),
}

pub enum FilesetExprRef<'a, Recurse> {
    Operator(Operator<Recurse>),
    Predicate(MetadataPredicate),
    AsyncPredicate(AsyncRegex<'a>),
}

// TODO: regex and etc
pub struct AsyncRegex<'a> {
    regex: &'a str,
}

// from expr to expr
impl<'a, A, B> MapLayer<B> for FilesetExprRef<'a, A> {
    type Unwrapped = A;
    type To = FilesetExprRef<'a, B>;
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To {
        todo!()
    }
}

impl<'a, A, B> MapLayerRef<B> for FilesetExprRef<'a, A> {
    type Unwrapped = A;
    type To = FilesetExprRef<'a, B>;
    fn map_layer_ref<F: FnMut(Self::Unwrapped) -> B>(&self, f: F) -> Self::To {
        todo!()
    }
}

// from specific 'fixed point' recursive-pointer form
impl<'a, B: 'a> MapLayer<B> for FileSetRef<'a> {
    type Unwrapped = FileSetRef<'a>;
    type To = FilesetExprRef<'a, B>;
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To {
        todo!()
    }
}

impl<A, B> MapLayer<B> for Operator<A> {
    type Unwrapped = A;
    type To = Operator<B>;
    fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        match self {
            Operator::Not(a) => Operator::Not(f(a)),
            Operator::And(a, b) => Operator::And(f(a), f(b)),
            Operator::Or(a, b) => Operator::Or(f(a), f(b)),
        }
    }
}

// not x
//     Files not in x. Short form is ! x.
// x and y
//     The intersection of files in x and y. Short form is x & y.
// x or y
//     The union of files in x and y. There are two alternative short forms: x | y and x + y.
#[derive(Debug)]
pub enum Operator<Recurse> {
    Not(Recurse),
    And(Recurse, Recurse),
    Or(Recurse, Recurse),
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
#[derive(Debug)]
pub enum MetadataPredicate {
    Binary,
    Exec,
    Size { allowed: Range<usize> },
    Symlink,
}

pub enum FileType {
    Binary,
    Exec,
    Symlink,
    Text,
}

pub trait FileLike {
    fn size(&self) -> usize;
    fn filetype(&self) -> FileType;
    fn contains(&self, regex: &str) -> bool;
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
