use crate::expr::short_circuit::ShortCircuit;
use crate::expr::Expr;
use crate::hybrid_regex::HybridRegex;
use crate::predicate::{Predicate, StreamingCompiledContentPredicateRef};
use crate::util::Done;
use futures::{Stream, StreamExt};
use regex_automata::dfa::Automaton;
use tokio::io::{self};

pub mod fs;

pub async fn run_contents_predicate_stream(
    e: Expr<Predicate<Done, Done, StreamingCompiledContentPredicateRef<'_>>>,
    mut s: impl Stream<Item = io::Result<Vec<u8>>> + std::marker::Unpin,
) -> io::Result<Expr<Predicate<Done, Done, Done>>> {
    let config = regex_automata::util::start::Config::new();

    // Initialize state for DFA patterns
    let mut e: Expr<Predicate<Done, Done, _>> = e.map_predicate(|p| match p {
        Predicate::Content(pred) => match &pred.inner {
            HybridRegex::RustDFA(dfa) => {
                let s = dfa
                    .start_state(&config)
                    .expect("DFA start_state failed: invalid regex configuration");
                Predicate::Content((pred, Some(s), Vec::new()))
            }
            HybridRegex::Pcre2(_) => {
                // PCRE2 patterns accumulate buffer
                Predicate::Content((pred, None, Vec::new()))
            }
        },
        _ => unreachable!(),
    });

    while let Some(next) = s.next().await {
        // read the next buffered chunk of bytes
        let bytes = next?;

        // advance each pattern appropriately
        e = e.reduce_predicate_and_short_circuit(move |p| match p {
            Predicate::Content((pred, state, mut buffer)) => {
                match &pred.inner {
                    HybridRegex::RustDFA(dfa) => {
                        // DFA streaming processing
                        let mut next_state = state.unwrap();
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
                                break ShortCircuit::Unknown(Predicate::Content((
                                    pred,
                                    Some(next_state),
                                    buffer,
                                )));
                            }
                        }
                    }
                    HybridRegex::Pcre2(_) => {
                        // PCRE2 needs full buffer
                        buffer.extend_from_slice(&bytes);
                        ShortCircuit::Unknown(Predicate::Content((pred, None, buffer)))
                    }
                }
            }
            _ => unreachable!(),
        });
    }

    // Final evaluation
    let e = e.reduce_predicate_and_short_circuit(|p| match p {
        Predicate::Content((pred, state, buffer)) => {
            match &pred.inner {
                HybridRegex::RustDFA(dfa) => {
                    let next_state = dfa.next_eoi_state(state.unwrap());
                    let matched = dfa.is_match_state(next_state);
                    ShortCircuit::Known(matched)
                }
                HybridRegex::Pcre2(_) => {
                    // Now check PCRE2 pattern against accumulated buffer
                    let matched = pred.inner.is_match(&buffer);
                    ShortCircuit::Known(matched)
                }
            }
        }
        _ => unreachable!(),
    });

    Ok(e)
}
