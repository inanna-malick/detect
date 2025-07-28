mod eval;
pub mod expr;
pub mod parser;
pub mod predicate;
mod util;
pub mod error_hints;
pub mod error;
pub mod parse_error;

#[cfg(test)]
mod parser_tests;

use std::{path::Path, sync::Arc, time::Instant};

use anyhow::Context;
use expr::{Expr, MetadataPredicate, NamePredicate};
use git2::{Repository, RepositoryOpenFlags, TreeWalkResult};
use ignore::WalkBuilder;
use parser::parse_expr;
use predicate::{Predicate, StreamingCompiledContentPredicate};
use slog::{debug, error, info, warn, Logger};
use error::DetectError;

pub fn run_git<F: FnMut(&str)>(
    logger: Logger,
    root: &Path,
    ref_: &str,
    expr: Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
    mut on_match: F,
) -> Result<(), anyhow::Error> {
    let repository = Repository::open_ext(
        root,
        RepositoryOpenFlags::empty(),
        &[] as &[&std::ffi::OsStr],
    )?;

    let ref_ = repository.revparse_single(ref_).context("ref not found")?;
    let commit = ref_
        .as_commit()
        .ok_or(anyhow::Error::msg("non-commit ref target"))?;

    let tree = repository.find_tree(commit.tree_id())?;

    let expr = expr.map_predicate_ref(|p| match p {
        Predicate::Name(n) => Predicate::Name(Arc::clone(n)),
        Predicate::Metadata(m) => Predicate::Metadata(Arc::clone(m)),
        Predicate::Content(c) => Predicate::Content(c.as_ref()),
    });

    tree.walk(git2::TreeWalkMode::PreOrder, |parent_path, entry| {
        let start = Instant::now();

        let Some(name) = entry.name() else {
            warn!(logger, "entry without name? weird, skip and continue"; "parent_path" => parent_path);
            return TreeWalkResult::Skip
        };
        let path = format!("{}{}", parent_path, name);

        debug!(logger, "walk"; "path" => &path);

        match eval::git::eval(&logger, &expr, &repository, &path, entry).context(format!(
            "failed to eval for ${} ${}",
            entry.name().unwrap_or("[unnamed]"),
            entry.id()
        )) {
            Ok(is_match) => {
                let duration = start.elapsed();

                debug!(logger, "visited entity"; "duration" => #?duration, "result" => is_match);

                if is_match {
                    on_match(&path);
                }

                TreeWalkResult::Ok
            }
            Err(e) => {
                error!(logger, "failed reading git entry, aborting tree walk"; "err" => #?e);
                TreeWalkResult::Abort
            }
        }
    })?;

    Ok(())
}

pub async fn parse_and_run_fs<F: FnMut(&Path)>(
    logger: Logger,
    root: &Path,
    respect_gitignore: bool,
    expr: String,
    mut on_match: F,
) -> Result<(), DetectError> {
    match parse_expr(&expr) {
        Ok(parsed_expr) => {
            let walker = WalkBuilder::new(root).hidden(false).git_ignore(respect_gitignore).build();

            let expr = parsed_expr.map_predicate_ref(|p| match p {
                Predicate::Name(n) => Predicate::Name(Arc::clone(n)),
                Predicate::Metadata(m) => Predicate::Metadata(Arc::clone(m)),
                Predicate::Content(c) => Predicate::Content(c.as_ref()),
            });

            info!(logger, "parsed expression"; "expr" => %expr);

            for entry in walker.into_iter() {
                let entry = entry.map_err(|e| DetectError::from(anyhow::Error::from(e)))?;
                let path = entry.path();

                let start = Instant::now();

                let is_match = eval::fs::eval(&logger, &expr, path)
                    .await
                    .context(format!("failed to eval for ${path:?}"))
                    .map_err(DetectError::from)?;

                let duration = start.elapsed();

                debug!(logger, "visited entity"; "path" => #?path, "duration" => #?duration, "result" => is_match);

                if is_match {
                    on_match(path);
                }
            }

            Ok(())
        }
        Err(err) => {
            Err(err.into())
        }
    }
}
