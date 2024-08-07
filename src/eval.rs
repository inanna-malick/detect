use crate::expr::Expr;
use crate::expr::{ContentPredicate, MetadataPredicate, NamePredicate};
use crate::predicate::Predicate;
use crate::util::Done;
use std::fs::Metadata;
use std::path::Path;
use tokio::fs::{self};
use tokio::io;

/// multipass evaluation with short circuiting, runs, in order:
/// - file name matchers
/// - metadata matchers
/// - file content matchers
pub async fn eval(
    e: &Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>,
    path: &Path,
) -> std::io::Result<bool> {
    let e: Expr<Predicate<Done, MetadataPredicate, ContentPredicate>> =
        e.reduce_predicate_and_short_circuit(|p| p.eval_name_predicate(path));

    if let Expr::Literal(b) = e {
        return Ok(b);
    }

    // read metadata
    let metadata = fs::metadata(path).await?;

    let e: Expr<Predicate<Done, Done, ContentPredicate>> =
        e.reduce_predicate_and_short_circuit(|p| p.eval_metadata_predicate(&metadata));

    if let Expr::Literal(b) = e {
        return Ok(b);
    }

    let e: Expr<Predicate<Done, Done, Done>> = run_contents_predicate(e, metadata, path).await?;

    if let Expr::Literal(b) = e {
        Ok(b)
    } else {
        // this is unreachable because at this point we've replaced every
        // predicate with boolean literals and reduced all binary operators
        unreachable!("programmer error")
    }
}

async fn run_contents_predicate<A, B>(
    e: Expr<Predicate<A, B, ContentPredicate>>,
    metadata: Metadata,
    path: &Path,
) -> io::Result<Expr<Predicate<A, B, Done>>> {
    // only try to read contents if it's a file according to entity metadata
    let utf8_contents = if metadata.is_file() {
        // read contents
        let contents = fs::read(path).await?;
        String::from_utf8(contents).ok()
    } else {
        None
    };

    let e = e.reduce_predicate_and_short_circuit(|p| {
        p.eval_file_content_predicate(utf8_contents.as_ref())
    });

    Ok(e)
}
