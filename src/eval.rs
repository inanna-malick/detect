use crate::expr::{recurse::ExprLayer, BorrowedExpr, Expr, OwnedExpr};
use crate::util::Done;
use recursion::Collapse;
use std::fs::{self};
use std::path::Path;

// multipass evaluation with short circuiting, runs, in order:
// - file name matchers
// - metadata matchers
// - file content matchers
pub fn eval(e: &OwnedExpr, path: &Path) -> std::io::Result<bool> {
    let e: BorrowedExpr<Done> = e.collapse_layers(|layer| {
        match layer {
            // evaluate all NamePredicate predicates
            ExprLayer::Name(p) => Expr::KnownResult(p.is_match(path)),
            // boilerplate
            ExprLayer::Operator(op) => op.attempt_short_circuit(),
            ExprLayer::KnownResult(k) => Expr::KnownResult(k),
            ExprLayer::Metadata(p) => Expr::Metadata(p),
            ExprLayer::Contents(p) => Expr::Contents(p),
        }
    });

    // short circuit before querying metadata (expensive)
    if let Expr::KnownResult(b) = e {
        return Ok(b);
    }

    // read metadata via STAT syscall
    let metadata = fs::metadata(path)?;

    // TODO: move walkdir stuff to main, thus making a lib specific to just a few things
    let e: BorrowedExpr<Done, Done> = e.collapse_layers(|layer| {
        match layer {
            // evaluate all MetadataPredicate predicates
            ExprLayer::Metadata(p) => Expr::KnownResult(p.is_match(&metadata)),
            // boilerplate
            ExprLayer::Operator(op) => op.attempt_short_circuit(),
            ExprLayer::KnownResult(k) => Expr::KnownResult(k),
            ExprLayer::Contents(p) => Expr::Contents(*p),
            // unreachable: predicate already evaluated
            ExprLayer::Name(_) => unreachable!("name predicate has already been evaluated"),
        }
    });

    // short circuit before reading contents (even more expensive)
    if let Expr::KnownResult(b) = e {
        return Ok(b);
    }

    // only try to read contents if it's a file according to entity metadata
    let utf8_contents = if metadata.is_file() {
        // read file contents via multiple syscalls
        let contents = fs::read(path)?;
        String::from_utf8(contents).ok()
    } else {
        None
    };

    let e: BorrowedExpr<Done, Done, Done> = e.collapse_layers(|layer| {
        match layer {
            // evaluate all ContentPredicate predicates
            ExprLayer::Contents(p) => {
                // only examine contents if we have a valid utf8 contents string
                let is_match = utf8_contents.as_ref().map_or(false, |s| p.is_match(s));
                Expr::KnownResult(is_match)
            }
            // boilerplate
            ExprLayer::Operator(op) => op.attempt_short_circuit(),
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
