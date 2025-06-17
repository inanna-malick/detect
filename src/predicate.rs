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
use chrono::{DateTime, Local, NaiveDate, Duration};

fn glob_to_regex(glob: &str) -> String {
    let mut regex = String::from("^");
    let mut chars = glob.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    regex.push_str(".*");
                } else {
                    regex.push_str("[^/]*");
                }
            }
            '?' => regex.push('.'),
            '[' => {
                regex.push('[');
                while let Some(ch) = chars.next() {
                    regex.push(ch);
                    if ch == ']' {
                        break;
                    }
                }
            }
            '.' | '(' | ')' | '+' | '|' | '^' | '$' | '@' | '%' => {
                regex.push('\\');
                regex.push(ch);
            }
            _ => regex.push(ch),
        }
    }
    
    regex.push('$');
    regex
}

fn parse_time_value(s: &str) -> anyhow::Result<DateTime<Local>> {
    // Handle relative time formats
    if s.starts_with('-') {
        let parts: Vec<&str> = s[1..].split('.').collect();
        if parts.len() == 2 {
            let number: i64 = parts[0].parse()?;
            let unit = parts[1];
            
            let duration = match unit {
                "seconds" | "second" | "secs" | "sec" | "s" => Duration::seconds(number),
                "minutes" | "minute" | "mins" | "min" | "m" => Duration::minutes(number),
                "hours" | "hour" | "hrs" | "hr" | "h" => Duration::hours(number),
                "days" | "day" | "d" => Duration::days(number),
                "weeks" | "week" | "w" => Duration::weeks(number),
                _ => anyhow::bail!("Unknown time unit: {}", unit),
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
    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap());
    }
    
    // Try parsing as ISO datetime
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Local));
    }
    
    anyhow::bail!("Cannot parse time value: {}", s)
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
    Glob(String),
    In(Vec<String>),
}

impl PartialEq for StringMatcher {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Regex(l0), Self::Regex(r0)) => l0.as_str() == r0.as_str(),
            (Self::Equals(l0), Self::Equals(r0)) => l0 == r0,
            (Self::NotEquals(l0), Self::NotEquals(r0)) => l0 == r0,
            (Self::Contains(l0), Self::Contains(r0)) => l0 == r0,
            (Self::Glob(l0), Self::Glob(r0)) => l0 == r0,
            (Self::In(l0), Self::In(r0)) => l0 == r0,
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
            StringMatcher::NotEquals(cmp) => cmp != s,
            StringMatcher::Contains(substr) => s.contains(substr),
            StringMatcher::Glob(pattern) => {
                // Convert glob pattern to regex
                let regex_pattern = glob_to_regex(pattern);
                Regex::new(&regex_pattern).map(|r| r.is_match(s)).unwrap_or(false)
            }
            StringMatcher::In(values) => values.contains(&s.to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NumberMatcher {
    In(Bound),
    Equals(u64),
}

#[derive(Clone, Debug)]
pub enum TimeMatcher {
    Before(DateTime<Local>),
    After(DateTime<Local>),
    Equals(DateTime<Local>),
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
                file_datetime.date_naive() == dt.date_naive()
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
    Glob,     // 'glob' - glob pattern
}

pub fn parse_string(op: &Op, rhs: &str) -> anyhow::Result<StringMatcher> {
    Ok(match op {
        Op::Matches => StringMatcher::Regex(Regex::new(rhs)?),
        Op::Equality => StringMatcher::Equals(rhs.to_owned()),
        Op::NotEqual => StringMatcher::NotEquals(rhs.to_owned()),
        Op::Contains => StringMatcher::Contains(rhs.to_owned()),
        Op::Glob => StringMatcher::Glob(rhs.to_owned()),
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
        x => anyhow::bail!("operator {:?} cannot be applied to string values", x),
    })
}

pub fn parse_string_dfa(op: Op, rhs: String) -> anyhow::Result<StreamingCompiledContentPredicate> {
    Ok(match op {
        Op::Matches => StreamingCompiledContentPredicate::new(rhs)?,
        Op::Equality => {
            let regex = format!("^{}$", regex::escape(&rhs));
            StreamingCompiledContentPredicate {
                inner: DFA::new(&regex)?,
                source: regex,
            }
        }
        Op::Contains => {
            let regex = regex::escape(&rhs);
            StreamingCompiledContentPredicate {
                inner: DFA::new(&regex)?,
                source: regex,
            }
        }
        Op::Glob => {
            let regex = glob_to_regex(&rhs);
            StreamingCompiledContentPredicate {
                inner: DFA::new(&regex)?,
                source: regex,
            }
        }
        x => anyhow::bail!("operator {:?} cannot be applied to contents", x),
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

pub fn parse_temporal(op: &Op, rhs: &str) -> anyhow::Result<TimeMatcher> {
    let parsed_time = parse_time_value(rhs)?;
    
    match op {
        Op::Equality => Ok(TimeMatcher::Equals(parsed_time)),
        Op::NumericComparison(op) => Ok(match op {
            NumericalOp::Greater | NumericalOp::GreaterOrEqual => TimeMatcher::After(parsed_time),
            NumericalOp::Less | NumericalOp::LessOrEqual => TimeMatcher::Before(parsed_time),
        }),
        x => anyhow::bail!("operator {:?} cannot be applied to temporal values", x),
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
            NamePredicate::Filename(x) => path
                .file_name()
                .and_then(|os_str| os_str.to_str())
                .is_some_and(|s| x.is_match(s)),
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
