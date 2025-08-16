mod eval;
pub mod expr;
mod hybrid_regex;
pub mod mcp_server;
pub mod parse_error;
pub mod parser;
pub mod predicate;
mod util;
pub mod v2_parser;

use std::{path::Path, sync::Arc, time::Instant};

use anyhow::Context;
use ignore::WalkBuilder;
use parse_error::DetectError;
use parser::parse_expr;
use predicate::Predicate;
use slog::{debug, info, Logger};

pub async fn parse_and_run_fs<F: FnMut(&Path)>(
    logger: Logger,
    root: &Path,
    respect_gitignore: bool,
    expr: String,
    mut on_match: F,
) -> Result<(), DetectError> {
    let expr_source = std::sync::Arc::from(expr.as_str());

    match parse_expr(&expr) {
        Ok(parsed_expr) => {
            let walker = WalkBuilder::new(root)
                .hidden(false)
                .git_ignore(respect_gitignore)
                .filter_entry(|entry| {
                    // Always exclude VCS directories, regardless of gitignore settings
                    // This matches ripgrep's behavior
                    !entry
                        .file_name()
                        .to_str()
                        .map(|s| s == ".git" || s == ".hg" || s == ".svn")
                        .unwrap_or(false)
                })
                .build();

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

                let is_match = eval::fs::eval(&logger, &expr, path, Some(root))
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
        Err(err) => Err(DetectError::parse_with_source(err, expr_source)),
    }
}
