use regex::Regex;
use std::ops::Range;

#[derive(Debug)]
pub enum NameMatcher {
    Regex(Regex),
}

#[derive(Debug, PartialEq, Eq)]
pub enum MetadataMatcher {
    Filesize(Range<u64>),
}

// predicates based on, eg, exif data or w/e also go here
// could get silly with it and run, eg, 'strings' (like the cmd) or w/e
// NOTE: could even abstract such that ppl can add their own functionality like this
#[derive(Debug)]
pub enum ContentsMatcher {
    Regex(Regex),
    Utf8,
}
