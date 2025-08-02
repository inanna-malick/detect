use crate::expr::short_circuit::ShortCircuit;
use crate::expr::Expr;
use crate::expr::{MetadataPredicate, NamePredicate};
use crate::predicate::{Predicate, StreamingCompiledContentPredicateRef};
use crate::util::Done;
use futures::TryStreamExt;
use slog::{debug, o, Logger};
use std::path::Path;
use tokio::fs::File;
use tokio::io::BufStream;
use tokio_util::io::ReaderStream;

use crate::eval::run_contents_predicate_stream;

/// multipass evaluation with short circuiting, runs, in order:
/// - file name matchers
/// - metadata matchers
/// - file content matchers
#[allow(dead_code)]
pub async fn eval<'dfa>(
    logger: &Logger,
    e: &'dfa Expr<
        Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicateRef<'dfa>>,
    >,
    path: &Path,
) -> std::io::Result<bool> {
    eval_with_base(logger, e, path, None).await
}

/// multipass evaluation with short circuiting and base path for relative paths
pub async fn eval_with_base<'dfa>(
    logger: &Logger,
    e: &'dfa Expr<
        Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicateRef<'dfa>>,
    >,
    path: &Path,
    base_path: Option<&Path>,
) -> std::io::Result<bool> {
    let logger = logger.new(o!("path" => format!("{:?}", path)));

    debug!(logger, "visit entity"; "expr" => %e);

    let e: Expr<Predicate<Done, MetadataPredicate, StreamingCompiledContentPredicateRef<'dfa>>> =
        e.reduce_predicate_and_short_circuit(|p| p.eval_name_predicate_with_base(path, base_path));

    if let Expr::Literal(b) = e {
        debug!(logger, "short circuit after path predicate eval"; "expr" => %e, "result" => %b);
        return Ok(b);
    }

    debug!(logger, "reduced expr after path predicate eval";  "expr" => %e);

    // open file handle and read metadata
    let file = File::open(path).await?;

    let metadata = file.metadata().await?;

    let e: Expr<Predicate<Done, Done, StreamingCompiledContentPredicateRef<'dfa>>> = e
        .reduce_predicate_and_short_circuit(|p| {
            p.eval_metadata_predicate_with_path(&metadata, path, base_path)
        });

    if let Expr::Literal(b) = e {
        debug!(logger, "short circuit after metadata predicate eval";  "expr" => %e, "result" => %b);
        return Ok(b);
    }

    debug!(logger, "reduced expr after metadata predicate eval";  "expr" => %e);

    let e: Expr<Predicate<Done, Done, Done>> = if metadata.is_file() {
        debug!(logger, "evaluating file content predicates");
        run_contents_predicate_stream(
            e,
            ReaderStream::new(BufStream::new(file)).map_ok(|b| b.to_vec()),
        )
        .await?
    } else {
        debug!(
            logger,
            "not a file, all file content predicates eval to false"
        );
        e.reduce_predicate_and_short_circuit(|p| match p {
            // not a file, so no content predicates match
            Predicate::Content(_) => ShortCircuit::Known(false),
            _ => unreachable!(),
        })
    };

    if let Expr::Literal(b) = e {
        debug!(logger, "file contents predicate eval finished, no predicates remain";  "result" => %b);
        Ok(b)
    } else {
        // this is unreachable because at this point we've replaced every
        // predicate with boolean literals and reduced all binary operators
        unreachable!("programmer error")
    }
}
