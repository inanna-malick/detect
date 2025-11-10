#![doc = include_str!("../README.md")]
#![warn(clippy::all)]
#![warn(clippy::cargo)]

pub mod eval;
pub mod expr;
pub mod parser;
pub mod predicate;
mod predicate_error;
pub mod util;

use std::{path::Path, sync::Arc, time::Instant};

use ignore::WalkBuilder;
use parser::{error::DetectError, RawParser, Typechecker};
use predicate::Predicate;
use slog::{debug, info, warn, Logger};

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
) -> Result<usize, DetectError> {
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
                        .is_some_and(|s| s == ".git" || s == ".hg" || s == ".svn")
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
            for entry in walker {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        // Skip entries we can't access (permission denied, etc.)
                        warn!(logger, "skipping entry due to walker error"; "error" => %e);
                        continue;
                    }
                };
                let path = entry.path();

                if path == root {
                    continue;
                }

                let start = Instant::now();

                let is_match = match eval::fs::eval(&logger, &expr, path, Some(root)).await {
                    Ok(result) => result,
                    Err(e) => {
                        // Handle I/O errors gracefully - skip files we can't access
                        if e.kind() == std::io::ErrorKind::PermissionDenied {
                            debug!(logger, "skipping file due to permission denied"; "path" => #?path);
                            continue;
                        }
                        // For other I/O errors, also skip but log at warning level
                        warn!(logger, "skipping file due to I/O error"; "path" => #?path, "error" => %e);
                        continue;
                    }
                };

                let duration = start.elapsed();

                debug!(logger, "visited entity"; "path" => #?path, "duration" => #?duration, "result" => is_match);

                if is_match {
                    match_count += 1;
                    on_match(path);
                }
            }

            if match_count == 0 {
                eprintln!("No files matched the query: {original_query}");
                eprintln!("Searched in: {}", root.display());
                if respect_gitignore {
                    eprintln!("Hint: Use -i flag to include gitignored files, or try broadening your search criteria");
                } else {
                    eprintln!("Hint: Try broadening your search criteria or check the path/expression syntax");
                }
            }

            Ok(match_count)
        }
        Err(err) => Err(err),
    }
}
