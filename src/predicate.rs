use regex::Regex;
use std::fs::FileType;
use std::ops::{RangeFrom, RangeTo};
use std::sync::Arc;
use std::{fmt::Display, fs::Metadata, ops::Range, path::Path};
use std::{os::unix::prelude::MetadataExt, os::unix::prelude::PermissionsExt};

use crate::expr::short_circuit::ShortCircuit;
use crate::parse;
use crate::util::Done;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawPredicate {
    pub lhs: Selector,
    pub op: Op,
    pub rhs: String,
}

impl RawPredicate {
    pub fn parse(
        &self,
    ) -> anyhow::Result<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>> {
        Ok(match self.lhs {
            Selector::Name => {
                Predicate::name(NamePredicate::Filename(parse_string(&self.op, &self.rhs)?))
            }
            Selector::FilePath => {
                Predicate::name(NamePredicate::Path(parse_string(&self.op, &self.rhs)?))
            }
            Selector::Extension => {
                Predicate::name(NamePredicate::Extension(parse_string(&self.op, &self.rhs)?))
            }
            Selector::EntityType => {
                Predicate::meta(MetadataPredicate::Type(parse_string(&self.op, &self.rhs)?))
            }
            Selector::Size => Predicate::meta(MetadataPredicate::Filesize(parse_numerical(
                &self.op, &self.rhs,
            )?)),
            Selector::Contents => {
                Predicate::contents(ContentPredicate::Contents(parse_string(&self.op, &self.rhs)?))
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selector {
    // NAME:
    Name,
    Extension,
    FilePath,
    // METADATA
    EntityType,
    Size,
    // TODO: more
    // CONTENTS
    Contents,
    // Encoding, TODO, eventually?
    // later - parse contents as json,toml,etc - can run selectors against that
}

#[derive(Clone, Debug)]
pub enum StringMatcher {
    Regex(Regex),
    Equals(String),
    Contains(String),
}

impl PartialEq for StringMatcher {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Regex(l0), Self::Regex(r0)) => l0.as_str() == r0.as_str(),
            (Self::Equals(l0), Self::Equals(r0)) => l0 == r0,
            (Self::Contains(l0), Self::Contains(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl Eq for StringMatcher {}

impl StringMatcher {
    pub fn is_match(&self, s: &str) -> bool {
        match self {
            StringMatcher::Regex(r) => r.is_match(s),
            StringMatcher::Equals(cmp) => cmp == s,
            StringMatcher::Contains(c) => s.contains(c),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NumberMatcher {
    In(Bound),
    Equals(u64),
}

impl NumberMatcher {
    pub fn is_match(&self, x: u64) -> bool {
        match self {
            NumberMatcher::In(b) => b.contains(&x),
            NumberMatcher::Equals(cmp) => x == *cmp,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    // TODO: choose operator for contains
    Contains, // 'contains'
    Matches,  // '~=', 'matches'
    Equality, // '==', '=', 'is'
    NumericComparison(NumericaComparisonOp),
}

pub fn parse_string(op: &Op, rhs: &str) -> anyhow::Result<StringMatcher> {
    Ok(match op {
        Op::Contains => StringMatcher::Contains(rhs.to_owned()),
        Op::Matches => StringMatcher::Regex(Regex::new(rhs)?),
        Op::Equality => StringMatcher::Equals(rhs.to_owned()),
        x => anyhow::bail!("operator {:?} cannot be applied to string values", x),
    })
}

pub fn parse_numerical(op: &Op, rhs: &str) -> anyhow::Result<NumberMatcher> {
    let parsed_rhs: u64 = rhs.parse()?;

    match op {
        Op::Equality => Ok(NumberMatcher::Equals(parsed_rhs)),
        Op::NumericComparison(op) => Ok(NumberMatcher::In(match op {
            NumericaComparisonOp::Greater => Bound::Left(parsed_rhs..),
            NumericaComparisonOp::GreaterOrEqual => Bound::Left(parsed_rhs.saturating_sub(1)..),
            NumericaComparisonOp::LessOrEqual => Bound::Right(..parsed_rhs),
            NumericaComparisonOp::Less => Bound::Right(..parsed_rhs.saturating_add(1)),
        })),
        x => anyhow::bail!("operator {:?} cannot be applied to numerical values", x),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NumericaComparisonOp {
    Greater,        // '>'
    GreaterOrEqual, // >=
    LessOrEqual,    // <=
    Less,           // <
}

#[derive(Debug, PartialEq, Eq)]
pub enum Predicate<Name, Metadata, Content> {
    Name(Arc<Name>),
    Metadata(Arc<Metadata>),
    Content(Arc<Content>),
}

impl<N, M, C> Predicate<N, M, C> {
    pub fn name(n: N) -> Self {
        Self::Name(Arc::new(n))
    }
    pub fn meta(m: M) -> Self {
        Self::Metadata(Arc::new(m))
    }
    pub fn contents(c: C) -> Self {
        Self::Content(Arc::new(c))
    }
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

impl<A, B> Predicate<NamePredicate, A, B> {
    pub fn eval_name_predicate(self, path: &Path) -> ShortCircuit<Predicate<Done, A, B>> {
        match self {
            Predicate::Name(p) => ShortCircuit::Known(p.is_match(path)),
            Predicate::Metadata(x) => ShortCircuit::Unknown(Predicate::Metadata(x)),
            Predicate::Content(x) => ShortCircuit::Unknown(Predicate::Content(x)),
        }
    }
}

impl<A, B> Predicate<A, MetadataPredicate, B> {
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

impl<A, B> Predicate<A, B, ContentPredicate> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamePredicate {
    Filename(StringMatcher),
    Path(StringMatcher),
    Extension(StringMatcher),
}

impl NamePredicate {
    pub fn is_match(&self, path: &Path) -> bool {
        match self {
            NamePredicate::Filename(x) => path
                .file_name()
                .and_then(|os_str| os_str.to_str())
                .map_or(false, |s| x.is_match(s)),
            NamePredicate::Path(x) => path.as_os_str().to_str().map_or(false, |s| x.is_match(s)),
            NamePredicate::Extension(x) => path
                .extension()
                .and_then(|os_str| os_str.to_str())
                .map_or(false, |s| x.is_match(s)),
        }
    }
}

/// Enum over range types, allows for x1..x2, ..x2, x1..
#[derive(Clone, Debug, PartialEq, Eq)]
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
    Filesize(NumberMatcher),
    Type(StringMatcher), //dir, exec, etc
}

impl MetadataPredicate {
    pub fn is_match(&self, metadata: &Metadata) -> bool {
        match self {
            MetadataPredicate::Filesize(range) => range.is_match(metadata.size()),
            MetadataPredicate::Type(matcher) => {
                use std::os::unix::fs::FileTypeExt;
                let ft: FileType = metadata.file_type();
                if ft.is_socket() {
                    matcher.is_match("sock") || matcher.is_match("socket")
                } else if ft.is_dir() {
                    matcher.is_match("dir") || matcher.is_match("directory")
                } else if ft.is_file() {
                    matcher.is_match("file")
                } else {
                    false
                }
            }
        }
    }
}

// predicates that scan the entire file
#[derive(Debug, Eq, PartialEq)]
pub enum ContentPredicate {
    Contents(StringMatcher),
    Utf8,
}

impl ContentPredicate {
    pub fn is_match(&self, utf8_contents: &str) -> bool {
        match self {
            ContentPredicate::Contents(regex) => regex.is_match(utf8_contents),
            ContentPredicate::Utf8 => true,
        }
    }
}
