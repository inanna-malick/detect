use crate::expr::{run_stage, ContentsMatcher, Expr, ExprTree, MetadataMatcher, NameMatcher};
use crate::util::{never, Done};
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
pub(crate) fn eval(e: &ExprTree, dir_entry: DirEntry) -> std::io::Result<bool> {
    let e: ExprTree<Done, &MetadataMatcher, &ContentsMatcher> = run_stage(
        e,
        |name_matcher| match dir_entry.file_name().to_str() {
            Some(s) => match name_matcher {
                NameMatcher::Regex(r) => Expr::KnownResult(r.is_match(s)),
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

    enum ContentMatcherInternal {
        RegexIndex(usize),
        IsUtf8,
    }
    let mut regexes = Vec::new();

    // harvest regexes, then read and run
    let e: ExprTree<Done, Done, ContentMatcherInternal> =
        run_stage(&e, never, never, |contents_matcher| {
            Expr::ContentsMatcher(match contents_matcher {
                ContentsMatcher::Regex(r) => {
                    regexes.push(r.as_str());
                    ContentMatcherInternal::RegexIndex(regexes.len() - 1)
                }
                ContentsMatcher::Utf8 => ContentMatcherInternal::IsUtf8,
            })
        });

    let mut matching_idxs = Vec::new();
    let mut is_utf8 = false;

    let regex_set = RegexSet::new(regexes.iter()).unwrap();

    let contents = fs::read(dir_entry.path())?;

    if let Ok(contents) = String::from_utf8(contents) {
        matching_idxs.extend(regex_set.matches(&contents).into_iter());
        is_utf8 = true;
    }

    let e: ExprTree<Done, Done, Done> = run_stage(&e, never, never, |c| {
        Expr::KnownResult(match c {
            ContentMatcherInternal::RegexIndex(regex_idx) => matching_idxs.contains(regex_idx),
            ContentMatcherInternal::IsUtf8 => is_utf8,
        })
    });

    // (assert) result is known at this point, no remaining branches

    if let Some(b) = e.known() {
        Ok(b)
    } else {
        unreachable!("programmer error")
    }
}
