use regex::Regex;
use std::ops::{RangeFrom, RangeTo};
use std::process::Stdio;
use std::sync::Arc;
use std::{fmt::Display, fs::Metadata, ops::Range, path::Path};
use std::{os::unix::prelude::MetadataExt, os::unix::prelude::PermissionsExt};

use crate::expr::short_circuit::ShortCircuit;
use crate::util::Done;

#[derive(Debug)]
pub enum Predicate<Name, Metadata, Content, Async> {
    Name(Arc<Name>),
    Metadata(Arc<Metadata>),
    Content(Arc<Content>),
    Async(Arc<Async>), // TODO: better name?
}

impl<A, B, C, D> Clone for Predicate<A, B, C, D> {
    fn clone(&self) -> Self {
        match self {
            Self::Name(arg0) => Self::Name(arg0.clone()),
            Self::Metadata(arg0) => Self::Metadata(arg0.clone()),
            Self::Content(arg0) => Self::Content(arg0.clone()),
            Self::Async(arg0) => Self::Async(arg0.clone()),
        }
    }
}

impl<A: Display, B: Display, C: Display, D: Display> Display for Predicate<A, B, C, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Predicate::Name(x) => write!(f, "{}", x),
            Predicate::Metadata(x) => write!(f, "{}", x),
            Predicate::Content(x) => write!(f, "{}", x),
            Predicate::Async(x) => write!(f, "{}", x),
        }
    }
}

impl<A, B, C> Predicate<NamePredicate, A, B, C> {
    pub fn eval_name_predicate(self, path: &Path) -> ShortCircuit<Predicate<Done, A, B, C>> {
        match self {
            Predicate::Name(p) => {
                // println!("")
                ShortCircuit::Known(p.is_match(path))
            }
            Predicate::Metadata(x) => ShortCircuit::Unknown(Predicate::Metadata(x)),
            Predicate::Content(x) => ShortCircuit::Unknown(Predicate::Content(x)),
            Predicate::Async(x) => ShortCircuit::Unknown(Predicate::Async(x)),
        }
    }
}

impl<A, B, C> Predicate<A, MetadataPredicate, B, C> {
    pub fn eval_metadata_predicate(
        self,
        metadata: &Metadata,
    ) -> ShortCircuit<Predicate<A, Done, B, C>> {
        match self {
            Predicate::Metadata(p) => ShortCircuit::Known(p.is_match(metadata)),
            Predicate::Content(x) => ShortCircuit::Unknown(Predicate::Content(x)),
            Predicate::Async(x) => ShortCircuit::Unknown(Predicate::Async(x)),
            Predicate::Name(x) => ShortCircuit::Unknown(Predicate::Name(x)),
        }
    }
}

impl<A, B, C> Predicate<A, B, ContentPredicate, C> {
    pub fn eval_file_content_predicate(
        self,
        contents: Option<&String>,
    ) -> ShortCircuit<Predicate<A, B, Done, C>> {
        match self {
            Predicate::Content(p) => ShortCircuit::Known(match contents {
                Some(contents) => p.is_match(contents),
                None => false,
            }),
            Predicate::Async(x) => ShortCircuit::Unknown(Predicate::Async(x)),
            Predicate::Name(x) => ShortCircuit::Unknown(Predicate::Name(x)),
            Predicate::Metadata(x) => ShortCircuit::Unknown(Predicate::Metadata(x)),
        }
    }
}

#[derive(Debug)]
pub enum NamePredicate {
    Filename(Regex),
    Path(Regex),
    Extension(String),
}

impl NamePredicate {
    pub fn is_match(&self, path: &Path) -> bool {
        match self {
            NamePredicate::Filename(regex) => path
                .file_name()
                .and_then(|os_str| os_str.to_str())
                .map_or(false, |s| regex.is_match(s)),
            NamePredicate::Path(regex) => path
                .as_os_str()
                .to_str()
                .map_or(false, |s| regex.is_match(s)),
            NamePredicate::Extension(e) => path
                .file_name()
                .and_then(|os_str| os_str.to_str())
                .map_or(false, |s| s.ends_with(e)),
        }
    }
}

impl Display for NamePredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NamePredicate::Filename(r) => write!(f, "filename({})", r.as_str()),
            NamePredicate::Extension(x) => write!(f, "extension({})", x.as_str()),
            NamePredicate::Path(r) => write!(f, "filepath({})", r.as_str()),
        }
    }
}

/// Enum over range types, allows for x1..x2, ..x2, x1..
#[derive(Debug, PartialEq, Eq)]
pub enum Bound {
    Full(Range<u64>),
    Left(RangeFrom<u64>),
    Right(RangeTo<u64>),
}

impl Bound {
    fn contains(&self, t: &u64) -> bool {
        match self {
            Bound::Full(b) => b.contains(t),
            Bound::Left(b) => b.contains(t),
            Bound::Right(b) => b.contains(t),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MetadataPredicate {
    Filesize(Bound),
    Executable(),
    Dir(),
}

impl MetadataPredicate {
    pub fn is_match(&self, metadata: &Metadata) -> bool {
        match self {
            MetadataPredicate::Filesize(range) => range.contains(&metadata.size()),
            MetadataPredicate::Executable() => {
                let permissions = metadata.permissions();
                let is_executable = permissions.mode() & 0o111 != 0;
                is_executable && !metadata.is_dir()
            }
            MetadataPredicate::Dir() => metadata.is_dir(),
        }
    }
}

impl Display for MetadataPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetadataPredicate::Filesize(fs) => match fs {
                Bound::Full(r) => write!(f, "size({}..{})", r.start, r.end),
                Bound::Left(r) => write!(f, "size({}..)", r.start),
                Bound::Right(r) => write!(f, "size(..{})", r.end),
            },
            MetadataPredicate::Executable() => write!(f, "executable()"),
            MetadataPredicate::Dir() => write!(f, "dir()"),
        }
    }
}

// predicates based on, eg, scanning file contents or trying to parse exif data go here
#[derive(Debug)]
pub enum ContentPredicate {
    Regex(Regex),
    Utf8,
}

impl ContentPredicate {
    pub fn is_match(&self, utf8_contents: &str) -> bool {
        match self {
            ContentPredicate::Regex(regex) => regex.is_match(utf8_contents),
            ContentPredicate::Utf8 => true,
        }
    }
}

impl Display for ContentPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentPredicate::Regex(r) => write!(f, "contains({})", r.as_str()),
            ContentPredicate::Utf8 => write!(f, "utf8()"),
        }
    }
}

// run cmd with file path as an argument
#[derive(Debug)]
pub enum ProcessPredicate {
    Process { cmd: String, expected_stdout: Regex },
}

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
    ) -> std::io::Result<ShortCircuit<Predicate<A, B, C, Done>>> {
        match self {
            Predicate::Async(x) => match x.as_ref() {
                ProcessPredicate::Process {
                    cmd,
                    expected_stdout: expected,
                } => {
                    use tokio::process::*;

                    // TODO: propagate ctrl-c and etc to child processes
                    let mut child = Command::new(cmd)
                        .arg(file_path)
                        .stdin(Stdio::null())
                        .stderr(Stdio::piped())
                        .stdout(Stdio::piped())
                        .spawn()?;

                    let processid = child.id();

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
