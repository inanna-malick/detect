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
use crate::parse_error::{PredicateParseError, TemporalError, TemporalErrorKind};
use chrono::{DateTime, Local, NaiveDate, Duration};


fn parse_time_value(s: &str) -> Result<DateTime<Local>, TemporalError> {
    // Handle relative time formats
    if s.starts_with('-') {
        let parts: Vec<&str> = s[1..].split('.').collect();
        if parts.len() == 2 {
            let number: i64 = parts[0].parse().map_err(|e| TemporalError {
                input: s.to_string(),
                kind: TemporalErrorKind::ParseInt(e),
            })?;
            let unit = parts[1];
            
            let duration = match unit {
                "seconds" | "second" | "secs" | "sec" | "s" => Duration::seconds(number),
                "minutes" | "minute" | "mins" | "min" | "m" => Duration::minutes(number),
                "hours" | "hour" | "hrs" | "hr" | "h" => Duration::hours(number),
                "days" | "day" | "d" => Duration::days(number),
                "weeks" | "week" | "w" => Duration::weeks(number),
                _ => return Err(TemporalError {
                    input: s.to_string(),
                    kind: TemporalErrorKind::UnknownUnit(unit.to_string()),
                }.into()),
            };
            
            return Ok(Local::now() - duration);
        }
    }
    
    // Handle special keywords
    match s {
        "now" => return Ok(Local::now()),
        "today" => {
            let today = Local::now().date_naive();
            return Ok(today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap());
        }
        "yesterday" => {
            let yesterday = Local::now().date_naive() - Duration::days(1);
            return Ok(yesterday.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap());
        }
        _ => {}
    }
    
    // Try parsing as absolute date/datetime
    match NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        Ok(date) => return Ok(date.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap()),
        Err(_) => {
            // Try other formats before failing
        }
    }
    
    // Try parsing as ISO datetime
    match DateTime::parse_from_rfc3339(s) {
        Ok(dt) => Ok(dt.with_timezone(&Local)),
        Err(e) => {
            // If all parsing attempts fail, return the last error
            Err(TemporalError {
                input: s.to_string(),
                kind: TemporalErrorKind::InvalidDate(e),
            })
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawPredicate {
    pub lhs: Selector,
    pub op: Op,
    pub rhs: String,
}

impl RawPredicate {
    pub fn parse(
        self,
    ) -> Result<
        Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>,
        PredicateParseError,
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
            Selector::Modified => Predicate::meta(MetadataPredicate::Modified(parse_temporal(
                &self.op, &self.rhs,
            )?)),
            Selector::Created => Predicate::meta(MetadataPredicate::Created(parse_temporal(
                &self.op, &self.rhs,
            )?)),
            Selector::Accessed => Predicate::meta(MetadataPredicate::Accessed(parse_temporal(
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
    // TEMPORAL
    Modified,
    Created,
    Accessed,
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
    NotEquals(String),
    Contains(String),
    In(Vec<String>),
}

impl PartialEq for StringMatcher {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Regex(l0), Self::Regex(r0)) => l0.as_str() == r0.as_str(),
            (Self::Equals(l0), Self::Equals(r0)) => l0 == r0,
            (Self::NotEquals(l0), Self::NotEquals(r0)) => l0 == r0,
            (Self::Contains(l0), Self::Contains(r0)) => l0 == r0,
            (Self::In(l0), Self::In(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl Eq for StringMatcher {}

impl StringMatcher {
    pub fn regex(s: &str) -> Result<Self, regex::Error> {
        Ok(Self::Regex(Regex::new(s)?))
    }

    pub fn is_match(&self, s: &str) -> bool {
        match self {
            StringMatcher::Regex(r) => r.is_match(s),
            StringMatcher::Equals(cmp) => cmp == s,
            StringMatcher::NotEquals(cmp) => cmp != s,
            StringMatcher::Contains(substr) => s.contains(substr),
            StringMatcher::In(values) => values.contains(&s.to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NumberMatcher {
    In(Bound),
    Equals(u64),
    NotEquals(u64),
}

#[derive(Clone, Debug)]
pub enum TimeMatcher {
    Before(DateTime<Local>),
    After(DateTime<Local>),
    Equals(DateTime<Local>),
    NotEquals(DateTime<Local>),
}

impl TimeMatcher {
    pub fn is_match(&self, timestamp: i64) -> bool {
        use std::time::UNIX_EPOCH;
        
        let file_time = UNIX_EPOCH + std::time::Duration::from_secs(timestamp as u64);
        let file_datetime: DateTime<Local> = file_time.into();
        
        match self {
            TimeMatcher::Before(dt) => file_datetime < *dt,
            TimeMatcher::After(dt) => file_datetime > *dt,
            TimeMatcher::Equals(dt) => {
                // For equality, we'll consider same day
                // FIXME: choose granularity, somewhow
                file_datetime.date_naive() == dt.date_naive()
            }
            TimeMatcher::NotEquals(dt) => {
                // FIXME: choose granularity, somewhow - maybe find lowest value (day/minute/etc that isn't all 0's)
                // For equality, we'll consider same day
                file_datetime.date_naive() != dt.date_naive()
            }
        }
    }
}

impl PartialEq for TimeMatcher {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TimeMatcher::Before(a), TimeMatcher::Before(b)) => a == b,
            (TimeMatcher::After(a), TimeMatcher::After(b)) => a == b,
            (TimeMatcher::Equals(a), TimeMatcher::Equals(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for TimeMatcher {}

impl NumberMatcher {
    pub fn is_match(&self, x: u64) -> bool {
        match self {
            NumberMatcher::In(b) => b.contains(&x),
            NumberMatcher::Equals(cmp) => x == *cmp,
            NumberMatcher::NotEquals(cmp) => x != *cmp,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    Matches,  // '~=', '~', '=~' - regex
    Equality, // '==', '='
    NotEqual, // '!='
    NumericComparison(NumericalOp),
    In,       // 'in' - set membership
    Contains, // 'contains' - substring
}

pub fn parse_string(op: &Op, rhs: &str) -> Result<StringMatcher, PredicateParseError> {
    Ok(match op {
        Op::Matches => {
            // Special case for '*' which users commonly expect to work
            let pattern = if rhs == "*" { ".*" } else { rhs };
            StringMatcher::Regex(Regex::new(pattern)?)
        },
        Op::Equality => StringMatcher::Equals(rhs.to_owned()),
        Op::NotEqual => StringMatcher::NotEquals(rhs.to_owned()),
        Op::Contains => StringMatcher::Contains(rhs.to_owned()),
        Op::In => {
            // Check if rhs is a JSON-encoded array (from set literal)
            if rhs.starts_with('[') && rhs.ends_with(']') {
                match serde_json::from_str::<Vec<String>>(rhs) {
                    Ok(values) => StringMatcher::In(values),
                    Err(_) => StringMatcher::In(vec![rhs.to_owned()]),
                }
            } else {
                StringMatcher::In(vec![rhs.to_owned()])
            }
        }
        Op::NumericComparison(_) => {
            return Err(PredicateParseError::IncompatibleOperation {
                reason: "Numeric comparison operators (>, <, >=, <=) cannot be used with string values",
            })
        }
    })
}

pub fn parse_string_dfa(op: Op, rhs: String) -> Result<StreamingCompiledContentPredicate, PredicateParseError> {
    Ok(match op {
        Op::Matches => StreamingCompiledContentPredicate::new(rhs)?,
        Op::Equality => {
            let regex = format!("^{}$", regex::escape(&rhs));
            match DFA::new(&regex) {
                Ok(inner) => StreamingCompiledContentPredicate { inner, source: regex },
                Err(e) => return Err(PredicateParseError::Dfa(e.to_string())),
            }
        }
        Op::Contains => {
            let regex = regex::escape(&rhs);
            match DFA::new(&regex) {
                Ok(inner) => StreamingCompiledContentPredicate { inner, source: regex },
                Err(e) => return Err(PredicateParseError::Dfa(e.to_string())),
            }
        }
        Op::NotEqual => {
            return Err(PredicateParseError::IncompatibleOperation {
                reason: "!= operator is not supported for contents predicates",
            })
        }
        Op::NumericComparison(_) => {
            return Err(PredicateParseError::IncompatibleOperation {
                reason: "Numeric comparison operators (>, <, >=, <=) cannot be used with contents",
            })
        }
        Op::In => {
            return Err(PredicateParseError::IncompatibleOperation {
                reason: "'in' operator is not supported for contents predicates",
            })
        }
    })
}

pub fn parse_numerical(op: &Op, rhs: &str) -> Result<NumberMatcher, PredicateParseError> {
    let parsed_rhs: u64 = rhs.parse()?;

    match op {
        Op::Equality => Ok(NumberMatcher::Equals(parsed_rhs)),
        Op::NumericComparison(op) => Ok(NumberMatcher::In(match op {
            NumericalOp::Greater => Bound::Left(parsed_rhs..),
            NumericalOp::GreaterOrEqual => Bound::Left(parsed_rhs.saturating_sub(1)..),
            NumericalOp::LessOrEqual => Bound::Right(..parsed_rhs),
            NumericalOp::Less => Bound::Right(..parsed_rhs.saturating_add(1)),
        })),
        Op::NotEqual => Ok(NumberMatcher::NotEquals(parsed_rhs)),
        Op::In => {
            // For now, 'in' with numeric values only supports single values
            // Could be extended to support JSON arrays of numbers
            Ok(NumberMatcher::Equals(parsed_rhs))
        }
        Op::Matches => {
            return Err(PredicateParseError::IncompatibleOperation {
                reason: "Regex operator ~= cannot be used with numeric values",
            })
        }
        Op::Contains => {
            return Err(PredicateParseError::IncompatibleOperation {
                reason: "'contains' operator cannot be used with numeric values",
            })
        }
    }
}

pub fn parse_temporal(op: &Op, rhs: &str) -> Result<TimeMatcher, PredicateParseError> {
    let parsed_time = parse_time_value(rhs)?;
    
    match op {
        Op::Equality => Ok(TimeMatcher::Equals(parsed_time)),
        Op::NotEqual => Ok(TimeMatcher::Equals(parsed_time)),
        Op::NumericComparison(op) => Ok(match op {
            NumericalOp::Greater | NumericalOp::GreaterOrEqual => TimeMatcher::After(parsed_time),
            NumericalOp::Less | NumericalOp::LessOrEqual => TimeMatcher::Before(parsed_time),
        }),
        Op::In => {
            // For now, 'in' with temporal values only supports single values
            // Could be extended to support sets of times
            Ok(TimeMatcher::Equals(parsed_time))
        }
        Op::Matches => {
            return Err(PredicateParseError::IncompatibleOperation {
                reason: "Regex operator ~= cannot be used with temporal values",
            })
        }
        Op::Contains => {
            return Err(PredicateParseError::IncompatibleOperation {
                reason: "'contains' operator cannot be used with temporal values",
            })
        }
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
}

impl NamePredicate {
    pub fn is_match(&self, path: &Path) -> bool {
        match self {
            NamePredicate::Filename(x) => {
                // Check against full filename
                let full_match = path
                    .file_name()
                    .and_then(|os_str| os_str.to_str())
                    .is_some_and(|s| x.is_match(s));
                
                // Also check against filename without extension (stem)
                let stem_match = path
                    .file_stem()
                    .and_then(|os_str| os_str.to_str())
                    .is_some_and(|s| x.is_match(s));
                
                // Return true if either matches
                full_match || stem_match
            },
            NamePredicate::Path(x) => path.as_os_str().to_str().is_some_and(|s| x.is_match(s)),
            NamePredicate::Extension(x) => path
                .extension()
                .and_then(|os_str| os_str.to_str())
                .is_some_and(|s| x.is_match(s)),
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

#[derive(Debug)]
pub enum MetadataPredicate {
    Filesize(NumberMatcher),
    Type(StringMatcher), //dir, exec, etc
    Modified(TimeMatcher),
    Created(TimeMatcher),
    Accessed(TimeMatcher),
}

impl Display for MetadataPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}

impl PartialEq for MetadataPredicate {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MetadataPredicate::Filesize(a), MetadataPredicate::Filesize(b)) => a == b,
            (MetadataPredicate::Type(a), MetadataPredicate::Type(b)) => a == b,
            (MetadataPredicate::Modified(a), MetadataPredicate::Modified(b)) => a == b,
            (MetadataPredicate::Created(a), MetadataPredicate::Created(b)) => a == b,
            (MetadataPredicate::Accessed(a), MetadataPredicate::Accessed(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for MetadataPredicate {}

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
            MetadataPredicate::Modified(matcher) => matcher.is_match(metadata.mtime()),
            MetadataPredicate::Created(matcher) => matcher.is_match(metadata.ctime()),
            MetadataPredicate::Accessed(matcher) => matcher.is_match(metadata.atime()),
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
            MetadataPredicate::Modified(_) | MetadataPredicate::Created(_) | MetadataPredicate::Accessed(_) => {
                // Git trees don't have timestamps
                false
            }
        }
    }

    pub fn is_match_git_blob(&self, entry: &Blob) -> bool {
        match self {
            MetadataPredicate::Filesize(range) => range.is_match(entry.size() as u64),
            MetadataPredicate::Type(matcher) => matcher.is_match("file"),
            MetadataPredicate::Modified(_) | MetadataPredicate::Created(_) | MetadataPredicate::Accessed(_) => {
                // Git blobs don't have timestamps
                false
            }
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
    pub fn new(source: String) -> Result<Self, PredicateParseError> {
        match DFA::new(&source) {
            Ok(inner) => Ok(Self { inner, source }),
            Err(e) => Err(PredicateParseError::Dfa(e.to_string())),
        }
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
        f.write_str(&format!("contents ~= {}", self.source))
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
        f.write_str(&format!("contents ~= {}", self.source))
    }
}
