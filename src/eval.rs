use recursion_schemes::recursive::RecursiveExt;

use crate::expr::recurse::{ShortCircuit, Operator};
use crate::expr::{recurse::ExprLayer, Expr};
use crate::expr::{ContentPredicate, MetadataPredicate, NamePredicate};
use crate::predicate::Predicate;
use crate::util::Done;
use std::fs::{self};
use std::path::Path;

// multipass evaluation with short circuiting, runs, in order:
// - file name matchers
// - metadata matchers
// - file content matchers
pub fn eval(
    e: &Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>,
    path: &Path,
) -> std::io::Result<bool> {
    let e: Expr<Predicate<Done, &MetadataPredicate, &ContentPredicate>> =
        match e.fold_recursive(|layer| match layer {
            ExprLayer::Operator(op) => op.attempt_short_circuit(),
            ExprLayer::Predicate(p) => p.run_phase(path),
        }) {
            ShortCircuit::Known(x) => return Ok(x),
            ShortCircuit::Unknown(e) => e,
        };

    // read metadata via STAT syscall
    let metadata = fs::metadata(path)?;

    let e: Expr<Predicate<Done, Done, &ContentPredicate>> =
        match e.fold_recursive(|layer| match layer {
            ExprLayer::Operator(op) => op.attempt_short_circuit(),
            ExprLayer::Predicate(p) => p.run_phase(&metadata),
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

    let res: bool = e.fold_recursive::<bool, _>(|layer| match layer {
        // no more short circuiting, we know we can calculate a result here
        ExprLayer::Operator(op) => match op {
            Operator::Not(x) => !x,
            Operator::And(a, b) => a && b,
            Operator::Or(a, b) => a || b,
        },
        ExprLayer::Predicate(p) => p.run_phase(utf8_contents.as_deref()),
    });

    Ok(res)
}
