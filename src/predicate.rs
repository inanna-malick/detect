use regex::Regex;
use std::ops::{RangeFrom, RangeTo};
use std::{fmt::Display, fs::Metadata, ops::Range, path::Path};
use std::{os::unix::prelude::MetadataExt, os::unix::prelude::PermissionsExt};

#[derive(Debug)]
pub enum NamePredicate {
    Regex(Regex),
    Extension(String),
}

impl PartialEq for NamePredicate {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Regex(l0), Self::Regex(r0)) => l0.as_str() == r0.as_str(),
            (Self::Extension(l0), Self::Extension(r0)) => l0.as_str() == r0.as_str(),
            _ => false,
        }
    }
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
#[derive(Debug, PartialEq)]
pub enum Bound {
    Full(Range<u64>),
    Left(RangeFrom<u64>),
    Right(RangeTo<u64>),
}

fn display_kb_mb(u: u64) -> String {
    let kb = 1024;
    let mb = kb * 1024;
    if u >= mb {
        format!("{}mb", u / mb)
    } else if u >= kb {
        format!("{}kb", u / kb)
    } else {
        format!("{}", u)
    }
}

impl Display for Bound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Bound::Full(r) => write!(f, "{}..{}", display_kb_mb(r.start), display_kb_mb(r.end)),
            Bound::Left(r) => write!(f, "{}..", display_kb_mb(r.start)),
            Bound::Right(r) => write!(f, "..{}", display_kb_mb(r.end)),
        }
    }
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

#[derive(Debug, PartialEq)]
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
            MetadataPredicate::Filesize(fs) => write!(f, "size({})", fs),
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

impl PartialEq for ContentPredicate {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Regex(l0), Self::Regex(r0)) => l0.as_str() == r0.as_str(),
            (Self::Utf8, Self::Utf8) => true,
            _ => false,
        }
    }
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
