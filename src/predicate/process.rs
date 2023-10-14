use std::{fmt::Display, path::Path, process::Stdio};

use regex::Regex;
use tokio::sync::mpsc;

use crate::{expr::short_circuit::ShortCircuit, util::Done};

use super::Predicate;

// run cmd with file path as an argument
#[derive(Debug)]
pub enum ProcessPredicate {
    Process { cmd: String, expected_stdout: Regex },
}

// only used for tests
impl PartialEq for ProcessPredicate {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Process {
                    cmd: l_cmd,
                    expected_stdout: l_expected_stdout,
                },
                Self::Process {
                    cmd: r_cmd,
                    expected_stdout: r_expected_stdout,
                },
            ) => l_cmd == r_cmd && l_expected_stdout.as_str() == r_expected_stdout.as_str(),
        }
    }
}

impl Eq for ProcessPredicate {}

impl Display for ProcessPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessPredicate::Process {
                cmd,
                expected_stdout: expected,
            } => {
                write!(f, "process(cmd={}, expected={})", cmd, expected.as_str())
            }
        }
    }
}

impl<A, B, C> Predicate<A, B, C, ProcessPredicate> {
    pub async fn eval_async_predicate(
        self,
        file_path: &Path,
        process_ids: mpsc::Sender<u32>,
    ) -> std::io::Result<ShortCircuit<Predicate<A, B, C, Done>>> {
        match self {
            Predicate::Process(x) => match x.as_ref() {
                ProcessPredicate::Process {
                    cmd,
                    expected_stdout: expected,
                } => {
                    use tokio::process::*;

                    // TODO: propagate ctrl-c and etc to child processes
                    let child = Command::new(cmd)
                        .arg(file_path)
                        .stdin(Stdio::null())
                        .stderr(Stdio::piped())
                        .stdout(Stdio::piped())
                        .spawn()?;

                    if let Some(processid) = child.id() {
                        // trac subprocess ids
                        process_ids.send(processid).await.unwrap();
                    }

                    let status = child.wait_with_output().await?;

                    let stdout = status.stdout;

                    // let stderr = status.stderr;
                    // println!("stderr: {:?}", String::from_utf8(stderr));

                    // if parse utf8 then run regex on stdout
                    let res = match String::from_utf8(stdout) {
                        Ok(utf8) => {
                            // println!("stdout == {}", utf8);
                            expected.is_match(&utf8)
                        }
                        Err(_) => todo!(), // not sure how to handle this - either error out or no-op
                    };

                    Ok(ShortCircuit::Known(res))
                }
            },
            Predicate::Content(x) => Ok(ShortCircuit::Unknown(Predicate::Content(x))),
            Predicate::Name(x) => Ok(ShortCircuit::Unknown(Predicate::Name(x))),
            Predicate::Metadata(x) => Ok(ShortCircuit::Unknown(Predicate::Metadata(x))),
        }
    }
}
