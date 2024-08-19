use crate::expr::short_circuit::ShortCircuit;
use crate::expr::Expr;
use crate::expr::{MetadataPredicate, NamePredicate};
use crate::predicate::{CompiledContentPredicateRef, Predicate};
use crate::util::Done;
use futures::StreamExt;
use regex_automata::dfa::Automaton;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{self, BufStream};
use tokio_util::io::ReaderStream;

/// multipass evaluation with short circuiting, runs, in order:
/// - file name matchers
/// - metadata matchers
/// - file content matchers
pub async fn eval<'dfa>(
    e: &'dfa Expr<Predicate<NamePredicate, MetadataPredicate, CompiledContentPredicateRef<'dfa>>>,
    path: &Path,
) -> std::io::Result<bool> {
    let e: Expr<Predicate<Done, MetadataPredicate, CompiledContentPredicateRef<'dfa>>> =
        e.reduce_predicate_and_short_circuit(|p| p.eval_name_predicate(path));

    if let Expr::Literal(b) = e {
        return Ok(b);
    }

    // open file handle and read metadata
    let file = File::open(path).await?;

    let metadata = file.metadata().await?;

    let e: Expr<Predicate<Done, Done, CompiledContentPredicateRef<'dfa>>> =
        e.reduce_predicate_and_short_circuit(|p| p.eval_metadata_predicate(&metadata));

    if let Expr::Literal(b) = e {
        return Ok(b);
    }

    let e: Expr<Predicate<Done, Done, Done>> = if metadata.is_file() {
        run_contents_predicate(e, file).await?
    } else {
        e.reduce_predicate_and_short_circuit(|p| match p {
            // not a file, so no content predicates match
            Predicate::Content(_) => ShortCircuit::Known(false),
            _ => unreachable!(),
        })
    };

    if let Expr::Literal(b) = e {
        Ok(b)
    } else {
        // this is unreachable because at this point we've replaced every
        // predicate with boolean literals and reduced all binary operators
        unreachable!("programmer error")
    }
}

async fn run_contents_predicate(
    e: Expr<Predicate<Done, Done, CompiledContentPredicateRef<'_>>>,
    file: File,
) -> io::Result<Expr<Predicate<Done, Done, Done>>> {
    let mut s = ReaderStream::new(BufStream::new(file));

    // TODO: customize config, probably
    let config = regex_automata::util::start::Config::new();

    let mut e: Expr<Predicate<Done, Done, _>> = e.map_predicate(|p| match p {
        Predicate::Content(dfa) => {
            let s = dfa
                .start_state(&config)
                .expect("programmer error, probably");
            Predicate::Content((dfa, s))
        }
        _ => unreachable!(),
    });

    while let Some(next) = s.next().await {
        // read the next buffered chunk of bytes
        let bytes = next?;

        // advance each dfa and short-circuit if possible
        e = e.reduce_predicate_and_short_circuit(|p| match p {
            Predicate::Content((dfa, state)) => {
                let mut next_state = state;
                let mut iter = bytes.iter();

                loop {
                    if let Some(byte) = iter.next() {
                        next_state = dfa.next_state(next_state, *byte);

                        if dfa.is_match_state(next_state) {
                            break ShortCircuit::Known(true);
                        }

                        if dfa.is_dead_state(next_state) {
                            break ShortCircuit::Known(false);
                        }
                    } else {
                        break ShortCircuit::Unknown(Predicate::Content((dfa, next_state)));
                    }
                }
            }
            _ => unreachable!(),
        });
    }

    let e = e.reduce_predicate_and_short_circuit(|p| match p {
        Predicate::Content((dfa, state)) => {
            let next_state = dfa.next_eoi_state(state);
            if dfa.is_match_state(next_state) {
                ShortCircuit::Known(true)
            } else {
                ShortCircuit::Known(false)
            }
        }
        _ => unreachable!(),
    });

    Ok(e)
}
