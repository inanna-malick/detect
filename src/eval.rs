use recursion_schemes::recursive::collapse::Collapsable;

use crate::expr::frame::Operator;
use crate::expr::short_circuit::ShortCircuit;
use crate::expr::{frame::ExprFrame, Expr};
use crate::expr::{ContentPredicate, MetadataPredicate, NamePredicate};
use crate::predicate::Predicate;
use crate::util::Done;
use tokio::fs;
use std::path::Path;

/// multipass evaluation with short circuiting, runs, in order:
/// - file name matchers
/// - metadata matchers
/// - file content matchers
pub async fn eval(
    e: &Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>,
    path: &Path,
) -> std::io::Result<bool> {
    let e: Expr<Predicate<Done, &MetadataPredicate, &ContentPredicate>> =
        match e.collapse_frames(|frame| match frame {
            ExprFrame::Operator(op) => op.attempt_short_circuit(),
            ExprFrame::Predicate(p) => p.eval_name_predicate(path),
        }) {
            ShortCircuit::Known(x) => return Ok(x),
            ShortCircuit::Unknown(e) => e,
        };

    // read metadata via STAT syscall
    let metadata = fs::metadata(path).await?;

    let e: Expr<Predicate<Done, Done, &ContentPredicate>> =
        match e.collapse_frames(|frame| match frame {
            ExprFrame::Operator(op) => op.attempt_short_circuit(),
            ExprFrame::Predicate(p) => p.eval_metadata_predicate(&metadata),
        }) {
            ShortCircuit::Known(x) => return Ok(x),
            ShortCircuit::Unknown(e) => e,
        };

    // only try to read contents if it's a file according to entity metadata
    let utf8_contents = if metadata.is_file() {
        // read file contents via multiple syscalls
        let contents = fs::read(path).await?;
        String::from_utf8(contents).ok()
    } else {
        None
    };

    let res = e.collapse_frames::<bool>(|frame| match frame {
        // don't attempt short circuiting, because we know we can calculate a result here
        ExprFrame::Operator(op) => match op {
            Operator::Not(x) => !x,
            Operator::And(a, b) => a && b,
            Operator::Or(a, b) => a || b,
        },
        ExprFrame::Predicate(p) => p.eval_file_content_predicate(utf8_contents.as_deref()),
    });

    Ok(res)
}