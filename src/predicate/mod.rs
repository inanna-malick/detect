mod enum_matcher;

pub use enum_matcher::{EnumMatcher, EnumPredicate};

use crate::hybrid_regex::{HybridRegex, StreamingHybridRegex};
use regex_automata::dfa::dense::DFA;
use std::collections::HashSet;
use std::fs::FileType;
use std::ops::{RangeFrom, RangeTo};
use std::os::unix::fs::FileTypeExt;
use std::os::unix::prelude::MetadataExt;
use std::sync::Arc;
use std::{
    fmt::{self, Display},
    fs::Metadata,
    ops::Range,
    path::Path,
};

use crate::expr::short_circuit::ShortCircuit;
use crate::predicate_error::PredicateParseError;
use crate::util::Done;
use chrono::{DateTime, Duration, Local, NaiveDate};

/// File type enumeration for type predicates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DetectFileType {
    File,
    Directory,
    Symlink,
    Socket,
    Fifo,
    BlockDevice,
    CharDevice,
}

impl DetectFileType {
    /// Check if a string matches any alias for this file type

    pub fn matches(&self, s: &str) -> bool {
        self.aliases().contains(&s)
    }

    /// Create from `std::fs::FileType`

    pub fn from_fs_type(ft: &FileType) -> Option<Self> {
        match () {
            () if ft.is_file() => Some(Self::File),
            () if ft.is_dir() => Some(Self::Directory),
            () if ft.is_symlink() => Some(Self::Symlink),
            () if ft.is_socket() => Some(Self::Socket),
            () if ft.is_fifo() => Some(Self::Fifo),
            () if ft.is_block_device() => Some(Self::BlockDevice),
            () if ft.is_char_device() => Some(Self::CharDevice),
            () => None,
        }
    }

    /// All enum variants for iteration
    fn all_variants() -> &'static [Self] {
        &[
            Self::File,
            Self::Directory,
            Self::Symlink,
            Self::Socket,
            Self::Fifo,
            Self::BlockDevice,
            Self::CharDevice,
        ]
    }
}

/// Implement `EnumPredicate` trait for parse-time validation
impl EnumPredicate for DetectFileType {
    fn from_str(s: &str) -> Result<Self, String> {
        let s_lower = s.to_lowercase();
        for variant in Self::all_variants() {
            if variant.aliases().contains(&s_lower.as_str()) {
                return Ok(*variant);
            }
        }
        Err(format!(
            "Unknown file type '{}'. Valid types: {}",
            s,
            Self::all_valid_strings().join(", ")
        ))
    }

    fn all_valid_strings() -> &'static [&'static str] {
        &[
            "file",
            "dir",
            "directory",
            "symlink",
            "link",
            "socket",
            "sock",
            "fifo",
            "pipe",
            "block",
            "blockdev",
            "char",
            "chardev",
        ]
    }

    fn as_str(&self) -> &'static str {
        match self {
            DetectFileType::File => "file",
            DetectFileType::Directory => "dir",
            DetectFileType::Symlink => "symlink",
            DetectFileType::Socket => "socket",
            DetectFileType::Fifo => "fifo",
            DetectFileType::BlockDevice => "block",
            DetectFileType::CharDevice => "char",
        }
    }

    fn aliases(&self) -> &'static [&'static str] {
        match self {
            DetectFileType::File => &["file"],
            DetectFileType::Directory => &["dir", "directory"],
            DetectFileType::Symlink => &["symlink", "link"],
            DetectFileType::Socket => &["socket", "sock"],
            DetectFileType::Fifo => &["fifo", "pipe"],
            DetectFileType::BlockDevice => &["block", "blockdev"],
            DetectFileType::CharDevice => &["char", "chardev"],
        }
    }
}

fn parse_duration(
    number: i64,
    unit: &str,
    original: &str,
) -> Result<Duration, PredicateParseError> {
    match unit {
        "seconds" | "second" | "secs" | "sec" | "s" => Ok(Duration::seconds(number)),
        "minutes" | "minute" | "mins" | "min" | "m" => Ok(Duration::minutes(number)),
        "hours" | "hour" | "hrs" | "hr" | "h" => Ok(Duration::hours(number)),
        "days" | "day" | "d" => Ok(Duration::days(number)),
        "weeks" | "week" | "w" => Ok(Duration::weeks(number)),
        _ => Err(PredicateParseError::Temporal(format!(
            "{original}: unknown unit: {unit}"
        ))),
    }
}

pub fn parse_time_value(s: &str) -> Result<DateTime<Local>, PredicateParseError> {
    let (is_negative, stripped) = if let Some(rest) = s.strip_prefix('-') {
        (true, rest)
    } else {
        (false, s)
    };

    if let Some((num_str, unit)) = stripped.split_once('.') {
        if let Ok(number) = num_str.parse::<i64>() {
            let duration = parse_duration(number, unit, s)?;
            return if is_negative {
                Ok(Local::now() - duration)
            } else {
                Ok(Local::now() + duration)
            };
        }
    }

    let digit_end = stripped.find(|c: char| !c.is_ascii_digit());
    if let Some(idx) = digit_end {
        let num_str = &stripped[..idx];
        let unit = &stripped[idx..];

        if !unit.starts_with('-') {
            if let Ok(number) = num_str.parse::<i64>() {
                if !unit.is_empty() {
                    if let Ok(duration) = parse_duration(number, unit, s) {
                        return if is_negative {
                            Ok(Local::now() - duration)
                        } else {
                            Ok(Local::now() + duration)
                        };
                    }
                }
            }
        }
    }

    if let Ok(date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        if let Some(time) = date.and_hms_opt(0, 0, 0) {
            if let chrono::LocalResult::Single(local_time) = time.and_local_timezone(Local) {
                return Ok(local_time);
            }
        }
        return Err(PredicateParseError::Temporal(format!("{s}: invalid date")));
    }

    match DateTime::parse_from_rfc3339(s) {
        Ok(dt) => Ok(dt.with_timezone(&Local)),
        Err(e) => Err(PredicateParseError::Temporal(format!(
            "{s}: invalid date: {e}"
        ))),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RhsValue {
    String(String),

    Number(u64),

    // Size with unit (converted to bytes)
    Size(u64),

    // Set of values (from [item1, item2] syntax)
    Set(Vec<String>),

    // Temporal values
    RelativeTime { value: i64, unit: TimeUnit },
    AbsoluteTime(String),
}

impl Display for RhsValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TimeUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
    Weeks,
}

pub type CompiledMatcher<'a> = DFA<&'a [u32]>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StringMatcher {
    Regex(HybridRegex),
    Equals(String),
    NotEquals(String),
    Contains(String),
    In(HashSet<String>),
}

impl StringMatcher {
    pub fn regex(s: &str) -> Result<Self, regex::Error> {
        Ok(Self::Regex(HybridRegex::new(s)?))
    }

    // Helper constructors for tests and programmatic usage

    pub fn eq(s: &str) -> Self {
        Self::Equals(s.to_string())
    }

    pub fn ne(s: &str) -> Self {
        Self::NotEquals(s.to_string())
    }

    pub fn contains(s: &str) -> Self {
        Self::Contains(s.to_string())
    }

    pub fn in_set<I: IntoIterator<Item = S>, S: AsRef<str>>(items: I) -> Self {
        Self::In(items.into_iter().map(|s| s.as_ref().to_string()).collect())
    }

    pub fn is_match(&self, s: &str) -> bool {
        match self {
            StringMatcher::Regex(r) => r.is_match(s),
            StringMatcher::Equals(cmp) => cmp == s,
            StringMatcher::NotEquals(cmp) => cmp != s,
            StringMatcher::Contains(substr) => s.contains(substr),
            StringMatcher::In(values) => values.contains(s),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NumberMatcher {
    In(Bound),
    Equals(u64),
    NotEquals(u64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimeMatcher {
    Before(DateTime<Local>),
    After(DateTime<Local>),
    BeforeOrEqual(DateTime<Local>),
    AfterOrEqual(DateTime<Local>),
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
            TimeMatcher::BeforeOrEqual(dt) => file_datetime <= *dt,
            TimeMatcher::AfterOrEqual(dt) => file_datetime >= *dt,
            TimeMatcher::Equals(dt) => file_datetime.date_naive() == dt.date_naive(),
            TimeMatcher::NotEquals(dt) => file_datetime.date_naive() != dt.date_naive(),
        }
    }
}

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

impl Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

pub fn parse_string(op: &Op, rhs: &RhsValue) -> Result<StringMatcher, PredicateParseError> {
    match rhs {
        RhsValue::String(s) => {
            Ok(match op {
                Op::Matches => {
                    // treat '*' as match-all, for convenience
                    let pattern = if s == "*" { ".*" } else { s };
                    StringMatcher::regex(pattern)?
                }
                Op::Equality => StringMatcher::Equals(s.clone()),
                Op::NotEqual => StringMatcher::NotEquals(s.clone()),
                Op::Contains => StringMatcher::Contains(s.clone()),
                Op::In => {
                    let mut set = HashSet::new();
                    set.insert(s.clone());
                    StringMatcher::In(set)
                }
                Op::NumericComparison(_) => return Err(PredicateParseError::Incompatible(
                    "Numeric comparison operators (>, <, >=, <=) cannot be used with string values"
                        .to_string(),
                )),
            })
        }
        RhsValue::Set(items) => match op {
            Op::In => Ok(StringMatcher::In(items.clone().into_iter().collect())),
            _ => Err(PredicateParseError::Incompatible(
                "Set values can only be used with 'in' operator".to_string(),
            )),
        },
        _ => Err(PredicateParseError::Incompatible(format!(
            "expected string or set, found {rhs:?}"
        ))),
    }
}

pub fn parse_string_dfa(
    op: Op,
    rhs: RhsValue,
) -> Result<StreamingCompiledContentPredicate, PredicateParseError> {
    let s = match rhs {
        RhsValue::String(s) => s,
        _ => {
            return Err(PredicateParseError::Incompatible(format!(
                "expected string, found {rhs:?}"
            )))
        }
    };

    Ok(match op {
        Op::Matches => StreamingCompiledContentPredicate::new(s)?,
        Op::Equality => {
            let regex = format!("^{}$", regex::escape(&s));
            StreamingCompiledContentPredicate::new(regex)?
        }
        Op::Contains => {
            let regex = regex::escape(&s);
            StreamingCompiledContentPredicate::new(regex)?
        }
        Op::NotEqual => {
            return Err(PredicateParseError::Incompatible(
                "!= operator is not supported for contents predicates".to_string(),
            ))
        }
        Op::NumericComparison(_) => {
            return Err(PredicateParseError::Incompatible(
                "Numeric comparison operators (>, <, >=, <=) cannot be used with contents"
                    .to_string(),
            ))
        }
        Op::In => {
            return Err(PredicateParseError::Incompatible(
                "'in' operator is not supported for contents predicates".to_string(),
            ))
        }
    })
}

pub fn parse_numerical(op: &Op, rhs: &RhsValue) -> Result<NumberMatcher, PredicateParseError> {
    let parsed_rhs: u64 = match rhs {
        RhsValue::Number(n) => *n,
        RhsValue::Size(bytes) => *bytes,
        _ => {
            return Err(PredicateParseError::Incompatible(format!(
                "expected number or size value, found {rhs:?}"
            )))
        }
    };

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
            // Could be extended to support sets of numbers
            Ok(NumberMatcher::Equals(parsed_rhs))
        }
        Op::Matches => Err(PredicateParseError::Incompatible(
            "Regex operator ~= cannot be used with numeric values".to_string(),
        )),
        Op::Contains => Err(PredicateParseError::Incompatible(
            "'contains' operator cannot be used with numeric values".to_string(),
        )),
    }
}

pub fn parse_temporal(op: &Op, rhs: &RhsValue) -> Result<TimeMatcher, PredicateParseError> {
    let s = match rhs {
        RhsValue::String(s) => s,
        _ => {
            return Err(PredicateParseError::Incompatible(format!(
                "expected time value, found {rhs:?}"
            )))
        }
    };
    let parsed_time = parse_time_value(s)?;

    match op {
        Op::Equality => Ok(TimeMatcher::Equals(parsed_time)),
        Op::NotEqual => Ok(TimeMatcher::NotEquals(parsed_time)),
        Op::NumericComparison(op) => Ok(match op {
            NumericalOp::Greater | NumericalOp::GreaterOrEqual => TimeMatcher::After(parsed_time),
            NumericalOp::Less | NumericalOp::LessOrEqual => TimeMatcher::Before(parsed_time),
        }),
        Op::In => Ok(TimeMatcher::Equals(parsed_time)),
        Op::Matches => Err(PredicateParseError::Incompatible(
            "Regex operator ~= cannot be used with temporal values".to_string(),
        )),
        Op::Contains => Err(PredicateParseError::Incompatible(
            "'contains' operator cannot be used with temporal values".to_string(),
        )),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NumericalOp {
    Greater,
    GreaterOrEqual,
    LessOrEqual,
    Less,
}

#[derive(Debug, PartialEq)]
pub enum Predicate<
    Name = NamePredicate,
    Metadata = MetadataPredicate,
    Content = StreamingCompiledContentPredicate,
    Structured = StructuredDataPredicate,
> {
    Name(Arc<Name>),
    Metadata(Arc<Metadata>),
    Content(Content),
    Structured(Structured),
}

impl<N, M, C, S> Predicate<N, M, C, S> {
    pub fn name(n: N) -> Self {
        Self::Name(Arc::new(n))
    }
    pub fn meta(m: M) -> Self {
        Self::Metadata(Arc::new(m))
    }
    pub fn contents(c: C) -> Self {
        Self::Content(c)
    }
    pub fn structured(s: S) -> Self {
        Self::Structured(s)
    }
}

impl<A, B, C: Clone, S: Clone> Clone for Predicate<A, B, C, S> {
    fn clone(&self) -> Self {
        match self {
            Self::Name(arg0) => Self::Name(arg0.clone()),
            Self::Metadata(arg0) => Self::Metadata(arg0.clone()),
            Self::Content(arg0) => Self::Content(arg0.clone()),
            Self::Structured(arg0) => Self::Structured(arg0.clone()),
        }
    }
}

impl<A: Display, B: Display, C: Display, S: std::fmt::Debug> Display for Predicate<A, B, C, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Predicate::Name(x) => write!(f, "{x}"),
            Predicate::Metadata(x) => write!(f, "{x}"),
            Predicate::Content(x) => write!(f, "{x}"),
            Predicate::Structured(x) => write!(f, "{x:?}"),
        }
    }
}

impl<A, B, S> Predicate<NamePredicate, A, B, S> {
    pub fn eval_name_predicate(
        self,
        path: &Path,
        base_path: Option<&Path>,
    ) -> ShortCircuit<Predicate<Done, A, B, S>> {
        match self {
            Predicate::Name(p) => ShortCircuit::Known(p.is_match_with_base(path, base_path)),
            Predicate::Metadata(x) => ShortCircuit::Unknown(Predicate::Metadata(x)),
            Predicate::Content(x) => ShortCircuit::Unknown(Predicate::Content(x)),
            Predicate::Structured(x) => ShortCircuit::Unknown(Predicate::Structured(x)),
        }
    }
}

impl<A, B, S> Predicate<A, MetadataPredicate, B, S> {
    pub fn eval_metadata_predicate(
        self,
        metadata: &Metadata,
    ) -> ShortCircuit<Predicate<A, Done, B, S>> {
        match self {
            Predicate::Metadata(p) => ShortCircuit::Known(p.is_match(metadata)),
            Predicate::Content(x) => ShortCircuit::Unknown(Predicate::Content(x)),
            Predicate::Name(x) => ShortCircuit::Unknown(Predicate::Name(x)),
            Predicate::Structured(x) => ShortCircuit::Unknown(Predicate::Structured(x)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamePredicate {
    BaseName(StringMatcher),  // filename without extension
    FileName(StringMatcher),  // complete filename with extension
    DirPath(StringMatcher),   // directory path only
    FullPath(StringMatcher),  // complete path including filename
    Extension(StringMatcher), // file extension
    ParentDir(StringMatcher), // immediate parent directory name
    Depth(NumberMatcher),     // directory depth from base path
}

impl NamePredicate {
    pub fn file_eq(name: &str) -> Self {
        Self::FileName(StringMatcher::eq(name))
    }

    pub fn stem_eq(name: &str) -> Self {
        Self::BaseName(StringMatcher::eq(name))
    }

    pub fn ext_eq(ext: &str) -> Self {
        Self::Extension(StringMatcher::eq(ext))
    }

    pub fn ext_in<I: IntoIterator<Item = S>, S: AsRef<str>>(exts: I) -> Self {
        Self::Extension(StringMatcher::in_set(exts))
    }

    pub fn path_eq(path: &str) -> Self {
        Self::FullPath(StringMatcher::eq(path))
    }

    pub fn is_match(&self, path: &Path) -> bool {
        self.is_match_with_base(path, None)
    }

    pub fn is_match_with_base(&self, path: &Path, base_path: Option<&Path>) -> bool {
        match self {
            NamePredicate::BaseName(x) => {
                // Match against filename without extension (stem)
                path.file_stem()
                    .and_then(|os_str| os_str.to_str())
                    .is_some_and(|s| x.is_match(s))
            }
            NamePredicate::FileName(x) => path
                .file_name()
                .and_then(|os_str| os_str.to_str())
                .is_some_and(|s| x.is_match(s)),
            NamePredicate::DirPath(x) => {
                // If base_path is provided, make the parent path relative to it
                if let Some(base) = base_path {
                    if let Some(parent) = path.parent() {
                        // Try to make parent relative to base
                        match parent.strip_prefix(base) {
                            Ok(relative) => {
                                relative.as_os_str().to_str().is_some_and(|s| x.is_match(s))
                            }
                            Err(_) => {
                                // If strip_prefix fails, fall back to absolute path
                                parent.as_os_str().to_str().is_some_and(|s| x.is_match(s))
                            }
                        }
                    } else {
                        false
                    }
                } else {
                    // No base path provided, use absolute path (backward compatibility)
                    path.parent()
                        .and_then(|p| p.as_os_str().to_str())
                        .is_some_and(|s| x.is_match(s))
                }
            }
            NamePredicate::FullPath(x) => {
                // If base_path is provided, make it relative
                if let Some(base) = base_path {
                    match path.strip_prefix(base) {
                        Ok(relative) => {
                            relative.as_os_str().to_str().is_some_and(|s| x.is_match(s))
                        }
                        Err(_) => {
                            // If strip_prefix fails, use absolute path
                            path.as_os_str().to_str().is_some_and(|s| x.is_match(s))
                        }
                    }
                } else {
                    path.as_os_str().to_str().is_some_and(|s| x.is_match(s))
                }
            }
            NamePredicate::Extension(x) => {
                // Handle empty extension case for files without extensions
                match path.extension() {
                    Some(ext) => ext.to_str().is_some_and(|s| x.is_match(s)),
                    None => {
                        // No extension - check if looking for empty string
                        x.is_match("")
                    }
                }
            }
            NamePredicate::ParentDir(x) => {
                // e.g., for "src/utils/helper.rs", parent_dir would be "utils"
                path.parent()
                    .and_then(|p| p.file_name())
                    .and_then(|os_str| os_str.to_str())
                    .is_some_and(|s| x.is_match(s))
            }
            NamePredicate::Depth(matcher) => {
                // Calculate depth from base path (or from root if no base)
                let depth = if let Some(base) = base_path {
                    match path.strip_prefix(base) {
                        Ok(relative) => relative.components().count() as u64,
                        Err(_) => path.components().count() as u64,
                    }
                } else {
                    path.components().count() as u64
                };
                matcher.is_match(depth)
            }
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
    Type(EnumMatcher<DetectFileType>), // file type with parse-time validation
    Modified(TimeMatcher),
    Created(TimeMatcher),
    Accessed(TimeMatcher),
}

impl MetadataPredicate {
    pub fn is_match(&self, metadata: &Metadata) -> bool {
        self.is_match_with_path(metadata)
    }

    pub fn is_match_with_path(&self, metadata: &Metadata) -> bool {
        match self {
            MetadataPredicate::Filesize(range) => range.is_match(metadata.size()),
            MetadataPredicate::Type(enum_matcher) => {
                let ft: FileType = metadata.file_type();
                if let Some(detect_type) = DetectFileType::from_fs_type(&ft) {
                    enum_matcher.is_match(&detect_type)
                } else {
                    false
                }
            }
            MetadataPredicate::Modified(matcher) => matcher.is_match(metadata.mtime()),
            MetadataPredicate::Created(matcher) => matcher.is_match(metadata.ctime()),
            MetadataPredicate::Accessed(matcher) => matcher.is_match(metadata.atime()),
        }
    }
}

/// Structured data predicate for yaml/json/toml queries
/// Separates value operations (==, >, etc) from string operations (regex, contains)
///
/// Value variants store both:
/// - `value`: Native parsed type for type-safe comparisons
/// - `raw_string`: Original RHS string for string coercion fallback in equality checks
///   (prevents information loss from parse→stringify, e.g., "`1_000`" → 1000 → "1000")
#[derive(Debug, Clone, PartialEq)]
pub enum StructuredDataPredicate {
    YamlValue {
        path: Vec<crate::parser::structured_path::PathComponent>,
        operator: crate::parser::typed::StructuredOperator,
        value: yaml_rust2::Yaml,
        raw_string: String,
    },
    YamlString {
        path: Vec<crate::parser::structured_path::PathComponent>,
        matcher: StringMatcher,
    },
    JsonValue {
        path: Vec<crate::parser::structured_path::PathComponent>,
        operator: crate::parser::typed::StructuredOperator,
        value: serde_json::Value,
        raw_string: String,
    },
    JsonString {
        path: Vec<crate::parser::structured_path::PathComponent>,
        matcher: StringMatcher,
    },
    TomlValue {
        path: Vec<crate::parser::structured_path::PathComponent>,
        operator: crate::parser::typed::StructuredOperator,
        value: toml::Value,
        raw_string: String,
    },
    TomlString {
        path: Vec<crate::parser::structured_path::PathComponent>,
        matcher: StringMatcher,
    },
}

#[derive(Debug)]
pub struct StreamingCompiledContentPredicate {
    inner: StreamingHybridRegex,
    source: String,
}

impl StreamingCompiledContentPredicate {
    pub fn new(source: String) -> Result<Self, PredicateParseError> {
        match StreamingHybridRegex::new(&source) {
            Ok(inner) => Ok(Self { inner, source }),
            Err(e) => Err(PredicateParseError::Dfa(e)),
        }
    }

    pub(crate) fn as_ref(&self) -> StreamingCompiledContentPredicateRef<'_> {
        StreamingCompiledContentPredicateRef {
            inner: &self.inner,
            source: &self.source,
        }
    }
}

impl PartialEq for StreamingCompiledContentPredicate {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

#[derive(Clone, Debug)]
pub struct StreamingCompiledContentPredicateRef<'a> {
    pub inner: &'a StreamingHybridRegex,
    pub source: &'a str,
}
