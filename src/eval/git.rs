use crate::expr::Expr;
use crate::expr::{MetadataPredicate, NamePredicate};
use crate::predicate::{Predicate, StreamingCompiledContentPredicateRef};
use crate::util::Done;
use git2::{ObjectType, Repository, TreeEntry};
use slog::{debug, o, Logger};
use std::path::Path;

use crate::eval::run_contents_predicate;

/// multipass evaluation with short circuiting, runs, in order:
/// - file name matchers
/// - metadata matchers
/// - file content matchers
pub fn eval<'dfa>(
    logger: &Logger,
    e: &'dfa Expr<
        Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicateRef<'dfa>>,
    >,
    repository: &Repository,
    path: &str,
    entry: &TreeEntry,
) -> anyhow::Result<bool> {
    let path = Path::new(path);

    let logger = logger.new(o!("path" => format!("{:?}", path)));

    debug!(logger, "visit entity"; "expr" => %e);

    // TODO: fuse this w/ metadata? ig
    let e: Expr<Predicate<Done, MetadataPredicate, StreamingCompiledContentPredicateRef<'dfa>>> =
        e.reduce_predicate_and_short_circuit(|p| p.eval_name_predicate(path));

    if let Expr::Literal(b) = e {
        debug!(logger, "short circuit after path predicate eval"; "expr" => %e, "result" => %b);
        return Ok(b);
    }

    debug!(logger, "reduced expr after path predicate eval";  "expr" => %e);

    let e = match entry.kind() {
        Some(ObjectType::Blob) => {
            let obj = entry.to_object(repository)?;
            let blob = obj
                .as_blob()
                .ok_or(anyhow::Error::msg("libgit2 usage error - blob not a blob"))?;

            let e: Expr<Predicate<Done, Done, StreamingCompiledContentPredicateRef<'dfa>>> =
                e.reduce_predicate_and_short_circuit(|p| p.eval_metadata_predicate_git_blob(blob));

            debug!(logger, "reduced expr after metadata predicate eval";  "expr" => %e);

            if let Expr::Literal(b) = e {
                debug!(logger, "short circuit after metadata predicate eval";  "expr" => %e, "result" => %b);
                return Ok(b);
            }

            run_contents_predicate(e, blob.content())?
        }
        Some(ObjectType::Tree) => e.reduce_predicate_and_short_circuit(|p| match p {
            Predicate::Metadata(m) => match m.as_ref() {
                MetadataPredicate::Filesize(_) => false,
                MetadataPredicate::Type(string_matcher) => {
                    string_matcher.is_match("directory") | string_matcher.is_match("dir")
                }
                MetadataPredicate::Modified(_)
                | MetadataPredicate::Created(_)
                | MetadataPredicate::Accessed(_) => {
                    // Git trees don't have timestamps
                    false
                }
            },
            Predicate::Content(_) => false,
            Predicate::Name(_) => unreachable!(),
        }),
        // not sure how we got here but it's definitely not a match
        None | Some(ObjectType::Any) | Some(ObjectType::Commit) | Some(ObjectType::Tag) => {
            Expr::Literal(false)
        }
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
