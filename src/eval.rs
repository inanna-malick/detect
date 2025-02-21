use crate::expr::short_circuit::ShortCircuit;
use crate::expr::Expr;
use crate::predicate::{CompiledContentPredicateRef, Predicate};
use crate::util::Done;
use futures::{Stream, StreamExt, TryStreamExt};
use regex_automata::dfa::Automaton;
use slog::{debug, o, Logger};
use tokio::io::{self, BufStream};
use tokio_util::io::ReaderStream;

pub mod fs;
pub mod github;

pub async fn run_contents_predicate(
    e: Expr<Predicate<Done, Done, CompiledContentPredicateRef<'_>>>,
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
                            break ShortCircuit::Known(true);
                        }

                        if dfa.inner.is_dead_state(next_state) {
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
            let dfa = dfa.inner;
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
