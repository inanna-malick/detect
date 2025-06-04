use git2::Blob;
use regex::Regex;
use regex_automata::dfa::dense::DFA;
use std::fs::FileType;
use std::ops::{RangeFrom, RangeTo};
use std::os::unix::prelude::MetadataExt;
use std::sync::Arc;
use std::{fmt::Display, fs::Metadata, ops::Range, path::Path};

use crate::expr::short_circuit::ShortCircuit;
use crate::util::Done;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawPredicate {
    pub lhs: Selector,
    pub op: Op,
    pub rhs: String,
}

impl RawPredicate {
    pub fn parse(
        self,
    ) -> anyhow::Result<
        Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>,
    > {
        Ok(match self.lhs {
            Selector::FileName => {
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
            Selector::Contents => Predicate::contents(parse_string_dfa(self.op, self.rhs)?),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selector {
    // NAME:
    FileName,
    Extension,
    FilePath,
    // METADATA
    EntityType,
    Size,
    // TODO: more
    // CONTENTS
    // FIXME: figure out a better Display impl if/when more selectors added
    Contents,
    // Encoding, TODO, eventually?
    // later - parse contents as json,toml,etc - can run selectors against that
}

pub type CompiledMatcher<'a> = DFA<&'a [u32]>;

#[derive(Clone, Debug)]
pub enum StringMatcher {
    Regex(Regex),
    Equals(String),
}

impl PartialEq for StringMatcher {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Regex(l0), Self::Regex(r0)) => l0.as_str() == r0.as_str(),
            (Self::Equals(l0), Self::Equals(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl Eq for StringMatcher {}

impl StringMatcher {
    pub fn regex(s: &str) -> anyhow::Result<Self> {
        Ok(Self::Regex(Regex::new(s)?))
    }

    pub fn is_match(&self, s: &str) -> bool {
        match self {
            StringMatcher::Regex(r) => r.is_match(s),
            StringMatcher::Equals(cmp) => cmp == s,
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
    Matches,  // '~=', 'matches'
    Equality, // '==', '=', 'is'
    NumericComparison(NumericalOp),
}

pub fn parse_string(op: &Op, rhs: &str) -> anyhow::Result<StringMatcher> {
    Ok(match op {
        Op::Matches => StringMatcher::Regex(Regex::new(rhs)?),
        Op::Equality => StringMatcher::Equals(rhs.to_owned()),
        x => anyhow::bail!("operator {:?} cannot be applied to string values", x),
    })
}

pub fn parse_string_dfa(op: Op, rhs: String) -> anyhow::Result<StreamingCompiledContentPredicate> {
    Ok(match op {
        Op::Matches => StreamingCompiledContentPredicate::new(rhs)?,
        Op::Equality => {
            let regex = format!("^{}$", rhs);
            StreamingCompiledContentPredicate {
                inner: DFA::new(&regex)?,
                source: regex,
            }
        }
        x => anyhow::bail!("operator {:?} cannot be applied to string values", x),
    })
}

pub fn parse_numerical(op: &Op, rhs: &str) -> anyhow::Result<NumberMatcher> {
    let parsed_rhs: u64 = rhs.parse()?;

    match op {
        Op::Equality => Ok(NumberMatcher::Equals(parsed_rhs)),
        Op::NumericComparison(op) => Ok(NumberMatcher::In(match op {
            NumericalOp::Greater => Bound::Left(parsed_rhs..),
            NumericalOp::GreaterOrEqual => Bound::Left(parsed_rhs.saturating_sub(1)..),
            NumericalOp::LessOrEqual => Bound::Right(..parsed_rhs),
            NumericalOp::Less => Bound::Right(..parsed_rhs.saturating_add(1)),
        })),
        x => anyhow::bail!("operator {:?} cannot be applied to numerical values", x),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NumericalOp {
    Greater,        // '>'
    GreaterOrEqual, // >=
    LessOrEqual,    // <=
    Less,           // <
}

#[derive(Debug, PartialEq, Eq)]
pub enum Predicate<Name, Metadata, Content> {
    Name(Arc<Name>),
    Metadata(Arc<Metadata>),
    Content(Content),
}

impl<N, M, C> Predicate<N, M, C> {
    pub fn name(n: N) -> Self {
        Self::Name(Arc::new(n))
    }
    pub fn meta(m: M) -> Self {
        Self::Metadata(Arc::new(m))
    }
    pub fn contents(c: C) -> Self {
        Self::Content(c)
    }
}

impl<A, B, C: Clone> Clone for Predicate<A, B, C> {
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
            Predicate::Name(x) => write!(f, "name: {}", x),
            Predicate::Metadata(x) => write!(f, "meta: {}", x),
            Predicate::Content(x) => write!(f, "file: {}", x),
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

    pub fn eval_metadata_predicate_git_tree(self) -> ShortCircuit<Predicate<A, Done, B>> {
        match self {
            Predicate::Metadata(p) => ShortCircuit::Known(p.is_match_git_tree()),
            Predicate::Content(x) => ShortCircuit::Unknown(Predicate::Content(x)),
            Predicate::Name(x) => ShortCircuit::Unknown(Predicate::Name(x)),
        }
    }

    pub fn eval_metadata_predicate_git_blob(
        self,
        blob: &Blob,
    ) -> ShortCircuit<Predicate<A, Done, B>> {
        match self {
            Predicate::Metadata(p) => ShortCircuit::Known(p.is_match_git_blob(blob)),
            Predicate::Content(x) => ShortCircuit::Unknown(Predicate::Content(x)),
            Predicate::Name(x) => ShortCircuit::Unknown(Predicate::Name(x)),
        }
    }
}

// impl<'dfa, A, B> Predicate<A, B, ContentPredicate<'dfa>> {
//     pub fn eval_file_content_predicate(
//         self,
//         contents: Option<&String>,
//     ) -> ShortCircuit<Predicate<A, B, Done>> {
//         match self {
//             Predicate::Content(p) => ShortCircuit::Known(match contents {
//                 Some(contents) => p.is_match(contents),
//                 None => false,
//             }),
//             Predicate::Name(x) => ShortCircuit::Unknown(Predicate::Name(x)),
//             Predicate::Metadata(x) => ShortCircuit::Unknown(Predicate::Metadata(x)),
//         }
//     }
// }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamePredicate {
    Filename(StringMatcher),
    Path(StringMatcher),
    Extension(StringMatcher),
    Equals(String),
    Regex(String),
}

impl NamePredicate {
    pub fn is_match(&self, path: &Path) -> bool {
        match self {
            NamePredicate::Filename(x) => path
                .file_name()
                .and_then(|os_str| os_str.to_str())
                .is_some_and(|s| x.is_match(s)),
            NamePredicate::Path(x) => path.as_os_str().to_str().is_some_and(|s| x.is_match(s)),
            NamePredicate::Extension(x) => path
                .extension()
                .and_then(|os_str| os_str.to_str())
                .is_some_and(|s| x.is_match(s)),
            NamePredicate::Equals(s) => path
                .file_name()
                .and_then(|os_str| os_str.to_str())
                .map(|name| name == s)
                .unwrap_or(false),
            NamePredicate::Regex(pattern) => {
                if let Ok(re) = Regex::new(pattern) {
                    path.file_name()
                        .and_then(|os_str| os_str.to_str())
                        .map(|name| re.is_match(name))
                        .unwrap_or(false)
                } else {
                    false
                }
            }
        }
    }
}

impl Display for NamePredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self))
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
    SizeGreater(u64),
    SizeLess(u64),
    SizeEquals(u64),
    IsExecutable,
    IsSymlink,
}

impl Display for MetadataPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
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
                } else if ft.is_fifo() {
                    matcher.is_match("fifo")
                } else if ft.is_block_device() {
                    matcher.is_match("block")
                } else if ft.is_char_device() {
                    matcher.is_match("char")
                } else if ft.is_dir() {
                    matcher.is_match("dir") || matcher.is_match("directory")
                } else if ft.is_file() {
                    matcher.is_match("file")
                } else {
                    false
                }
            }
            MetadataPredicate::SizeGreater(n) => metadata.size() > *n,
            MetadataPredicate::SizeLess(n) => metadata.size() < *n,
            MetadataPredicate::SizeEquals(n) => metadata.size() == *n,
            MetadataPredicate::IsExecutable => {
                use std::os::unix::fs::PermissionsExt;
                metadata.permissions().mode() & 0o111 != 0
            }
            MetadataPredicate::IsSymlink => metadata.file_type().is_symlink(),
        }
    }

    pub fn is_match_git_tree(&self) -> bool {
        match self {
            MetadataPredicate::Filesize(_) => {
                // it's not a file
                false
            }
            MetadataPredicate::Type(matcher) => {
                matcher.is_match("dir") || matcher.is_match("directory")
            }
            MetadataPredicate::SizeGreater(_) => false,
            MetadataPredicate::SizeLess(_) => false,
            MetadataPredicate::SizeEquals(_) => false,
            MetadataPredicate::IsExecutable => false,
            MetadataPredicate::IsSymlink => false,
        }
    }

    pub fn is_match_git_blob(&self, entry: &Blob) -> bool {
        match self {
            MetadataPredicate::Filesize(range) => range.is_match(entry.size() as u64),
            MetadataPredicate::Type(matcher) => matcher.is_match("file"),
            MetadataPredicate::SizeGreater(n) => entry.size() as u64 > *n,
            MetadataPredicate::SizeLess(n) => (entry.size() as u64) < *n,
            MetadataPredicate::SizeEquals(n) => entry.size() as u64 == *n,
            MetadataPredicate::IsExecutable => false, // Can't determine from blob
            MetadataPredicate::IsSymlink => false, // Can't determine from blob
        }
    }
}


// predicates that scan the entire file
pub struct StreamingCompiledContentPredicate {
    // compiled automaton
    inner: DFA<Vec<u32>>,
    // source regex, for logging
    source: String,
}

impl StreamingCompiledContentPredicate {
    pub fn new(source: String) -> anyhow::Result<Self> {
        Ok(Self {
            inner: DFA::new(&source)?,
            source,
        })
    }

    pub(crate) fn as_ref(&self) -> StreamingCompiledContentPredicateRef<'_> {
        StreamingCompiledContentPredicateRef {
            inner: self.inner.as_ref(),
            source: &self.source,
        }
    }
}

impl PartialEq for StreamingCompiledContentPredicate {
    fn eq(&self, other: &Self) -> bool {
        // compare source regexes only
        self.source == other.source
    }
}

impl std::fmt::Debug for StreamingCompiledContentPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledContentPredicate")
            .field("inner", &"_")
            .field("source", &self.source)
            .finish()
    }
}

impl Display for StreamingCompiledContentPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("@file ~= {}", self.source))
    }
}

// predicates that scan the entire file
#[derive(Clone, Debug)]
pub struct StreamingCompiledContentPredicateRef<'a> {
    // compiled automaton
    pub inner: DFA<&'a [u32]>,
    // source regex, for logging
    pub source: &'a str,
}

impl Display for StreamingCompiledContentPredicateRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("@file ~= {}", self.source))
    }
}
