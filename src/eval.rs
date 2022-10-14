use crate::expr::{ContentsMatcher, Expr, ExprLayer, MetadataMatcher, NameMatcher};
use crate::util::Done;
use recursion::Collapse;
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
pub(crate) fn eval(e: &Expr, dir_entry: DirEntry) -> std::io::Result<bool> {
    use ExprLayer::*;
    let e: Expr<Done, &MetadataMatcher, &ContentsMatcher> = e.collapse_layers(|layer| {
        Expr::new(match layer {
            Operator(x) => match x.eval() {
                None => Operator(x),
                Some(k) => KnownResult(k),
            },
            // evaluate all NameMatcher predicates
            Name(name_matcher) => match dir_entry.file_name().to_str() {
                Some(s) => match name_matcher {
                    NameMatcher::Regex(r) => KnownResult(r.is_match(s)),
                },
                None => KnownResult(false),
            },
            // boilerplate
            KnownResult(k) => KnownResult(k),
            Metadata(p) => Metadata(p),
            Contents(p) => Contents(p),
        })
    });

    if let Some(b) = e.known() {
        return Ok(b);
    }

    // short circuit or query metadata (expensive)
    let metadata = dir_entry.metadata()?;

    let e: Expr<Done, Done, &ContentsMatcher> = e.collapse_layers(|layer| {
        Expr::new(match layer {
            Operator(x) => match x.eval() {
                None => Operator(x),
                Some(k) => KnownResult(k),
            },
            Metadata(p) => match p {
                MetadataMatcher::Filesize(range) => KnownResult(range.contains(&metadata.size())),
            },
            // boilerplate
            KnownResult(k) => KnownResult(k),
            Contents(p) => Contents(*p),
            // already processed
            Name(_) => unreachable!("already evaluated as witnessed by uninhabitated type"),
        })
    });

    if let Some(b) = e.known() {
        return Ok(b);
    }

    enum ContentMatcherInternal {
        RegexIndex(usize),
        IsUtf8,
    }
    let mut regexes = Vec::new();

    // harvest regexes so we can run a single fused RegexSet pass
    let e: Expr<Done, Done, ContentMatcherInternal> = e.collapse_layers(|layer| {
        Expr::new(match layer {
            Operator(x) => match x.eval() {
                None => Operator(x),
                Some(k) => KnownResult(k),
            },
            Contents(p) => Contents(match p {
                ContentsMatcher::Regex(r) => {
                    regexes.push(r.as_str());
                    ContentMatcherInternal::RegexIndex(regexes.len() - 1)
                }
                ContentsMatcher::Utf8 => ContentMatcherInternal::IsUtf8,
            }),
            // boilerplate
            KnownResult(k) => KnownResult(k),
            // already processed
            Name(_) => unreachable!("already evaluated as witnessed by uninhabitated type"),
            Metadata(_) => unreachable!("already evaluated as witnessed by uninhabitated type"),
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

    let e: Expr<Done, Done, Done> = e.collapse_layers(|layer| {
        Expr::new(match layer {
            Operator(x) => match x.eval() {
                None => Operator(x),
                Some(k) => KnownResult(k),
            },
            Contents(p) => KnownResult(match p {
                ContentMatcherInternal::RegexIndex(regex_idx) => matching_idxs.contains(regex_idx),
                ContentMatcherInternal::IsUtf8 => is_utf8,
            }),
            // boilerplate
            KnownResult(k) => KnownResult(k),
            // already processed
            Name(_) => unreachable!("already evaluated as witnessed by uninhabitated type"),
            Metadata(_) => unreachable!("already evaluated as witnessed by uninhabitated type"),
        })
    });

    Ok(e.known()
        .expect("all predicates evaluated, should have known result"))
}
