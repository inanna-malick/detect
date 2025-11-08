use crate::expr::short_circuit::ShortCircuit;
use crate::expr::Expr;
use crate::predicate::{Predicate, StreamingCompiledContentPredicateRef};
use crate::util::Done;
use futures::{Stream, StreamExt};
use regex_automata::dfa::Automaton;
use tokio::io::{self};

pub mod fs;
pub mod structured;

pub async fn run_contents_predicate_stream(
    e: Expr<Predicate<Done, Done, StreamingCompiledContentPredicateRef<'_>>>,
    mut s: impl Stream<Item = io::Result<Vec<u8>>> + std::marker::Unpin,
) -> io::Result<Expr<Predicate<Done, Done, Done>>> {
    let config = regex_automata::util::start::Config::new();

    // Initialize state for DFA patterns
    let mut e: Expr<Predicate<Done, Done, _>> = e.map_predicate(|p| match p {
        Predicate::Content(pred) => {
            let dfa = pred.inner;
                let s = dfa
                    .start_state(&config)
                    .expect("DFA start_state failed: invalid regex configuration");
                Predicate::Content((dfa, s))
        },
        _ => unreachable!(),
    });

    while let Some(next) = s.next().await {
        // read the next buffered chunk of bytes
        let bytes = next?;

        // advance each pattern appropriately
        e = e.reduce_predicate_and_short_circuit(move |p| match p {
            Predicate::Content((dfa, state)) => {
                        // DFA streaming processing
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
                                break ShortCircuit::Unknown(Predicate::Content((
                                    dfa,
                                    next_state,
                                )));
                            }
                        }
            }
            _ => unreachable!(),
        });
    }

    // Final evaluation
    let e = e.reduce_predicate_and_short_circuit(|p| match p {
        Predicate::Content((dfa, state)) => {
                    let next_state = dfa.next_eoi_state(state);
                    let matched = dfa.is_match_state(next_state);
                    ShortCircuit::Known(matched)
        }
        _ => unreachable!(),
    });

    Ok(e)
}
