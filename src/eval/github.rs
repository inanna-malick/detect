use crate::expr::short_circuit::ShortCircuit;
use crate::expr::Expr;
use crate::expr::{MetadataPredicate, NamePredicate};
use crate::predicate::{CompiledContentPredicateRef, Predicate};
use crate::util::Done;
use anyhow::Context as _;
use futures::TryStreamExt;
use octorust::types::GitTree;
use octorust::Client;
use slog::{debug, o, Logger};
use std::path::Path;
use tokio::fs::File;
use tokio::io::BufStream;
use tokio_util::io::ReaderStream;

use crate::eval::{github, run_contents_predicate};

/// multipass evaluation with short circuiting, runs, in order:
/// - file name matchers
/// - metadata matchers
/// - file content matchers
pub async fn eval<'dfa>(
    logger: &Logger,
    e: &'dfa Expr<Predicate<NamePredicate, MetadataPredicate, CompiledContentPredicateRef<'dfa>>>,
    client: &Client,
    entry: &GitTree,
    owner: &str,
    repo: &str,
) -> anyhow::Result<bool> {
    let path = Path::new(&entry.path);
    let logger = logger.new(o!("path" => format!("{:?}", path)));

    debug!(logger, "visit entity"; "expr" => %e);

    // TODO: fuse this w/ metadata? ig
    let e: Expr<Predicate<Done, MetadataPredicate, CompiledContentPredicateRef<'dfa>>> =
        e.reduce_predicate_and_short_circuit(|p| p.eval_name_predicate(path));

    if let Expr::Literal(b) = e {
        debug!(logger, "short circuit after path predicate eval"; "expr" => %e, "result" => %b);
        return Ok(b);
    }

    debug!(logger, "reduced expr after path predicate eval";  "expr" => %e);

    let e: Expr<Predicate<Done, Done, CompiledContentPredicateRef<'dfa>>> =
        e.reduce_predicate_and_short_circuit(|p| p.eval_metadata_predicate_github(&entry));

    if let Expr::Literal(b) = e {
        debug!(logger, "short circuit after metadata predicate eval";  "expr" => %e, "result" => %b);
        return Ok(b);
    }

    debug!(logger, "reduced expr after metadata predicate eval";  "expr" => %e);

    let e: Expr<Predicate<Done, Done, Done>> = if entry.type_ == "blob" {
        debug!(logger, "evaluating file content predicates");
        // TODO: write custom impl to allow for streaming binary response, this is just for the prototype
        use base64::prelude::*;

        let blob = client.git().get_blob(owner, repo, &entry.sha).await?;
        let content: String = blob.body.content.chars().filter(|c| *c != '\n').collect();
        let content = BASE64_STANDARD.decode(content).context("b64")?;
        run_contents_predicate(e, futures::stream::iter(Some(Ok(content)))).await?
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
