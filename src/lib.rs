#![doc = include_str!("../README.md")]

pub mod eval;
pub mod expr;
mod hybrid_regex;
#[cfg(feature = "mcp")]
pub mod mcp_server;
pub mod parser;
pub mod predicate;
mod predicate_error;
mod util;

use std::{path::Path, sync::Arc, time::Instant};

use anyhow::Context;
use ignore::WalkBuilder;
use parser::{error::DetectError, RawParser, Typechecker};
use predicate::Predicate;
use slog::{debug, info, Logger};

pub async fn parse_and_run_fs<F: FnMut(&Path)>(
    logger: Logger,
    root: &Path,
    respect_gitignore: bool,
    expr: String,
    mut on_match: F,
) -> Result<(), DetectError> {
    // Use parser: parse then typecheck
    let original_query = expr.clone();
    let parse_result = RawParser::parse_raw_expr(&expr)
        .and_then(|raw_expr| Typechecker::typecheck(raw_expr, &expr));

    match parse_result {
        Ok(parsed_expr) => {
            // Validate root path exists and is a directory
            if !root.exists() {
                return Err(DetectError::DirectoryNotFound {
                    path: root.display().to_string(),
                });
            }
            if !root.is_dir() {
                return Err(DetectError::NotADirectory {
                    path: root.display().to_string(),
                });
            }

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
                Predicate::StructuredData(s) => Predicate::StructuredData(s.clone()),
            });

            info!(logger, "parsed expression"; "expr" => %expr);

            let mut match_count = 0;
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
                    match_count += 1;
                    on_match(path);
                }
            }

            // Provide helpful feedback when no matches are found
            if match_count == 0 {
                eprintln!("No files matched the query: {}", original_query);
                eprintln!("Searched in: {}", root.display());
                if respect_gitignore {
                    eprintln!("Hint: Use -i flag to include gitignored files, or try broadening your search criteria");
                } else {
                    eprintln!("Hint: Try broadening your search criteria or check the path/expression syntax");
                }
            }

            Ok(())
        }
        Err(err) => Err(err),
    }
}
