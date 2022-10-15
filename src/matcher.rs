use regex::Regex;
use std::{fmt::Display, ops::Range};

#[derive(Debug)]
pub enum NameMatcher {
    Regex(Regex),
    Extension(String),
}

impl Display for NameMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameMatcher::Regex(r) => write!(f, "filename({})", r.as_str()),
            NameMatcher::Extension(x) => write!(f, "extension({})", x.as_str()),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MetadataMatcher {
    Filesize(Range<u64>),
}

impl Display for MetadataMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetadataMatcher::Filesize(fs) => write!(f, "size({:?})", fs),
        }
    }
}

// predicates based on, eg, exif data or w/e also go here
// could get silly with it and run, eg, 'strings' (like the cmd) or w/e
// NOTE: could even abstract such that ppl can add their own functionality like this
#[derive(Debug)]
pub enum ContentsMatcher {
    Regex(Regex),
    Utf8,
}

impl Display for ContentsMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentsMatcher::Regex(r) => write!(f, "contains({})", r.as_str()),
            ContentsMatcher::Utf8 => write!(f, "utf8()"),
        }
    }
}
