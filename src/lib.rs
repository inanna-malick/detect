mod eval;
pub mod expr;
pub mod parser;
pub mod predicate;
pub mod query;
mod util;

use std::{collections::HashSet, path::Path, sync::Arc, time::Instant};

use anyhow::Context;
use expr::{Expr, MetadataPredicate, NamePredicate};
use git2::{DiffDelta, DiffOptions, Repository, RepositoryOpenFlags, TreeWalkResult};
use ignore::WalkBuilder;
use parser::parse_expr;
use predicate::{Predicate, StreamingCompiledContentPredicate};
use slog::{debug, error, info, warn, Logger};

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

pub fn run_git_range<F: FnMut(&str)>(
    logger: Logger,
    root: &Path,
    range: &str,
    expr: Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
    mut on_match: F,
) -> Result<(), anyhow::Error> {
    let repository = Repository::open_ext(
        root,
        RepositoryOpenFlags::empty(),
        &[] as &[&std::ffi::OsStr],
    )?;

    // Parse the range (e.g., "HEAD~10..HEAD" or "main..feature-branch")
    let revspec = repository.revparse(range)?;
    
    let from_commit = if let Some(from) = revspec.from() {
        from.as_commit()
            .ok_or(anyhow::Error::msg("'from' is not a commit"))?
    } else {
        anyhow::bail!("Invalid range: missing 'from' commit");
    };
    
    let to_commit = revspec
        .to()
        .ok_or(anyhow::Error::msg("Invalid range: missing 'to' commit"))?
        .as_commit()
        .ok_or(anyhow::Error::msg("'to' is not a commit"))?;

    info!(logger, "Searching git range"; "from" => %from_commit.id(), "to" => %to_commit.id());

    // Get the diff between the two commits
    let from_tree = from_commit.tree()?;
    let to_tree = to_commit.tree()?;
    
    let mut diff_options = DiffOptions::new();
    let diff = repository.diff_tree_to_tree(Some(&from_tree), Some(&to_tree), Some(&mut diff_options))?;
    
    // Collect all files that were added or modified
    let mut changed_files = HashSet::new();
    diff.foreach(
        &mut |delta: DiffDelta, _progress| {
            // We're interested in files that exist in the 'to' state
            if let Some(new_file) = delta.new_file().path() {
                if let Some(path_str) = new_file.to_str() {
                    changed_files.insert(path_str.to_string());
                }
            }
            true
        },
        None,
        None,
        None,
    )?;

    info!(logger, "Found changed files"; "count" => changed_files.len());

    // Now evaluate the expression against the changed files at the 'to' commit
    let expr = expr.map_predicate_ref(|p| match p {
        Predicate::Name(n) => Predicate::Name(Arc::clone(n)),
        Predicate::Metadata(m) => Predicate::Metadata(Arc::clone(m)),
        Predicate::Content(c) => Predicate::Content(c.as_ref()),
    });

    // Walk the tree at the 'to' commit, but only process files that were changed
    to_tree.walk(git2::TreeWalkMode::PreOrder, |parent_path, entry| {
        let start = Instant::now();

        let Some(name) = entry.name() else {
            warn!(logger, "entry without name? weird, skip and continue"; "parent_path" => parent_path);
            return TreeWalkResult::Skip
        };
        let path = format!("{}{}", parent_path, name);

        // Skip if this file wasn't changed in the range
        if !changed_files.contains(&path) {
            return TreeWalkResult::Ok;
        }

        debug!(logger, "walk changed file"; "path" => &path);

        match eval::git::eval(&logger, &expr, &repository, &path, entry).context(format!(
            "failed to eval for ${} ${}",
            entry.name().unwrap_or("[unnamed]"),
            entry.id()
        )) {
            Ok(is_match) => {
                let duration = start.elapsed();

                debug!(logger, "visited changed entity"; "duration" => #?duration, "result" => is_match);

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
) -> Result<(), anyhow::Error> {
    match parse_expr(&expr) {
        Ok(expr) => {
            let walker = WalkBuilder::new(root).git_ignore(respect_gitignore).build();

            let expr = expr.map_predicate_ref(|p| match p {
                Predicate::Name(n) => Predicate::Name(Arc::clone(n)),
                Predicate::Metadata(m) => Predicate::Metadata(Arc::clone(m)),
                Predicate::Content(c) => Predicate::Content(c.as_ref()),
            });

            info!(logger, "parsed expression"; "expr" => %expr);

            for entry in walker.into_iter() {
                let entry = entry?;
                let path = entry.path();

                let start = Instant::now();

                let is_match = eval::fs::eval(&logger, &expr, path)
                    .await
                    .context(format!("failed to eval for ${path:?}"))?;

                let duration = start.elapsed();

                debug!(logger, "visited entity"; "path" => #?path, "duration" => #?duration, "result" => is_match);

                if is_match {
                    on_match(path);
                }
            }

            Ok(())
        }
        Err(err) => panic!("{:?}", err),
    }
}
