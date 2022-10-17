use regex::Regex;
use std::{fmt::Display, fs::Metadata, ops::Range, path::Path};
use std::{os::unix::prelude::MetadataExt, os::unix::prelude::PermissionsExt};

#[derive(Debug)]
pub enum NamePredicate {
    Regex(Regex),
    Extension(String),
}

impl NamePredicate {
    pub fn is_match(&self, path: &Path) -> bool {
        match path.to_str() {
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

#[derive(Debug, PartialEq, Eq)]
pub enum MetadataPredicate {
    Filesize(Range<u64>),
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
                is_executable && !metadata.is_dir() && metadata.size() > 0
            }
            MetadataPredicate::Dir() => metadata.is_dir(),
        }
    }
}

impl Display for MetadataPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetadataPredicate::Filesize(fs) => write!(f, "size({:?})", fs),
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
