#![feature(box_patterns)]

use std::{
    convert::Infallible,
    future::{self, Future},
    marker::PhantomData,
    ops::Range,
};

use recursion::{
    map_layer::{MapLayer, MapLayerRef},
    stack_machine_lazy::{
        unfold_and_fold, unfold_and_fold_annotate_result, unfold_and_fold_short_circuit,
        ShortCircuit,
    },
};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

// idea: can I just grab the fast traversal engine out of fd or similar? would be nice af

// idea: do annotate pass and annotate with pure res off of metadata, then build async tree
// pub fn eval_2<'a, File: FileLike>(
//     f: &File,
//     e: FileSetRef<'a>,
// ) -> Box<dyn Future<Output = std::io::Result<bool>>> {
//     unfold_and_fold_annotate_result::<
//         Infallible,                                      // pure eval pass is infalible
//         FileSetRef<'a>,                                  // input expr that is folded over
//         Box<dyn Future<Output = std::io::Result<bool>>>, // async eval tree (for content search)
//         Option<bool>,                                    // annotation - in-memory-only eval pass
//         FilesetExprRef<'a, FileSetRef<'a>>,
//         FilesetExprRef<'a, Box<dyn Future<Output = std::io::Result<bool>>>>,
//         FilesetExprRef<'a, Option<bool>>,
//     >(
//         e,
//         |x| Ok(x.map_layer(|x| x)),
//         |annotated| {
//             use crate::Operator::*;
//             use FilesetExprRef::*;
//             Ok(match annotated {
//                 Operator(o) => match o {
//                     Not(Some(x)) => Some(!x),
//                     And(Some(false), _) | And(_, Some(false)) => Some(false),
//                     Or(Some(true), _) | Or(_, Some(true)) => Some(true),
//                     _ => None,
//                 },
//                 FilesetExprRef::Predicate(_) => todo!(), // eval where possible
//             })
//         },
//         |annotation, x| match annotation {
//             Some(res) => Ok(Box::new(future::ready(Ok(res)))), // anything already finished just gets an 'ok' future
//             None => {
//                 // NOTE: can simplify my life drastically with one unreachable!, specifically _here we only eval async predicates_
//                 // if we're in this branch with a pure metadata query predicate it's a KNOWN ERROR STATE
//                 // ALSO: this is all re: the same file, can just fuze the regexes (galaxy brain vibes holy shit)
//                 todo!()
//             }
//         },
//     )
//     .unwrap()
// }

// two passes, better types!
pub fn eval_3<'a, File: FileLike>(
    f: &File,
    e: FileSetRef<'a>,
) -> Box<dyn Future<Output = std::io::Result<bool>>> {
    // intermediate state
    enum Intermediate<'a, Recurse> {
        Operator(Operator<Recurse>),
        KnownResult(bool), // pure code leading to result via previous stage of processing
        AsyncPredicate(AsyncPredicate<'a>), // async predicate, not yet run
    }

    // from expr to expr
    impl<'a, A, B> MapLayer<B> for Intermediate<'a, A> {
        type Unwrapped = A;
        type To = Intermediate<'a, B>;
        fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To {
            match self {
                Intermediate::Operator(o) => Intermediate::Operator(o.map_layer(f)),
                Intermediate::KnownResult(k) => Intermediate::KnownResult(k),
                Intermediate::AsyncPredicate(a) => Intermediate::AsyncPredicate(a),
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
                FilesetExprRef::Predicate(p) => {
                    Intermediate::KnownResult(eval_predicate(f, p))
                }
                FilesetExprRef::AsyncPredicate(x) => Intermediate::AsyncPredicate(x),
            }))
        },
    );

    unfold_and_fold::<_, Box<dyn Future<Output = std::io::Result<bool>>>, _, _>(
        x,
        |fseri| *fseri.0,
        |collapse| match collapse {
            Intermediate::Operator(o) => eval_operator_async(o),
            Intermediate::KnownResult(b) => Box::new(async move { Ok(b) }),
            Intermediate::AsyncPredicate(p) => eval_async_predicate(f, p),
        },
    )
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

fn eval_operator_async(o: Operator<Box<dyn Future<Output = std::io::Result<bool>>>>) -> Box<dyn Future<Output = std::io::Result<bool>>> {
    Box::new(async { todo!()} )
}




fn eval_operator(o: Operator<bool>) -> bool {
    match o {
        Operator::Not(x) => !x,
        Operator::And(x, y) => x && y,
        Operator::Or(x, y) => todo!(),
    }
}


fn eval_async_predicate<File: FileLike>(f: &File, p: AsyncPredicate) -> Box<dyn Future<Output = std::io::Result<bool>>> {
    todo!()
}

fn eval_predicate<File: FileLike>(f: &File, p: MetadataPredicateRef) -> bool {
    match p {
        MetadataPredicateRef::Binary => todo!(),
        MetadataPredicateRef::Exec => todo!(),
        MetadataPredicateRef::Grep { regex } => todo!(),
        MetadataPredicateRef::Size { allowed: size } => todo!(),
        MetadataPredicateRef::Symlink => todo!(),
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
    Predicate(MetadataPredicateRef<'a>),
    AsyncPredicate(AsyncPredicate<'a>),
}



// TODO: regex and etc
pub struct AsyncPredicate<'a> {
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
    Grep { regex: String }, // TODO: parsed regex type
    Size { allowed: Range<usize> },
    Symlink,
}

pub enum MetadataPredicateRef<'a> {
    Binary,
    Exec,
    Grep { regex: &'a str }, // TODO: parsed regex type
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
