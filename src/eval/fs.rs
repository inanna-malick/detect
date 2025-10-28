use crate::expr::short_circuit::ShortCircuit;
use crate::expr::Expr;
use crate::predicate::{
    MetadataPredicate, NamePredicate, Predicate, StreamingCompiledContentPredicateRef,
};
use crate::util::Done;
use futures::{stream, TryStreamExt};
use slog::{debug, o, Logger};
use std::path::Path;
use tokio::fs::File;
use tokio::io::BufStream;
use tokio_util::io::ReaderStream;

use crate::eval::run_contents_predicate_stream;
use crate::eval::structured::{eval_structured_predicate, ParsedDocuments};

/// multipass evaluation with short circuiting, runs, in order:
/// - file name matchers
/// - metadata matchers
/// - file content matchers
pub async fn eval<'dfa>(
    logger: &Logger,
    e: &'dfa Expr<
        Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicateRef<'dfa>>,
    >,
    path: &Path,
    base_path: Option<&Path>,
) -> std::io::Result<bool> {
    let logger = logger.new(o!("path" => format!("{:?}", path)));

    debug!(logger, "visit entity"; "expr" => %e);

    let e: Expr<Predicate<Done, MetadataPredicate, StreamingCompiledContentPredicateRef<'dfa>>> =
        e.reduce_predicate_and_short_circuit(|p| p.eval_name_predicate_with_base(path, base_path));

    if let Expr::Literal(b) = e {
        debug!(logger, "short circuit after path predicate eval"; "expr" => %e, "result" => %b);
        return Ok(b);
    }

    debug!(logger, "reduced expr after path predicate eval";  "expr" => %e);

    // open file handle and read metadata
    let file = File::open(path).await?;

    let metadata = file.metadata().await?;

    let e: Expr<Predicate<Done, Done, StreamingCompiledContentPredicateRef<'dfa>>> = e
        .reduce_predicate_and_short_circuit(|p| {
            p.eval_metadata_predicate_with_path(&metadata, path, base_path)
        });

    if let Expr::Literal(b) = e {
        debug!(logger, "short circuit after metadata predicate eval";  "expr" => %e, "result" => %b);
        return Ok(b);
    }

    debug!(logger, "reduced expr after metadata predicate eval";  "expr" => %e);

    // Determine which predicates remain for optimized file reading
    let has_structured = e.contains_structured_predicates();
    let has_content = e.contains_content_predicates();

    if !metadata.is_file() {
        debug!(
            logger,
            "not a file, all structured/content predicates eval to false"
        );
        let e: Expr<Predicate<Done, Done, Done, Done>> = e.reduce_predicate_and_short_circuit(|p| match p {
            Predicate::Content(_) => ShortCircuit::Known(false),
            Predicate::Structured(_) => ShortCircuit::Known(false),
            _ => unreachable!("only Content and Structured predicates should remain after metadata phase"),
        });

        if let Expr::Literal(b) = e {
            debug!(logger, "evaluation finished"; "result" => b);
            return Ok(b);
        } else {
            unreachable!("all predicates should be reduced to literals after evaluation")
        }
    }

    match (has_structured, has_content) {
        (true, true) => {
            debug!(logger, "evaluating both structured and content predicates - single file read");
            // Read file once as bytes
            let bytes = tokio::fs::read(path).await?;

            // Try to interpret as UTF-8
            match std::str::from_utf8(&bytes) {
                Ok(contents) => {
                    // UTF-8: evaluate structured predicates first
                    let mut cache = ParsedDocuments::new();
                    let e = e.reduce_predicate_and_short_circuit(|p| match p {
                        Predicate::Structured(s) => {
                            match eval_structured_predicate(&s, contents, &mut cache) {
                                Ok(result) => ShortCircuit::Known(result),
                                Err(_) => ShortCircuit::Known(false),
                            }
                        }
                        Predicate::Content(c) => ShortCircuit::Unknown(Predicate::Content(c)),
                        _ => unreachable!("only Structured and Content predicates should remain"),
                    });

                    // Check if short-circuited after structured
                    if let Expr::Literal(b) = e {
                        debug!(logger, "short circuit after structured predicates"; "result" => b);
                        return Ok(b);
                    }

                    // Evaluate content predicates using in-memory stream (8KB chunks)
                    const CHUNK_SIZE: usize = 8192;
                    let chunks: Vec<Result<Vec<u8>, std::io::Error>> = bytes
                        .chunks(CHUNK_SIZE)
                        .map(|chunk| Ok(chunk.to_vec()))
                        .collect();

                    let e = run_contents_predicate_stream(e, stream::iter(chunks)).await?;

                    if let Expr::Literal(b) = e {
                        debug!(logger, "evaluation finished"; "result" => b);
                        Ok(b)
                    } else {
                        unreachable!("all content predicates should be reduced to literals after streaming")
                    }
                }
                Err(_) => {
                    debug!(logger, "file is not UTF-8, structured predicates = false, using streaming content");
                    // Non-UTF-8: structured predicates fail, stream content
                    let e = e.reduce_predicate_and_short_circuit(|p| match p {
                        Predicate::Structured(_) => ShortCircuit::Known(false),
                        Predicate::Content(c) => ShortCircuit::Unknown(Predicate::Content(c)),
                        _ => unreachable!("only Structured and Content predicates should remain"),
                    });

                    if let Expr::Literal(b) = e {
                        debug!(logger, "short circuit after structured=false"; "result" => b);
                        return Ok(b);
                    }

                    // Stream bytes for content matching
                    const CHUNK_SIZE: usize = 8192;
                    let chunks: Vec<Result<Vec<u8>, std::io::Error>> = bytes
                        .chunks(CHUNK_SIZE)
                        .map(|chunk| Ok(chunk.to_vec()))
                        .collect();

                    let e = run_contents_predicate_stream(e, stream::iter(chunks)).await?;

                    if let Expr::Literal(b) = e {
                        debug!(logger, "evaluation finished"; "result" => b);
                        Ok(b)
                    } else {
                        unreachable!("all content predicates should be reduced to literals after streaming")
                    }
                }
            }
        }
        (true, false) => {
            debug!(logger, "evaluating structured predicates only");
            // Read file as string for structured evaluation
            let e = match tokio::fs::read_to_string(path).await {
                Ok(contents) => {
                    let mut cache = ParsedDocuments::new();
                    e.reduce_predicate_and_short_circuit(|p| match p {
                        Predicate::Structured(s) => {
                            match eval_structured_predicate(&s, &contents, &mut cache) {
                                Ok(result) => ShortCircuit::<Predicate<Done, Done, Done>>::Known(result),
                                Err(_) => ShortCircuit::<Predicate<Done, Done, Done>>::Known(false),
                            }
                        }
                        _ => unreachable!("only Structured predicates should remain when has_content is false"),
                    })
                }
                Err(_) => {
                    // Non-UTF-8 or read error: all structured predicates = false
                    e.reduce_predicate_and_short_circuit(|p| match p {
                        Predicate::Structured(_) => ShortCircuit::<Predicate<Done, Done, Done>>::Known(false),
                        _ => unreachable!("only Structured predicates should remain when has_content is false"),
                    })
                }
            };

            if let Expr::Literal(b) = e {
                debug!(logger, "evaluation finished"; "result" => b);
                Ok(b)
            } else {
                unreachable!("all structured predicates should be reduced to literals after evaluation")
            }
        }
        (false, true) => {
            debug!(logger, "evaluating content predicates only - streaming");
            // Original streaming path
            let e = run_contents_predicate_stream(
                e,
                ReaderStream::new(BufStream::new(file)).map_ok(|b| b.to_vec()),
            )
            .await?;

            if let Expr::Literal(b) = e {
                debug!(logger, "evaluation finished"; "result" => b);
                Ok(b)
            } else {
                unreachable!("all content predicates should be reduced to literals after streaming")
            }
        }
        (false, false) => {
            // No structured or content predicates remain (already short-circuited)
            unreachable!("both has_structured and has_content are false - should have short-circuited")
        }
    }
}
