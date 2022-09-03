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

// TODO:
// - filename search pattern - name(...)
// - extension searcher (shorthand) - ext(...)

impl MetadataLike for fs::Metadata {
    fn size(&self) -> u64 {
        self.len()
    }

    fn filetype(&self) -> FileType {
        let ft = self.file_type();
        let res = if ft.is_dir() {
            FileType::Dir
        } else if ft.is_file() {
            FileType::File
        } else if ft.is_symlink() {
            FileType::Symlink
        } else {
            FileType::Unknown
        };

        // println!("metadata filetype res: {:?}", res);

        res
    }
}

impl FileLike for DirEntry {
    fn contents(&self) -> BoxFuture<io::Result<String>> {
        async { fs::read_to_string(self.path()) }.boxed()
    }
}

pub async fn parse_and_run(s: String) -> Result<(), anyhow::Error> {
    let res = parser::or().parse(position::Stream::new(&s[..]));
    println!("parsed expr: {:?}", res);
    let e = res?;
    run(&e.0).await // NOTE: having this be async is kinda fucky, rust file io is all sync AFAIK
}

pub async fn run(e: &ExprTree) -> Result<(), anyhow::Error> {
    use walkdir::WalkDir;

    fn traverse(e: &ExprTree, entry: &DirEntry) -> Result<bool, anyhow::Error> {
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            // short circuit for directories - always continue here
            return Ok(true);
        }

        let res = block_on(eval(entry, &metadata, e))?;
        Ok(res)
    }

    let walker = WalkDir::new(".").into_iter();
    for entry in walker.filter_entry(|entry| {
        // just default to skip if it fails (todo - could just panic, no other error reporting mechanism here tho)
        traverse(e, entry).unwrap_or(false)
    }) {
        let entry = entry?;
        if !entry.metadata()?.is_dir() {
            println!("{}", entry.path().display());
        }
    }

    Ok(())
}

pub async fn eval<'a, File: FileLike, Metadata: MetadataLike>(
    f: &'a File,
    m: &Metadata,
    e: &'a ExprTree,
) -> io::Result<bool> {
    use eval_internal::*;

    // println!("eval {:?}", e);

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
                    ExprRef::MetadataPredicate(p) => Intermediate::KnownResult(eval_predicate(m, p)),
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
        println!("short circuit");
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



// runs a two-pass eval process, first attempting to find a pure answer and then
// running the async portion of the expr language against the file's contents (expensive, best avoided)
pub fn eval_ex(
    e: &ExprTree,
) -> io::Result<bool> {
    use eval_experimental::*;


    type FileName = ();
    type Metadata = ();
    type Contents = ();

    // First pass: convert to intermediate expr type, no short circuiting or anything yet
    let stage1: Fix<FileName, Metadata, Contents> =
        unfold_and_fold(
            e,
            |x| x.fs_ref.as_ref_expr(),
            |layer| {
                Fix(Box::new(match layer {
                    ExprRef::Operator(o) => Intermediate::Operator(o),
                    ExprRef::MetadataPredicate(_) => todo!(), // metadata, first pass
                    ExprRef::RegexPredicate(_) => todo!(), // regex (file content read) pass
                }))
            },
        );


    let stage2: Fix<Done, Metadata, Contents> = run_stage()
        unfold_and_fold(
            e,
            |x| x.fs_ref.as_ref_expr(),
            |layer| {
                Fix(Box::new(match layer {
                    ExprRef::Operator(o) => Intermediate::Operator(o),
                    ExprRef::MetadataPredicate(_) => todo!(), // metadata, first pass
                    ExprRef::RegexPredicate(_) => todo!(), // regex (file content read) pass
                }))
            },
        );


        todo!("")
}

mod eval_experimental {
    use super::*;

    pub(crate) enum Done {}

    pub(crate) fn never<A, B>(a: A) -> B {
        unreachable!("never")
    }


    pub(crate) fn identity<A>(a: A) -> A {
        a
    }

    // NOTE! metadata predicates are actually a second pass stage in their own right b/c getting them is a syscall (I think, lol)
    // NOTE(cont)! run name predicate first, the rest are packed into stages
    // NOTE(cont)! I think last stage is probably just arbitrary process execution
    pub(crate) enum Intermediate<Recurse, Stage1, Stage2, Stage3> {
        Operator(Operator<Recurse>),
        KnownResult(bool), // pure code leading to result via previous stage of processing
        Stage1(Stage1),    // async predicate, not yet run
        Stage2(Stage2),    // async predicate, not yet run
        Stage3(Stage3),    // async predicate, not yet run
    }

    // NOTE: we are not running this async by default because all this filesystem stuff is really just synchronous
    // NOTE: even the async stuff tends to run against, like, one thing and can thus be done _in between_ run_stage calls
    // NOTE: like, even regexset, watchers can be replaced with indexes. also I think I can drop the abstraction boundary and just
    // NOTE: write this over, specifically, unix files
    pub(crate) fn run_stage<S1A, S2A, S3A, S1B, S2B, S3B, F1, F2, F3>(
        e: Fix<S1A, S2A, S3A>,
        f1: F1,
        f2: F2,
        f3: F3,
    ) -> Fix<S1B, S2B, S3B>
    where
        F1: Fn(S1A) -> S1B,
        F2: Fn(S2A) -> S2B,
        F3: Fn(S3A) -> S3B,
    {
        unfold_and_fold(
            e,
            |Fix(box x)| x,
            |layer| {
                Fix(Box::new(match layer {
                    Intermediate::Operator(x) => match x {
                        // short circuit
                        Operator::And(xs)
                            if xs.iter().any(|b: &Fix<_, _, _>| b.known() == Some(false)) =>
                        {
                            Intermediate::KnownResult(false)
                        }
                        Operator::Or(xs)
                            if xs.iter().any(|b: &Fix<_, _, _>| b.known() == Some(true)) =>
                        {
                            Intermediate::KnownResult(true)
                        }
                        x => match x.known() {
                            None => Intermediate::Operator(x),
                            // if all sub-exprs have known results, so does this operator expr
                            Some(o) => Intermediate::KnownResult(o.eval()),
                        },
                    },
                    Intermediate::KnownResult(x) => todo!(),
                    Intermediate::Stage1(s1) => Intermediate::Stage1(f1(s1)),
                    Intermediate::Stage2(s2) => Intermediate::Stage2(f2(s2)),
                    Intermediate::Stage3(s3) => Intermediate::Stage3(f3(s3)),
                }))
            },
        )
    }

    impl<Stage1, Stage2, Stage3, A, B> MapLayer<B> for Intermediate<A, Stage1, Stage2, Stage3> {
        type Unwrapped = A;
        type To = Intermediate<B, Stage1, Stage2, Stage3>;
        fn map_layer<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To {
            use Intermediate::*;
            match self {
                Operator(o) => Operator(o.map_layer(f)),
                KnownResult(k) => KnownResult(k),
                Stage1(x) => Stage1(x),
                Stage2(x) => Stage2(x),
                Stage3(x) => Stage3(x),
            }
        }
    }

    pub(crate) struct Fix<S1, S2, S3>(pub(crate) Box<Intermediate<Fix<S1, S2, S3>, S1, S2, S3>>);

    impl<S1, S2, S3> Fix<S1, S2, S3> {
        fn known(&self) -> Option<bool> {
            match *self.0 {
                Intermediate::KnownResult(b) => Some(b),
                _ => None,
            }
        }
    }


    impl<S1, S2, S3> Operator<Fix<S1, S2, S3>> {
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
    let res = match &p {
        MetadataPredicate::Binary => m.filetype() == FileType::Binary,
        MetadataPredicate::Exec => m.filetype() == FileType::Exec,
        MetadataPredicate::Size { allowed } => allowed.contains(&m.size()),
        MetadataPredicate::Symlink => m.filetype() == FileType::Symlink,
    };

    println!("eval predicate {:?} yielding {:?}", p, res);

    res
}
