use crate::expr::short_circuit::ShortCircuit;
use crate::expr::Expr;
use crate::predicate::{Predicate, StreamingCompiledContentPredicateRef};
use crate::util::Done;
use futures::{Stream, StreamExt};
use regex_automata::dfa::Automaton;
use tokio::io::{self};

pub mod fs;
pub mod git;

pub async fn run_contents_predicate_stream(
    e: Expr<Predicate<Done, Done, StreamingCompiledContentPredicateRef<'_>>>,
    mut s: impl Stream<Item = io::Result<Vec<u8>>> + std::marker::Unpin,
) -> io::Result<Expr<Predicate<Done, Done, Done>>> {
    // TODO: customize config, probably
    let config = regex_automata::util::start::Config::new();

    let mut e: Expr<Predicate<Done, Done, _>> = e.map_predicate(|p| match p {
        Predicate::Content(dfa) => {
            let s = dfa
                .inner
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
        e = e.reduce_predicate_and_short_circuit(move |p| match p {
            Predicate::Content((dfa, state)) => {
                let mut next_state = state;
                let mut iter = bytes.iter();

                loop {
                    if let Some(byte) = iter.next() {
                        next_state = dfa.inner.next_state(next_state, *byte);

                        if dfa.inner.is_match_state(next_state) {
                            // Apply negation if needed
                            break ShortCircuit::Known(!dfa.negate);
                        }

                        if dfa.inner.is_dead_state(next_state) {
                            // For negated patterns, dead state means no match, which is success
                            break ShortCircuit::Known(dfa.negate);
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
            let next_state = dfa.inner.next_eoi_state(state);

            // Check for match at end of input
            let matched = dfa.inner.is_match_state(next_state);
            // Apply negation if needed
            ShortCircuit::Known(matched != dfa.negate)
        }
        _ => unreachable!(),
    });

    Ok(e)
}

pub fn run_contents_predicate(
    e: Expr<Predicate<Done, Done, StreamingCompiledContentPredicateRef<'_>>>,
    buf: &[u8],
) -> io::Result<Expr<Predicate<Done, Done, Done>>> {
    // TODO: customize config, probably
    let config = regex_automata::util::start::Config::new();

    let mut e: Expr<Predicate<Done, Done, _>> = e.map_predicate(|p| match p {
        Predicate::Content(dfa) => {
            let s = dfa
                .inner
                .start_state(&config)
                .expect("programmer error, probably");
            Predicate::Content((dfa, s))
        }
        _ => unreachable!(),
    });

    // probably nothing close to optimal but idk w/e
    for bytes in buf.chunks(1024) {
        // advance each dfa and short-circuit if possible
        e = e.reduce_predicate_and_short_circuit(move |p| match p {
            Predicate::Content((dfa, state)) => {
                let mut next_state = state;
                let mut iter = bytes.iter();

                loop {
                    if let Some(byte) = iter.next() {
                        next_state = dfa.inner.next_state(next_state, *byte);

                        if dfa.inner.is_match_state(next_state) {
                            // Apply negation if needed
                            break ShortCircuit::Known(!dfa.negate);
                        }

                        if dfa.inner.is_dead_state(next_state) {
                            // For negated patterns, dead state means no match, which is success
                            break ShortCircuit::Known(dfa.negate);
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
            let next_state = dfa.inner.next_eoi_state(state);

            // Check for match at end of input
            let matched = dfa.inner.is_match_state(next_state);
            // Apply negation if needed
            ShortCircuit::Known(matched != dfa.negate)
        }
        _ => unreachable!(),
    });

    Ok(e)
}
