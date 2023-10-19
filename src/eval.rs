use crate::expr::Expr;
use crate::expr::{ContentPredicate, MetadataPredicate, NamePredicate};
use crate::predicate::{Predicate, ProcessPredicate};
use crate::util::Done;
use futures::FutureExt;
use std::fs::Metadata;
use std::os::unix::prelude::MetadataExt;
use std::path::Path;
use tokio::fs::{self};
use tokio::io;

// file size at which program-based matchers are run before file content matchers
const LARGE_FILE_SIZE: u64 = 1024 * 1024; // totally arbitrary

/// multipass evaluation with short circuiting, runs, in order:
/// - file name matchers
/// - metadata matchers
/// - file content matchers
pub async fn eval(
    e: &Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate, ProcessPredicate>>,
    path: &Path,
) -> std::io::Result<bool> {
    let e: Expr<Predicate<Done, MetadataPredicate, ContentPredicate, ProcessPredicate>> =
        e.reduce_predicate_and_short_circuit(|p| p.eval_name_predicate(path));

    if let Expr::Literal(b) = e {
        return Ok(b);
    }

    // read metadata
    let metadata = fs::metadata(path).await?;

    let e: Expr<Predicate<Done, Done, ContentPredicate, ProcessPredicate>> =
        e.reduce_predicate_and_short_circuit(|p| p.eval_metadata_predicate(&metadata));

    if let Expr::Literal(b) = e {
        return Ok(b);
    }

    // the ordering of the file contents and process predicates is determined by file
    // size - for large files it makes more sense to run processes
    // (that may just peek at the first few bytes) first
    let e: Expr<Predicate<Done, Done, Done, Done>> = if metadata.size() > LARGE_FILE_SIZE {
        // run program-based matchers first
        let e: Expr<Predicate<Done, Done, ContentPredicate, Done>> =
            run_process_predicate(e, path).await?;

        run_contents_predicate(e, metadata, path).await?
    } else {
        let e: Expr<Predicate<Done, Done, Done, ProcessPredicate>> =
            run_contents_predicate(e, metadata, path).await?;

        run_process_predicate(e, path).await?
    };

    if let Expr::Literal(b) = e {
        Ok(b)
    } else {
        // this is unreachable because at this point we've replaced every
        // predicate with boolean literals and reduced all binary operators
        unreachable!("programmer error")
    }
}

async fn run_process_predicate<A, B, C>(
    e: Expr<Predicate<A, B, C, ProcessPredicate>>,
    path: &Path,
) -> io::Result<Expr<Predicate<A, B, C, Done>>>
where
    A: Sync + Send + 'static,
    B: Sync + Send + 'static,
    C: Sync + Send + 'static,
{
    let e = e
        .reduce_predicate_and_short_circuit_async(|p| p.eval_async_predicate(path).boxed())
        .await?;

    Ok(e)
}

async fn run_contents_predicate<A, B, C>(
    e: Expr<Predicate<A, B, ContentPredicate, C>>,
    metadata: Metadata,
    path: &Path,
) -> io::Result<Expr<Predicate<A, B, Done, C>>> {
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
