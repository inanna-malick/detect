#![doc = include_str!("../README.md")]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::cargo)]

pub mod eval;
pub mod expr;
pub mod hybrid_regex;
pub mod parser;
pub mod predicate;
mod predicate_error;
pub mod util;

use std::{path::Path, sync::Arc, time::Instant};

use anyhow::Context;
use ignore::WalkBuilder;
use parser::{error::DetectError, RawParser, Typechecker};
use predicate::Predicate;
use slog::{debug, info, Logger};

/// Runtime configuration for detect operations
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum file size (in bytes) for structured data parsing (YAML/JSON/TOML)
    /// Files larger than this will skip structured data evaluation
    pub max_structured_size: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_structured_size: 10 * 1024 * 1024, // 10MB default
        }
    }
}

pub async fn parse_and_run_fs<F: FnMut(&Path)>(
    logger: Logger,
    root: &Path,
    respect_gitignore: bool,
    expr: String,
    config: RuntimeConfig,
    mut on_match: F,
) -> Result<(), DetectError> {
    let original_query = expr.clone();
    let parse_result = RawParser::parse_raw_expr(&expr)
        .and_then(|raw_expr| Typechecker::typecheck(raw_expr, &expr, &config));

    match parse_result {
        Ok(parsed_expr) => {
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
                Predicate::Structured(s) => Predicate::Structured(s.clone()),
            });

            info!(logger, "parsed expression"; "expr" => %expr);

            let mut match_count = 0;
            for entry in walker.into_iter() {
                let entry = entry.map_err(|e| DetectError::from(anyhow::Error::from(e)))?;
                let path = entry.path();

                if path == root {
                    continue;
                }

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
