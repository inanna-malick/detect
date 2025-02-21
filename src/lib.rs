mod eval;
pub mod expr;
pub mod parser;
pub mod predicate;
mod util;

use std::{path::Path, sync::Arc, time::Instant};

use anyhow::Context;
use expr::{Expr, MetadataPredicate, NamePredicate};
use ignore::WalkBuilder;
use parser::parse_expr;
use predicate::{CompiledContentPredicate, Predicate};
use slog::{debug, info, Logger};

pub async fn parse_and_run_github<F: FnMut(&str)>(
    logger: Logger,
    // root: &Path,
    // respect_gitignore: bool,
    expr: Expr<Predicate<NamePredicate, MetadataPredicate, CompiledContentPredicate>>,
    mut on_match: F,
) -> Result<(), anyhow::Error> {
    use octorust::{auth::Credentials, Client};

    let github = Client::new(String::from("detect-cli-tool"), None).unwrap();

    let owner = "inanna-malick";
    let repo = "detect";
    let ref_ = "heads/main";

    let ref_ = github.git().get_ref(owner, repo, ref_).await?;

    let commit = github
        .git()
        .get_commit(owner, repo, &ref_.body.object.sha)
        .await?;

    // assertion: since recursive is passed in this will be the full tree (maybe? need to learn how large trees are handled)
    let tree = github
        .git()
        .get_tree(owner, repo, &commit.body.tree.sha, "true")
        .await?;

    let expr = expr.map_predicate_ref(|p| match p {
        Predicate::Name(n) => Predicate::Name(Arc::clone(n)),
        Predicate::Metadata(m) => Predicate::Metadata(Arc::clone(m)),
        Predicate::Content(c) => Predicate::Content(c.as_ref()),
    });

    for entry in tree.body.tree.into_iter() {
        // let path = Path::new(&entry.path);

        let start = Instant::now();

        let is_match = eval::github::eval(&logger, &expr, &github, &entry, owner, repo)
            .await
            .context(format!("failed to eval for ${entry:?}"))?;

        let duration = start.elapsed();

        debug!(logger, "visited entity"; "duration" => #?duration, "result" => is_match);

        if is_match {
            let sha = &commit.body.sha;
            let path = entry.path;
            let url = format!("https://github.com/{owner}/{repo}/blob/{sha}/{path}");
// 
            on_match(&url);
        }

    }

    // println!("tree: {:?}", tree.body.tree);
    // let tree_entries = tree.body.tree.into_i

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
