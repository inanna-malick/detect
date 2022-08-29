#![feature(box_patterns)]

mod expr;
mod file;
mod operator;
mod parser;

use combine::{EasyParser, Parser};
use expr::MetadataPredicate;
use file::{FileLike, FileType, MetadataLike};
use futures::{channel::oneshot, executor::block_on, future::BoxFuture, FutureExt};
use recursion::{map_layer::MapLayer, stack_machine_lazy::unfold_and_fold};
use regex::RegexSet;
use std::{
    fs::{self},
    io,
};
use walkdir::DirEntry;

use crate::{
    expr::{ExprRef, ExprTree, RegexPredicate},
    operator::Operator,
};

use combine::stream::position;

impl MetadataLike for fs::Metadata {
    fn size(&self) -> u64 {
        self.len()
    }

    fn filetype(&self) -> FileType {
        let ft = self.file_type();
        if ft.is_dir() {
            FileType::Dir
        } else if ft.is_file() {
            FileType::File
        } else if ft.is_symlink() {
            FileType::Symlink
        } else {
            FileType::Unknown
        }
    }
}

impl FileLike for DirEntry {
    fn contents(&self) -> BoxFuture<io::Result<String>> {
        async { fs::read_to_string(self.path()) }.boxed()
    }
}

pub async fn parse_and_run(s:String) -> Result<(), anyhow::Error> {
    let e = parser::or().parse(position::Stream::new(&s[..]))?;
    run(&e.0).await // NOTE: having this be async is kinda fucky, rust file io is all sync AFAIK
}


pub async fn run(e: &ExprTree) -> Result<(), anyhow::Error> {
    use walkdir::WalkDir;

    fn traverse(e: &ExprTree, entry: &DirEntry) -> Result<bool, anyhow::Error> {
        let metadata = entry.metadata()?;

        let res = block_on(eval(entry, &metadata, e))?;
        Ok(res)
    }

    let walker = WalkDir::new(".").into_iter();
    for entry in walker.filter_entry(|entry| {
        // just default to skip if it fails (todo - could just panic, no other error reporting mechanism here tho)
        traverse(e, entry).unwrap_or(false)
    }) {
        println!("{}", entry?.path().display());
    }

    Ok(())
}

// runs a two-pass eval process, first attempting to find a pure answer and then
// running the async portion of the expr language against the file's contents (expensive, best avoided)
pub async fn eval<'a, File: FileLike, Metadata: MetadataLike>(
    f: &'a File,
    m: &Metadata,
    e: &'a ExprTree,
) -> io::Result<bool> {
    use eval_internal::*;

    // First pass: evaluate as much of the expression tree as we can without running async grep operations,
    // short circuiting on AND and OR and pruning async grep operations in short-circuited subtrees
    let intermediate: Fix<'a> =
        unfold_and_fold::<&'a ExprTree, Fix<'a>, ExprRef<'a, &'a ExprTree>, ExprRef<'a, Fix<'a>>>(
            e,
            |x| x.fs_ref.as_ref_expr(),
            |layer| {
                Fix(Box::new(match layer {
                    ExprRef::Operator(x) => match x {
                        // short circuit
                        Operator::And(xs) if xs.iter().any(|b| b.0.known() == Some(false)) => {
                            Intermediate::KnownResult(false)
                        }
                        Operator::Or(xs) if xs.iter().any(|b| b.0.known() == Some(true)) => {
                            Intermediate::KnownResult(true)
                        }
                        x => match x.known() {
                            None => Intermediate::Operator(x),
                            // if all sub-exprs have known results, so does this operator expr
                            Some(o) => Intermediate::KnownResult(o.eval()),
                        },
                    },
                    ExprRef::Predicate(p) => Intermediate::KnownResult(eval_predicate(m, p)),
                    ExprRef::RegexPredicate(x) => {
                        if m.filetype() == FileType::File {
                            Intermediate::RegexPredicate(x)
                        } else {
                            // not a text file, no possiblity of match
                            Intermediate::KnownResult(false)
                        }
                    }
                }))
            },
        );

    // short circuit if we have a known result at the root node
    if let Fix(box Intermediate::KnownResult(x)) = intermediate {
        return Ok(x);
    }

    // used to register a watcher against the single-pass regex run
    struct RegexWatcher<'a> {
        sender: oneshot::Sender<io::Result<bool>>,
        regex: RegexPredicate<'a>,
    }
    let mut regexes: Vec<RegexWatcher<'a>> = vec![];

    // build but do not run future - we need to run our regex set against the file first
    let eval_fut = unfold_and_fold::<_, BoxFuture<'a, io::Result<bool>>, _, _>(
        intermediate,
        |Fix(box x)| x,
        |collapse| match collapse {
            Intermediate::Operator(o) => o.eval_async().boxed(),
            Intermediate::KnownResult(b) => async move { Ok(b) }.boxed(),
            Intermediate::RegexPredicate(regex) => {
                let (sender, receive) = oneshot::channel();
                regexes.push(RegexWatcher { sender, regex });
                async move {
                    match receive.await {
                        Ok(msg) => msg,
                        Err(oneshot::Canceled) => {
                            unreachable!("FIXME - fold into std io Err result")
                        }
                    }
                }
                .boxed()
            }
        },
    );

    // if we have some regexes to run, run them (assertion: we should always have regexes if we make it here)
    // this is a bit complex, but it lets us run all the regexes relevant to the expression being evaluated in one pass
    if !regexes.is_empty() {
        let regex_set = RegexSet::new(regexes.iter().map(|x| x.regex.regex)).unwrap();

        // expensive async fetch of file contents
        let contents = f.contents().await?;

        // run regex set and collect match indexes
        let matching_idxs = regex_set.matches(&contents);
        // let each watcher know if it has a match or not
        for (idx, watcher) in regexes.into_iter().enumerate() {
            let is_match = matching_idxs.iter().any(|i| i == idx);
            watcher
                .sender
                .send(Ok(is_match))
                .expect("just assume this always succeeds for now");
        }
    }

    // now we can await - the oneshot channels have their results
    eval_fut.await
}

mod eval_internal {
    use super::*;

    // intermediate state
    pub(crate) enum Intermediate<'a, Recurse> {
        Operator(Operator<Recurse>),
        KnownResult(bool), // pure code leading to result via previous stage of processing
        RegexPredicate(RegexPredicate<'a>), // async predicate, not yet run
    }

    impl<'a, X> Intermediate<'a, X> {
        pub fn known(&self) -> Option<bool> {
            match self {
                Intermediate::KnownResult(x) => Some(*x),
                _ => None,
            }
        }
    }

    // from expr to expr
    impl<'a, A, B> MapLayer<B> for Intermediate<'a, A> {
        type Unwrapped = A;
        type To = Intermediate<'a, B>;
        fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To {
            use Intermediate::*;
            match self {
                Operator(o) => Operator(o.map_layer(f)),
                KnownResult(k) => KnownResult(k),
                RegexPredicate(a) => RegexPredicate(a),
            }
        }
    }

    pub(crate) struct Fix<'a>(pub(crate) Box<Intermediate<'a, Fix<'a>>>);

    impl<'a> Fix<'a> {
        fn known(&self) -> Option<bool> {
            match *self.0 {
                Intermediate::KnownResult(b) => Some(b),
                _ => None,
            }
        }
    }

    impl<'a> Operator<Fix<'a>> {
        pub(crate) fn known(&self) -> Option<Operator<bool>> {
            match self {
                Operator::Not(a) => a.known().map(Operator::Not),
                Operator::And(xs) => {
                    if let Some(all_known) = xs.iter().map(|x| x.known()).collect() {
                        Some(Operator::And(all_known))
                    } else {
                        None
                    }
                }
                Operator::Or(xs) => {
                    if let Some(all_known) = xs.iter().map(|x| x.known()).collect() {
                        Some(Operator::And(all_known))
                    } else {
                        None
                    }
                }
            }
        }
    }
}

fn eval_predicate<Metadata: MetadataLike>(m: &Metadata, p: MetadataPredicate) -> bool {
    match p {
        MetadataPredicate::Binary => m.filetype() == FileType::Binary,
        MetadataPredicate::Exec => m.filetype() == FileType::Exec,
        MetadataPredicate::Size { allowed } => allowed.contains(&m.size()),
        MetadataPredicate::Symlink => m.filetype() == FileType::Symlink,
    }
}
