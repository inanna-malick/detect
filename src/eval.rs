use crate::expr::Expr;
use crate::expr::{ContentPredicate, MetadataPredicate, NamePredicate};
use crate::util::Done;
use std::fs::{self};
use std::path::Path;

/// multipass evaluation with short circuiting, runs, in order:
/// - file name matchers
/// - metadata matchers
/// - file content matchers
pub fn eval(
    e: &Expr<NamePredicate, MetadataPredicate, ContentPredicate>,
    path: &Path,
) -> std::io::Result<bool> {
    let e: Expr<Done, MetadataPredicate, ContentPredicate> =
        e.map_predicate(|p| p.eval_name_predicate(path)).reduce();

    if let Expr::Literal(b) = e {
        return Ok(b);
    }

    // read metadata
    let metadata = fs::metadata(path)?;

    let e: Expr<Done, Done, ContentPredicate> = e
        .map_predicate(|p| p.eval_metadata_predicate(&metadata))
        .reduce();

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

    let e: Expr<Done, Done, Done> = e
        .map_predicate(|p| p.eval_file_content_predicate(utf8_contents.as_ref()))
        .reduce();

    if let Expr::Literal(b) = e {
        Ok(b)
    } else {
        unreachable!("programmer error")
    }
}
