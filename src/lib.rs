mod eval;
pub mod expr;
mod parser;
pub mod predicate;
mod util;

use std::{path::Path, sync::Arc, time::Instant};

use anyhow::Context;
use expr::{Expr, MetadataPredicate, NamePredicate};
use ignore::WalkBuilder;
use nom_locate::LocatedSpan;
use nom_recursive::RecursiveInfo;
use parser::{convert_error, expr};
use predicate::{CompiledContentPredicate, Predicate};
use slog::{debug, info, Logger};

use crate::eval::eval;

type ContentPredicate = CompiledContentPredicate;

pub fn parse(
    data: &str,
) -> Result<Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>, String> {
    let data: LocatedSpan<&str, RecursiveInfo> = LocatedSpan::new_extra(data, RecursiveInfo::new());

    match expr(data) {
        Ok(x) => Ok(x.1),
        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => Err(convert_error(data, e)),
        Err(nom::Err::Incomplete(_)) => {
            unimplemented!("should not hit incomplete case")
        }
    }
}

pub async fn parse_and_run<F: FnMut(&Path)>(
    logger: Logger,
    root: &Path,
    respect_gitignore: bool,
    expr: String,
    mut on_match: F,
) -> Result<(), anyhow::Error> {
    match parse(&expr) {
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
        Err(err) => panic!("{}", err),
    }
}
