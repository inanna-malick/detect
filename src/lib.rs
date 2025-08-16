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
use predicate::Predicate;
use slog::{debug, info, Logger};
use v2_parser::{RawParseError, RawParser, TypecheckError, Typechecker};

// Convert v2_parser's RawParseError to DetectError
fn convert_raw_parse_error(err: RawParseError, _source: Arc<str>) -> DetectError {
    match err {
        RawParseError::Syntax(_pest_err) => {
            // For now, convert to a generic syntax error
            // TODO: Convert pest error properly when Rule types align
            DetectError::from(anyhow::anyhow!("Syntax error in expression"))
        }
        RawParseError::InvalidEscape { char, position } => DetectError::from(anyhow::anyhow!(
            "Invalid escape sequence '\\{}' at position {}",
            char,
            position
        )),
        RawParseError::UnterminatedEscape => DetectError::from(anyhow::anyhow!(
            "Unterminated escape sequence at end of string"
        )),
        RawParseError::Internal(msg) => {
            DetectError::from(anyhow::anyhow!("Internal parser error: {}", msg))
        }
    }
}

// Convert v2_parser's TypecheckError to DetectError
fn convert_typecheck_error(err: TypecheckError, _source: Arc<str>) -> DetectError {
    // Convert typechecker errors to anyhow errors for now
    // The v1 ParseError doesn't have direct mappings for these
    match err {
        TypecheckError::UnknownSelector(sel) => {
            DetectError::from(anyhow::anyhow!("Unknown selector: {}", sel))
        }
        TypecheckError::UnknownOperator(op) => {
            DetectError::from(anyhow::anyhow!("Unknown operator: {}", op))
        }
        TypecheckError::IncompatibleOperator { selector, operator } => {
            DetectError::from(anyhow::anyhow!(
                "Operator '{}' is not compatible with selector '{}'",
                operator,
                selector
            ))
        }
        TypecheckError::InvalidValue { expected, found } => DetectError::from(anyhow::anyhow!(
            "Expected {} value, found: {}",
            expected,
            found
        )),
        TypecheckError::Internal(msg) => {
            DetectError::from(anyhow::anyhow!("Internal error: {}", msg))
        }
    }
}

pub async fn parse_and_run_fs<F: FnMut(&Path)>(
    logger: Logger,
    root: &Path,
    respect_gitignore: bool,
    expr: String,
    mut on_match: F,
) -> Result<(), DetectError> {
    let expr_source = std::sync::Arc::from(expr.as_str());

    // Use v2_parser: parse then typecheck
    let parse_result = RawParser::parse_raw_expr(&expr)
        .map_err(|e| convert_raw_parse_error(e, Arc::clone(&expr_source)))
        .and_then(|raw_expr| {
            Typechecker::typecheck(raw_expr)
                .map_err(|e| convert_typecheck_error(e, Arc::clone(&expr_source)))
        });

    match parse_result {
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
        Err(err) => Err(err),
    }
}
