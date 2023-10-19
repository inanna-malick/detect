use crate::expr::Expr;
use crate::expr::{ContentPredicate, MetadataPredicate, NamePredicate};
use crate::predicate::Predicate;
use crate::util::Done;
use std::fs::{self};
use std::path::Path;

/// multipass evaluation with short circuiting, runs, in order:
/// - file name matchers
/// - metadata matchers
/// - file content matchers
pub fn eval(
    e: &Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>,
    path: &Path,
) -> std::io::Result<bool> {
    let e: Expr<Predicate<Done, MetadataPredicate, ContentPredicate>> =
        e.reduce_predicate_and_short_circuit(|p| p.eval_name_predicate(path));

    if let Expr::Literal(b) = e {
        return Ok(b);
    }

    // read metadata
    let metadata = fs::metadata(path)?;

    let e: Expr<Predicate<Done, Done, ContentPredicate>> =
        e.reduce_predicate_and_short_circuit(|p| p.eval_metadata_predicate(&metadata));

    if let Expr::Literal(b) = e {
        return Ok(b);
    }

    // only try to read contents if it's a file according to entity metadata
    let utf8_contents = if metadata.is_file() {
        // read contents
        let contents = fs::read(path)?;
        String::from_utf8(contents).ok()
    } else {
        None
    };

    let e: Expr<Predicate<Done, Done, Done>> = e.reduce_predicate_and_short_circuit(|p| {
        p.eval_file_content_predicate(utf8_contents.as_ref())
    });

    if let Expr::Literal(b) = e {
        Ok(b)
    } else {
        // this is unreachable because at this point we've replaced every
        // predicate with boolean literals and reduced all binary operators
        unreachable!("programmer error")
    }
}
