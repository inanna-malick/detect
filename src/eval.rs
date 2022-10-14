use crate::expr::{ContentsMatcher, Expr, ExprLayer, MetadataMatcher, NameMatcher};
use crate::util::Done;
use recursion::Collapse;
use std::path::Path;
use std::{
    fs::{self},
    os::unix::prelude::MetadataExt,
};

// multipass evaluation with short circuiting, runs, in order:
// - file name matchers
// - metadata matchers
// - file content matchers
pub fn eval(
    e: &Expr<NameMatcher, MetadataMatcher, ContentsMatcher>,
    path: &Path,
) -> std::io::Result<bool> {
    let e: Expr<Done, &MetadataMatcher, &ContentsMatcher> = e.collapse_layers(|layer| {
        match layer {
            // evaluate all NameMatcher predicates
            ExprLayer::Name(name_matcher) => match path.to_str() {
                Some(s) => match name_matcher {
                    NameMatcher::Regex(r) => Expr::KnownResult(r.is_match(s)),
                },
                None => Expr::KnownResult(false),
            },
            // boilerplate
            ExprLayer::Operator(x) => match x.eval() {
                None => Expr::Operator(Box::new(x)),
                Some(k) => Expr::KnownResult(k),
            },
            ExprLayer::KnownResult(k) => Expr::KnownResult(k),
            ExprLayer::Metadata(p) => Expr::Metadata(p),
            ExprLayer::Contents(p) => Expr::Contents(p),
        }
    });

    // short circuit before querying metadata (expensive)
    if let Expr::KnownResult(b) = e {
        return Ok(b);
    }

    let metadata = fs::metadata(path)?;

    let e: Expr<Done, Done, &ContentsMatcher> = e.collapse_layers(|layer| {
        match layer {
            // evaluate all MetadataMatcher predicates
            ExprLayer::Metadata(p) => match p {
                MetadataMatcher::Filesize(range) => {
                    Expr::KnownResult(range.contains(&metadata.size()))
                }
            },
            // boilerplate
            ExprLayer::Operator(x) => match x.eval() {
                None => Expr::Operator(Box::new(x)),
                Some(k) => Expr::KnownResult(k),
            },
            ExprLayer::KnownResult(k) => Expr::KnownResult(k),
            ExprLayer::Contents(p) => Expr::Contents(*p),
            // unreachable: predicate already evaluated
            ExprLayer::Name(_) => unreachable!("name predicate has already been evaluated"),
        }
    });

    // short circuit before reading file contents (even more expensive)
    if let Expr::KnownResult(b) = e {
        return Ok(b);
    }

    let contents = fs::read(path)?;
    let utf8_contents = String::from_utf8(contents).ok();

    let e: Expr<Done, Done, Done> = e.collapse_layers(|layer| {
        match layer {
            // evaluate all MetadataMatcher predicates
            ExprLayer::Contents(p) => Expr::KnownResult({
                if let Some(utf8_contents) = utf8_contents.as_ref() {
                    match p {
                        ContentsMatcher::Regex(regex) => regex.is_match(utf8_contents),
                        ContentsMatcher::Utf8 => true,
                    }
                } else {
                    false
                }
            }),
            // boilerplate
            ExprLayer::Operator(x) => match x.eval() {
                None => Expr::Operator(Box::new(x)),
                Some(k) => Expr::KnownResult(k),
            },
            ExprLayer::KnownResult(k) => Expr::KnownResult(k),
            // unreachable: predicates already evaluated
            ExprLayer::Name(_) => unreachable!("name predicate has already been evaluated"),
            ExprLayer::Metadata(_) => unreachable!("metadata predicate has already been evaluated"),
        }
    });

    if let Expr::KnownResult(b) = e {
        Ok(b)
    } else {
        panic!("programmer error, should have known result after all predicates evaluated")
    }
}
