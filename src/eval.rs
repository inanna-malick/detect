use recursion_schemes::recursive::RecursiveExt;

use crate::expr::recurse::ShortCircuit;
use crate::expr::{recurse::ExprLayer, BorrowedExpr, Expr, OwnedExpr};
use crate::util::Done;
use std::fs::{self};
use std::path::Path;

// multipass evaluation with short circuiting, runs, in order:
// - file name matchers
// - metadata matchers
// - file content matchers
pub fn eval(e: &OwnedExpr, path: &Path) -> std::io::Result<bool> {
    let e: BorrowedExpr<Done> = match e.fold_recursive(|layer| {
        match layer {
            // evaluate all NamePredicate predicates
            ExprLayer::Name(p) => ShortCircuit::Known(p.is_match(path)),
            // boilerplate
            ExprLayer::Operator(op) => op.attempt_short_circuit(),
            ExprLayer::Metadata(p) => ShortCircuit::Unknown(Expr::Metadata(p)),
            ExprLayer::Contents(p) => ShortCircuit::Unknown(Expr::Contents(p)),
        }
    }) {
        ShortCircuit::Known(x) => return Ok(x),
        ShortCircuit::Unknown(e) => e,
    };

    // read metadata via STAT syscall
    let metadata = fs::metadata(path)?;

    let e: BorrowedExpr<Done, Done> = match e.fold_recursive(|layer| {
        match layer {
            // evaluate all MetadataPredicate predicates
            ExprLayer::Metadata(p) => ShortCircuit::Known(p.is_match(&metadata)),
            // boilerplate
            ExprLayer::Operator(op) => op.attempt_short_circuit(),
            ExprLayer::Contents(p) => ShortCircuit::Unknown(Expr::Contents(*p)),
            // unreachable: predicate already evaluated
            ExprLayer::Name(_) => unreachable!("name predicate has already been evaluated"),
        }
    }) {
        ShortCircuit::Known(x) => return Ok(x),
        ShortCircuit::Unknown(e) => e,
    };

    // only try to read contents if it's a file according to entity metadata
    let utf8_contents = if metadata.is_file() {
        // read file contents via multiple syscalls
        let contents = fs::read(path)?;
        String::from_utf8(contents).ok()
    } else {
        None
    };

    match e.fold_recursive(|layer| {
        match layer {
            // evaluate all ContentPredicate predicates
            ExprLayer::Contents(p) => {
                // only examine contents if we have a valid utf8 contents string
                let is_match = utf8_contents.as_ref().map_or(false, |s| p.is_match(s));
                ShortCircuit::Known::<Expr<Done, Done, Done>>(is_match)
            }
            // boilerplate
            ExprLayer::Operator(op) => op.attempt_short_circuit(),
            // unreachable: predicates already evaluated
            ExprLayer::Name(_) => unreachable!("name predicate has already been evaluated"),
            ExprLayer::Metadata(_) => unreachable!("metadata predicate has already been evaluated"),
        }
    }) {
        ShortCircuit::Known(x) => return Ok(x),
        ShortCircuit::Unknown(_) => {
            panic!("programmer error, should have known result after all predicates evaluated")
        }
    }
}
