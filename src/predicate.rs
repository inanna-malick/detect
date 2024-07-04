use regex::Regex;
use std::ops::{RangeFrom, RangeTo};
use std::sync::Arc;
use std::{fmt::Display, fs::Metadata, ops::Range, path::Path};
use std::{os::unix::prelude::MetadataExt, os::unix::prelude::PermissionsExt};

use crate::expr::short_circuit::ShortCircuit;
use crate::util::Done;

#[derive(Debug, PartialEq, Eq)]
pub enum Predicate<Name, Metadata, Content> {
    Name(Arc<Name>),
    Metadata(Arc<Metadata>),
    Content(Arc<Content>),
}

impl<A, B, C> Clone for Predicate<A, B, C> {
    fn clone(&self) -> Self {
        match self {
            Self::Name(arg0) => Self::Name(arg0.clone()),
            Self::Metadata(arg0) => Self::Metadata(arg0.clone()),
            Self::Content(arg0) => Self::Content(arg0.clone()),
        }
    }
}

impl<A: Display, B: Display, C: Display> Display for Predicate<A, B, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Predicate::Name(x) => write!(f, "{}", x),
            Predicate::Metadata(x) => write!(f, "{}", x),
            Predicate::Content(x) => write!(f, "{}", x),
        }
    }
}

impl<A, B, C> Predicate<NamePredicate, A, B> {
    pub fn eval_name_predicate(self, path: &Path) -> ShortCircuit<Predicate<Done, A, B>> {
        match self {
            Predicate::Name(p) => ShortCircuit::Known(p.is_match(path)),
            Predicate::Metadata(x) => ShortCircuit::Unknown(Predicate::Metadata(x)),
            Predicate::Content(x) => ShortCircuit::Unknown(Predicate::Content(x)),
        }
    }
}

impl<A, B, C> Predicate<A, MetadataPredicate, B> {
    pub fn eval_metadata_predicate(
        self,
        metadata: &Metadata,
    ) -> ShortCircuit<Predicate<A, Done, B>> {
        match self {
            Predicate::Metadata(p) => ShortCircuit::Known(p.is_match(metadata)),
            Predicate::Content(x) => ShortCircuit::Unknown(Predicate::Content(x)),
            Predicate::Name(x) => ShortCircuit::Unknown(Predicate::Name(x)),
        }
    }
}

impl<A, B, C> Predicate<A, B, ContentPredicate> {
    pub fn eval_file_content_predicate(
        self,
        contents: Option<&String>,
    ) -> ShortCircuit<Predicate<A, B, Done>> {
        match self {
            Predicate::Content(p) => ShortCircuit::Known(match contents {
                Some(contents) => p.is_match(contents),
                None => false,
            }),
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

// only used for tests
impl PartialEq for NamePredicate {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Filename(l0), Self::Filename(r0)) => l0.as_str() == r0.as_str(),
            (Self::Path(l0), Self::Path(r0)) => l0.as_str() == r0.as_str(),
            (Self::Extension(l0), Self::Extension(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl Eq for NamePredicate {}

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

// predicates that scan the entire file
#[derive(Debug)]
pub enum ContentPredicate {
    Regex(Regex),
    Utf8,
}

// only used for tests
impl PartialEq for ContentPredicate {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Regex(l0), Self::Regex(r0)) => l0.as_str() == r0.as_str(),
            (Self::Utf8, Self::Utf8) => true,
            _ => false,
        }
    }
}

impl Eq for ContentPredicate {}

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
