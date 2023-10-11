use crate::expr::Expr;
use crate::expr::{ContentPredicate, MetadataPredicate, NamePredicate};
use crate::predicate::ProcessPredicate;
use crate::util::Done;
use futures::FutureExt;
use std::os::unix::prelude::MetadataExt;
use std::path::Path;
use tokio::fs::{self};

// file size at which program-based matchers are run before file content matchers
const LARGE_FILE_SIZE: u64 = 1024 * 1024; // totally arbitrary

/// multipass evaluation with short circuiting, runs, in order:
/// - file name matchers
/// - metadata matchers
/// - file content matchers
pub async fn eval(
    e: &Expr<NamePredicate, MetadataPredicate, ContentPredicate, ProcessPredicate>,
    path: &Path,
) -> std::io::Result<bool> {
    let e: Expr<Done, MetadataPredicate, ContentPredicate, ProcessPredicate> =
        e.map_predicate(|p| p.eval_name_predicate(path)).reduce();

    if let Expr::Literal(b) = e {
        return Ok(b);
    }

    // read metadata
    let metadata = fs::metadata(path).await?;

    let e: Expr<Done, Done, ContentPredicate, ProcessPredicate> = e
        .map_predicate(|p| p.eval_metadata_predicate(&metadata))
        .reduce();

    if let Expr::Literal(b) = e {
        return Ok(b);
    }

    // the ordering of the file contents and process predicates is determined by file
    // size - for large files it makes more sense to run processes
    // (that may just peek at the first few bytes) first
    let e: Expr<Done, Done, Done, Done> = if metadata.size() > LARGE_FILE_SIZE {
        // run program-based matchers first
        let e: Expr<Done, Done, ContentPredicate, Done> = e
            .map_predicate_async(|p| p.eval_async_predicate(path).boxed())
            .await?
            .reduce();

        // only try to read contents if it's a file according to entity metadata
        let utf8_contents = if metadata.is_file() {
            // read contents
            let contents = fs::read(path).await?;
            String::from_utf8(contents).ok()
        } else {
            None
        };

        e.map_predicate(|p| p.eval_file_content_predicate(utf8_contents.as_ref()))
            .reduce()
    } else {
        // only try to read contents if it's a file according to entity metadata
        let utf8_contents = if metadata.is_file() {
            // read contents
            let contents = fs::read(path).await?;
            String::from_utf8(contents).ok()
        } else {
            None
        };

        let e: Expr<Done, Done, Done, ProcessPredicate> = e
            .map_predicate(|p| p.eval_file_content_predicate(utf8_contents.as_ref()))
            .reduce();

        e.map_predicate_async(|p| p.eval_async_predicate(path).boxed())
            .await?
            .reduce()
    };

    if let Expr::Literal(b) = e {
        Ok(b)
    } else {
        // this is unreachable because at this point we've replaced every
        // predicate with boolean literals and reduced all binary operators
        unreachable!("programmer error")
    }
}
