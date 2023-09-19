use regex::Regex;
use std::ops::{RangeFrom, RangeTo};
use std::{fmt::Display, fs::Metadata, ops::Range, path::Path};
use std::{os::unix::prelude::MetadataExt, os::unix::prelude::PermissionsExt};

use crate::expr::short_circuit::ShortCircuit;
use crate::expr::Expr;
use crate::util::Done;

pub enum Predicate<Name = NamePredicate, Metadata = MetadataPredicate, Content = ContentPredicate> {
    Name(Name),
    Metadata(Metadata),
    Content(Content),
}

impl<A: Display, B: Display, C:Display> Display for Predicate<A,B,C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Predicate::Name(x) => write!(f, "{}", x),
            Predicate::Metadata(x) => write!(f, "{}", x),
            Predicate::Content(x) => write!(f, "{}", x),
        }
    }
}

impl Predicate<NamePredicate, MetadataPredicate, ContentPredicate> {
    pub fn eval_name_predicate(
        &self,
        path: &Path,
    ) -> ShortCircuit<Expr<Predicate<Done, &MetadataPredicate, &ContentPredicate>>> {
        match self {
            Predicate::Name(p) => ShortCircuit::Known(p.is_match(path)),
            Predicate::Metadata(x) => {
                ShortCircuit::Unknown(Expr::Predicate(Predicate::Metadata(x)))
            }
            Predicate::Content(x) => ShortCircuit::Unknown(Expr::Predicate(Predicate::Content(x))),
        }
    }
}

impl Predicate<Done, &MetadataPredicate, &ContentPredicate> {
    pub fn eval_metadata_predicate(
        &self,
        metadata: &Metadata,
    ) -> ShortCircuit<Expr<Predicate<Done, Done, &ContentPredicate>>> {
        match self {
            Predicate::Metadata(p) => ShortCircuit::Known(p.is_match(metadata)),
            Predicate::Content(x) => ShortCircuit::Unknown(Expr::Predicate(Predicate::Content(x))),
            _ => unreachable!(),
        }
    }
}

impl Predicate<Done, Done, &ContentPredicate> {
    pub fn eval_file_content_predicate(&self, contents: Option<&str>) -> bool {
        match self {
            Predicate::Content(p) => match contents {
                Some(contents) => p.is_match(contents),
                None => false,
            },
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub enum NamePredicate {
    Regex(Regex),
    Extension(String),
}

impl NamePredicate {
    pub fn is_match(&self, path: &Path) -> bool {
        match path.file_name().and_then(|os_str| os_str.to_str()) {
            Some(s) => match self {
                NamePredicate::Regex(r) => r.is_match(s),
                NamePredicate::Extension(x) => s.ends_with(x),
            },
            None => false,
        }
    }
}

impl Display for NamePredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NamePredicate::Regex(r) => write!(f, "filename({})", r.as_str()),
            NamePredicate::Extension(x) => write!(f, "extension({})", x.as_str()),
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
