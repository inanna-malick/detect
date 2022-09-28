use crate::expr::{run_stage, ContentsMatcher, Expr, ExprTree, MetadataMatcher, NameMatcher};
use crate::util::{never, Done};
use bumpalo::Bump;
use regex::RegexSet;
use std::{
    fs::{self},
    os::unix::prelude::MetadataExt,
};
use walkdir::DirEntry;

// multipass evaluation with short circuiting, runs, in order:
// - file name matchers
// - metadata matchers
// - file content matchers
pub(crate) fn eval(e: &ExprTree, dir_entry: &DirEntry) -> std::io::Result<bool> {
    let scratchpad_arena = Bump::new();
    let e: ExprTree<Done, &MetadataMatcher, &ContentsMatcher> = run_stage(
        &scratchpad_arena,
        format!("{}_match_name", dir_entry.file_name().to_str().unwrap()),
        e,
        |name_matcher| match dir_entry.file_name().to_str() {
            Some(s) => match name_matcher {
                NameMatcher::Filename(r) => Expr::KnownResult(r.is_match(s)),
            },
            None => Expr::KnownResult(false),
        },
        Expr::MetadataMatcher,
        Expr::ContentsMatcher,
    );

    if let Some(b) = e.known() {
        return Ok(b);
    }

    // short circuit or query metadata (expensive)
    let metadata = dir_entry.metadata()?;

    let e: ExprTree<Done, Done, &ContentsMatcher> = run_stage(
        &scratchpad_arena,
        format!("{}_match_metadata", dir_entry.file_name().to_str().unwrap()),
        &e,
        never,
        |metadata_matcher| match metadata_matcher {
            MetadataMatcher::Filesize(range) => Expr::KnownResult(range.contains(&metadata.size())),
        },
        |c| Expr::ContentsMatcher(*c),
    );

    if let Some(b) = e.known() {
        return Ok(b);
    }

    #[derive(Debug, PartialEq, Eq, Clone)]
    enum ContentMatcherInternal {
        RegexIndex(usize),
        IsUtf8,
    }
    let mut regexes = Vec::new();

    // harvest regexes, then read and run
    let e: ExprTree<Done, Done, ContentMatcherInternal> = run_stage(
        &scratchpad_arena,
        format!(
            "{}_harvest_regexes",
            dir_entry.file_name().to_str().unwrap()
        ),
        &e,
        never,
        never,
        |contents_matcher| {
            Expr::ContentsMatcher(match contents_matcher {
                ContentsMatcher::FileContents(r) => {
                    regexes.push(r.as_str());
                    ContentMatcherInternal::RegexIndex(regexes.len() - 1)
                }
                ContentsMatcher::IsUtf8 => ContentMatcherInternal::IsUtf8,
            })
        },
    );

    let mut matching_idxs = Vec::new();
    let mut is_utf8 = false;

    let regex_set = RegexSet::new(regexes.iter()).unwrap();

    let contents = fs::read(dir_entry.path())?;

    if let Ok(contents) = String::from_utf8(contents) {
        matching_idxs.extend(regex_set.matches(&contents).into_iter());
        is_utf8 = true;
    }

    let e: ExprTree<Done, Done, Done> = run_stage(
        &scratchpad_arena,
        format!("{}_check_regexes", dir_entry.file_name().to_str().unwrap()),
        &e,
        never,
        never,
        |c| {
            Expr::KnownResult(match c {
                ContentMatcherInternal::RegexIndex(regex_idx) => matching_idxs.contains(regex_idx),
                ContentMatcherInternal::IsUtf8 => is_utf8,
            })
        },
    );

    Ok(e.known()
        .expect("all predicates evaluated, should have known result"))
}
