mod eval;
pub mod expr;
pub mod parser;
pub mod predicate;
mod util;

use std::{path::Path, sync::Arc, time::Instant};

use anyhow::Context;
use expr::{MetadataPredicate, NamePredicate};
use ignore::WalkBuilder;
use parser::parse_expr;
use predicate::Predicate;
use slog::{debug, info, Logger};

use crate::eval::eval;

pub async fn parse_and_run<F: FnMut(&Path)>(
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

                let is_match = eval(&logger, &expr, path)
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
