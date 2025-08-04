pub mod ast;
pub mod diagnostics;
pub mod error;
pub mod error_hints;
mod eval;
pub mod expr;
pub mod output;
pub mod parse_error;
pub mod parser;
pub mod predicate;
mod util;

#[cfg(test)]
mod parser_tests;

#[cfg(test)]
mod proptest_generators;

use std::{path::Path, sync::Arc, time::Instant};

use anyhow::Context;
use error::DetectError;
use expr::{MetadataPredicate, NamePredicate};
use ignore::WalkBuilder;
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

                let is_match = eval::fs::eval_with_base(&logger, &expr, path, Some(root))
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
