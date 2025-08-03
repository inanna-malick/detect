use git2::Blob;
use regex::Regex;
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
use crate::parse_error::{PredicateParseError, TemporalError, TemporalErrorKind};
use crate::util::Done;
use chrono::{DateTime, Duration, Local, NaiveDate};

/// File type enumeration for type predicates
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    /// Primary string representation for this file type
    pub fn as_str(&self) -> &'static str {
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

    /// All aliases that match this file type
    pub fn aliases(&self) -> &'static [&'static str] {
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

    /// Check if a string matches any alias for this file type
    pub fn matches(&self, s: &str) -> bool {
        self.aliases().iter().any(|&alias| alias == s)
    }

    /// Create from std::fs::FileType
    pub fn from_fs_type(ft: &FileType) -> Option<Self> {
        if ft.is_file() {
            Some(DetectFileType::File)
        } else if ft.is_dir() {
            Some(DetectFileType::Directory)
        } else if ft.is_symlink() {
            Some(DetectFileType::Symlink)
        } else if ft.is_socket() {
            Some(DetectFileType::Socket)
        } else if ft.is_fifo() {
            Some(DetectFileType::Fifo)
        } else if ft.is_block_device() {
            Some(DetectFileType::BlockDevice)
        } else if ft.is_char_device() {
            Some(DetectFileType::CharDevice)
        } else {
            None
        }
    }
}

/// Check for common regex patterns that might be mistakes
fn check_regex_patterns(pattern: &str) -> Option<String> {
    // Check for empty regex
    if pattern.is_empty() {
        return Some("Empty regex pattern will match every line in every file. Consider using a more specific pattern.".to_string());
    }
    
    // Check for unescaped dots that look like file extensions
    if pattern.starts_with('.') && !pattern.starts_with("\\.") {
        // Check if it looks like a file extension pattern
        let rest = &pattern[1..];
        if rest.chars().all(|c| c.is_alphanumeric() || c == '$') {
            return Some(format!(
                "Pattern '{}' has an unescaped dot which matches any character. \
                For file extensions, consider using 'path.extension == {}' or escape the dot: '\\.{}'",
                pattern,
                rest.trim_end_matches('$'),
                rest
            ));
        }
    }
    
    // Check for patterns that look like they're trying to match file extensions
    if pattern.ends_with("js") || pattern.ends_with("ts") || pattern.ends_with("rs") 
       || pattern.ends_with("py") || pattern.ends_with("go") {
        if !pattern.contains('.') && !pattern.contains('\\') {
            return Some(format!(
                "Pattern '{}' might not match as expected. \
                For file extensions, use 'path.extension == {}' or a proper regex like '\\.({})'",
                pattern, pattern, pattern
            ));
        }
    }
    
    // Check for common glob patterns that don't work in regex
    // Note: single '*' is already handled by parse_string function
    
    if pattern.contains("**") {
        return Some("Pattern '**' is not valid in regex. Use '.*' for matching any characters.".to_string());
    }
    
    None
}

fn parse_duration(number: i64, unit: &str, original: &str) -> Result<Duration, TemporalError> {
    match unit {
        "seconds" | "second" | "secs" | "sec" | "s" => Ok(Duration::seconds(number)),
        "minutes" | "minute" | "mins" | "min" | "m" => Ok(Duration::minutes(number)),
        "hours" | "hour" | "hrs" | "hr" | "h" => Ok(Duration::hours(number)),
        "days" | "day" | "d" => Ok(Duration::days(number)),
        "weeks" | "week" | "w" => Ok(Duration::weeks(number)),
        _ => Err(TemporalError {
            input: original.to_string(),
            kind: TemporalErrorKind::UnknownUnit(unit.to_string()),
        }),
    }
}

pub fn parse_time_value(s: &str) -> Result<DateTime<Local>, TemporalError> {
    // Handle relative time formats (both new and legacy)
    // New format: -7days, 7days, 30minutes
    // Legacy format: -7.days
    
    // Try parsing as relative time (with or without minus)
    let (is_negative, stripped) = if let Some(rest) = s.strip_prefix('-') {
        (true, rest)
    } else {
        (false, s)
    };
    
    // First try legacy format with period
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
    
    // Try new format without period (e.g., "7days", "30m")
    // Find where the digits end and the unit begins
    let digit_end = stripped.find(|c: char| !c.is_ascii_digit());
    if let Some(idx) = digit_end {
        let num_str = &stripped[..idx];
        let unit = &stripped[idx..];
        
        // Check if this looks like a date (has dashes after digits) rather than duration
        // Dates look like: 2024-01-01, not like: 7days
        if !unit.starts_with('-') {
            if let Ok(number) = num_str.parse::<i64>() {
                if !unit.is_empty() {
                    // Try to parse as duration unit
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

    // Handle special keywords
    match s {
        "now" => return Ok(Local::now()),
        "today" => {
            let today = Local::now().date_naive();
            return Ok(today
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(Local)
                .unwrap());
        }
        "yesterday" => {
            let yesterday = Local::now().date_naive() - Duration::days(1);
            return Ok(yesterday
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(Local)
                .unwrap());
        }
        _ => {}
    }

    // Try parsing as absolute date/datetime
    match NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        Ok(date) => {
            return Ok(date
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_local_timezone(Local)
                .unwrap())
        }
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

#[derive(Clone, Debug, PartialEq)]
pub enum RhsValue {
    // String values for name, path, ext, type selectors
    String(String),

    // Plain numeric value (bytes)
    Number(u64),

    // Size with unit (converted to bytes)
    Size(u64),

    // Set of values (from [item1, item2] syntax)
    Set(Vec<String>),

    // Temporal values
    RelativeTime { value: i64, unit: TimeUnit },
    AbsoluteTime(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum TimeUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
    Weeks,
    Months,
}

// Helper function to quote strings when needed
fn quote_if_needed(s: &str) -> String {
    // Check if string could be parsed as a size value
    // This checks if the string could be ambiguously parsed as a size
    let could_be_size_prefix = s
        .chars()
        .take_while(|c| c.is_numeric() || *c == '.')
        .count()
        > 0
        && s.len() > 1
        && s.chars()
            .nth(
                s.chars()
                    .take_while(|c| c.is_numeric() || *c == '.')
                    .count(),
            )
            .map(|c| matches!(c, 'k' | 'm' | 'g' | 't' | 'K' | 'M' | 'G' | 'T'))
            .unwrap_or(false);

    // Check if string looks like a number
    // FIXME: If it looks like a number but it's being used in a string matcher context,
    // it should be coerced to a string using the original text representation
    let looks_like_number = !s.is_empty() && s.chars().all(|c| c.is_numeric());

    // Check if string needs quoting
    let needs_quotes = s.is_empty() ||
        could_be_size_prefix ||
        looks_like_number ||
        s.contains(' ') ||
        s.contains('"') ||
        s.contains('\\') || // TODO/FIXME: is this needed?
        s.contains('{') || // TODO/FIXME: is this needed?
        s.contains('}') || // TODO/FIXME: is this needed?
        s.contains(')') ||
        s.contains('(') ||
        !s.chars().all(|c| c.is_alphanumeric() || "_./+-@#$%^&*~()|?".contains(c));

    if needs_quotes {
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{}\"", escaped)
    } else {
        s.to_string()
    }
}

impl Display for RhsValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RhsValue::String(s) => write!(f, "{}", quote_if_needed(s)),
            RhsValue::Number(n) => write!(f, "{}", n),
            RhsValue::Size(bytes) => write!(f, "{}", bytes),
            RhsValue::Set(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", quote_if_needed(item))?;
                }
                write!(f, "]")
            }
            RhsValue::RelativeTime { value, unit } => {
                let unit_str = match unit {
                    TimeUnit::Seconds => "seconds",
                    TimeUnit::Minutes => "minutes",
                    TimeUnit::Hours => "hours",
                    TimeUnit::Days => "days",
                    TimeUnit::Weeks => "weeks",
                    TimeUnit::Months => "months",
                };
                write!(f, "-{}.{}", value, unit_str)
            }
            RhsValue::AbsoluteTime(s) => {
                write!(f, "{}", quote_if_needed(s))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawPredicate {
    pub lhs: Selector,
    pub op: Op,
    pub rhs: RhsValue,
}

impl RawPredicate {
    pub fn parse(
        self,
    ) -> Result<
        Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>,
        PredicateParseError,
    > {
        Ok(match self.lhs {
            Selector::BaseName => {
                Predicate::name(NamePredicate::BaseName(parse_string(&self.op, &self.rhs)?))
            }
            Selector::FileName => {
                Predicate::name(NamePredicate::FileName(parse_string(&self.op, &self.rhs)?))
            }
            Selector::DirPath => {
                Predicate::name(NamePredicate::DirPath(parse_string(&self.op, &self.rhs)?))
            }
            Selector::FullPath => {
                Predicate::name(NamePredicate::FullPath(parse_string(&self.op, &self.rhs)?))
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
            Selector::Depth => Predicate::meta(MetadataPredicate::Depth(parse_numerical(
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
    BaseName,  // filename without extension (e.g. "report" from "report.txt")
    FileName,  // complete filename with extension (e.g. "report.txt")
    DirPath,   // directory path only (e.g. "src/services")
    FullPath,  // complete path including filename (e.g. "src/services/report.txt")
    Extension, // file extension without dot (e.g. "txt")
    // METADATA
    EntityType,
    Size,
    Depth, // directory depth from base path
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

impl Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Selector::BaseName => write!(f, "basename"),
            Selector::FileName => write!(f, "filename"),
            Selector::DirPath => write!(f, "dirpath"),
            Selector::FullPath => write!(f, "fullpath"),
            Selector::Extension => write!(f, "ext"),
            Selector::EntityType => write!(f, "type"),
            Selector::Size => write!(f, "size"),
            Selector::Depth => write!(f, "depth"),
            Selector::Modified => write!(f, "modified"),
            Selector::Created => write!(f, "created"),
            Selector::Accessed => write!(f, "accessed"),
            Selector::Contents => write!(f, "contents"),
        }
    }
}

pub type CompiledMatcher<'a> = DFA<&'a [u32]>;

#[derive(Clone, Debug)]
pub enum StringMatcher {
    Regex(Regex),
    Equals(String),
    NotEquals(String),
    Contains(String),
    In(HashSet<String>),
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

impl Display for StringMatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StringMatcher::Regex(r) => {
                let pattern = r.as_str();
                // Always quote regex patterns to handle special characters and empty patterns
                write!(
                    f,
                    "~= \"{}\"",
                    pattern.replace('\\', "\\\\").replace('"', "\\\"")
                )
            }
            StringMatcher::Equals(s) => write!(f, "== {}", quote_if_needed(s)),
            StringMatcher::NotEquals(s) => write!(f, "!= {}", quote_if_needed(s)),
            StringMatcher::Contains(s) => write!(f, "contains {}", quote_if_needed(s)),
            StringMatcher::In(items) => {
                write!(f, "in [")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", quote_if_needed(item))?;
                }
                write!(f, "]")
            }
        }
    }
}

impl StringMatcher {
    pub fn regex(s: &str) -> Result<Self, regex::Error> {
        // Check for common regex mistakes and provide warnings
        if let Some(warning) = check_regex_patterns(s) {
            eprintln!("Warning: {}", warning);
        }
        Ok(Self::Regex(Regex::new(s)?))
    }
    
    pub fn regex_with_warnings(s: &str) -> Result<(Self, Option<String>), regex::Error> {
        let warning = check_regex_patterns(s);
        Ok((Self::Regex(Regex::new(s)?), warning))
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
                // For date-only comparisons (e.g., "2024-01-01"), we compare just the date
                // For datetime comparisons, we'd compare with time granularity
                // Since we currently only parse dates as YYYY-MM-DD (setting time to 00:00:00),
                // we compare date portions for equality
                file_datetime.date_naive() == dt.date_naive()
            }
            TimeMatcher::NotEquals(dt) => {
                // Same logic as Equals but inverted
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

impl Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Matches => write!(f, "~="),
            Op::Equality => write!(f, "=="),
            Op::NotEqual => write!(f, "!="),
            Op::NumericComparison(NumericalOp::Greater) => write!(f, ">"),
            Op::NumericComparison(NumericalOp::GreaterOrEqual) => write!(f, ">="),
            Op::NumericComparison(NumericalOp::Less) => write!(f, "<"),
            Op::NumericComparison(NumericalOp::LessOrEqual) => write!(f, "<="),
            Op::In => write!(f, "in"),
            Op::Contains => write!(f, "contains"),
        }
    }
}

pub fn parse_string(op: &Op, rhs: &RhsValue) -> Result<StringMatcher, PredicateParseError> {
    match rhs {
        RhsValue::String(s) => {
            Ok(match op {
                Op::Matches => {
                    // Special case for '*' which users commonly expect to work
                    let pattern = if s == "*" { ".*" } else { s };
                    StringMatcher::Regex(Regex::new(pattern)?)
                },
                Op::Equality => StringMatcher::Equals(s.clone()),
                Op::NotEqual => StringMatcher::NotEquals(s.clone()),
                Op::Contains => StringMatcher::Contains(s.clone()),
                Op::In => {
                    let mut set = HashSet::new();
                    set.insert(s.clone());
                    StringMatcher::In(set)
                }
                Op::NumericComparison(_) => {
                    return Err(PredicateParseError::IncompatibleOperation {
                        reason: "Numeric comparison operators (>, <, >=, <=) cannot be used with string values",
                    })
                }
            })
        }
        RhsValue::Set(items) => match op {
            Op::In => Ok(StringMatcher::In(items.iter().cloned().collect())),
            _ => Err(PredicateParseError::IncompatibleOperation {
                reason: "Set values can only be used with 'in' operator",
            }),
        },
        _ => Err(PredicateParseError::IncompatibleValue {
            expected: "string or set",
            found: format!("{:?}", rhs),
        }),
    }
}

pub fn parse_string_dfa(
    op: Op,
    rhs: RhsValue,
) -> Result<StreamingCompiledContentPredicate, PredicateParseError> {
    let s = match rhs {
        RhsValue::String(s) => s,
        _ => {
            return Err(PredicateParseError::IncompatibleValue {
                expected: "string",
                found: format!("{:?}", rhs),
            })
        }
    };

    Ok(match op {
        Op::Matches => StreamingCompiledContentPredicate::new(s)?,
        Op::Equality => {
            let regex = format!("^{}$", regex::escape(&s));
            match DFA::new(&regex) {
                Ok(inner) => StreamingCompiledContentPredicate {
                    inner,
                    source: regex,
                },
                Err(e) => return Err(PredicateParseError::Dfa(e.to_string())),
            }
        }
        Op::Contains => {
            let regex = regex::escape(&s);
            match DFA::new(&regex) {
                Ok(inner) => StreamingCompiledContentPredicate {
                    inner,
                    source: regex,
                },
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

pub fn parse_numerical(op: &Op, rhs: &RhsValue) -> Result<NumberMatcher, PredicateParseError> {
    let parsed_rhs: u64 = match rhs {
        RhsValue::Number(n) => *n,
        RhsValue::Size(bytes) => *bytes,
        _ => {
            return Err(PredicateParseError::IncompatibleValue {
                expected: "number or size value",
                found: format!("{:?}", rhs),
            })
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
        Op::Matches => Err(PredicateParseError::IncompatibleOperation {
            reason: "Regex operator ~= cannot be used with numeric values",
        }),
        Op::Contains => Err(PredicateParseError::IncompatibleOperation {
            reason: "'contains' operator cannot be used with numeric values",
        }),
    }
}

pub fn parse_temporal(op: &Op, rhs: &RhsValue) -> Result<TimeMatcher, PredicateParseError> {
    let s = match rhs {
        RhsValue::String(s) => s,
        // TODO: Handle RhsValue::RelativeTime and RhsValue::AbsoluteTime
        _ => {
            return Err(PredicateParseError::IncompatibleValue {
                expected: "time value",
                found: format!("{:?}", rhs),
            })
        }
    };
    let parsed_time = parse_time_value(s)?;

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
        Op::Matches => Err(PredicateParseError::IncompatibleOperation {
            reason: "Regex operator ~= cannot be used with temporal values",
        }),
        Op::Contains => Err(PredicateParseError::IncompatibleOperation {
            reason: "'contains' operator cannot be used with temporal values",
        }),
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

    pub fn eval_name_predicate_with_base(
        self,
        path: &Path,
        base_path: Option<&Path>,
    ) -> ShortCircuit<Predicate<Done, A, B>> {
        match self {
            Predicate::Name(p) => ShortCircuit::Known(p.is_match_with_base(path, base_path)),
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

    pub fn eval_metadata_predicate_with_path(
        self,
        metadata: &Metadata,
        path: &Path,
        base_path: Option<&Path>,
    ) -> ShortCircuit<Predicate<A, Done, B>> {
        match self {
            Predicate::Metadata(p) => {
                ShortCircuit::Known(p.is_match_with_path(metadata, Some(path), base_path))
            }
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
    BaseName(StringMatcher),  // filename without extension
    FileName(StringMatcher),  // complete filename with extension
    DirPath(StringMatcher),   // directory path only
    FullPath(StringMatcher),  // complete path including filename
    Extension(StringMatcher), // file extension
    ParentDir(StringMatcher), // immediate parent directory name
}

impl NamePredicate {
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
            NamePredicate::FileName(x) => {
                // Match against complete filename with extension
                path.file_name()
                    .and_then(|os_str| os_str.to_str())
                    .is_some_and(|s| x.is_match(s))
            }
            NamePredicate::DirPath(x) => {
                // Match against directory path only (parent)
                // If base_path is provided, make the parent path relative to it
                if let Some(base) = base_path {
                    if let Some(parent) = path.parent() {
                        // Try to make parent relative to base
                        match parent.strip_prefix(base) {
                            Ok(relative) => {
                                // Use relative path
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
                // Match against complete path including filename
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
                // Match against extension without dot
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
                // Match against immediate parent directory name only
                // e.g., for "src/utils/helper.rs", parent_dir would be "utils"
                path.parent()
                    .and_then(|p| p.file_name())
                    .and_then(|os_str| os_str.to_str())
                    .is_some_and(|s| x.is_match(s))
            }
        }
    }
}

impl Display for NamePredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NamePredicate::BaseName(matcher) => write!(f, "path.stem {}", matcher),
            NamePredicate::FileName(matcher) => write!(f, "path.name {}", matcher),
            NamePredicate::DirPath(matcher) => write!(f, "path.parent {}", matcher),
            NamePredicate::FullPath(matcher) => write!(f, "path.full {}", matcher),
            NamePredicate::Extension(matcher) => write!(f, "path.extension {}", matcher),
            NamePredicate::ParentDir(matcher) => write!(f, "path.parent_dir {}", matcher),
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

#[derive(Debug)]
pub enum MetadataPredicate {
    Filesize(NumberMatcher),
    Type(StringMatcher), //dir, exec, etc
    Modified(TimeMatcher),
    Created(TimeMatcher),
    Accessed(TimeMatcher),
    Depth(NumberMatcher), // Directory depth from base path
}

impl Display for MetadataPredicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetadataPredicate::Filesize(matcher) => write!(f, "size {}", matcher),
            MetadataPredicate::Type(matcher) => write!(f, "type {}", matcher),
            MetadataPredicate::Modified(matcher) => write!(f, "modified {}", matcher),
            MetadataPredicate::Created(matcher) => write!(f, "created {}", matcher),
            MetadataPredicate::Accessed(matcher) => write!(f, "accessed {}", matcher),
            MetadataPredicate::Depth(matcher) => write!(f, "depth {}", matcher),
        }
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
            (MetadataPredicate::Depth(a), MetadataPredicate::Depth(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for MetadataPredicate {}

impl Display for NumberMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NumberMatcher::In(bound) => {
                // Convert Bound back to operator syntax
                // This is a lossy conversion because we can't perfectly reconstruct
                // whether it was > or >= from the Bound representation
                match bound {
                    Bound::Left(range) => {
                        // For now, just use > for all left bounds
                        write!(f, "> {}", range.start)
                    }
                    Bound::Right(range) => {
                        // For now, just use < for all right bounds
                        write!(f, "< {}", range.end)
                    }
                    Bound::Full(range) => {
                        // This could represent complex queries, just use > for simplicity
                        // Since Full ranges are rare in our usage
                        write!(f, "> {}", range.start)
                    }
                }
            }
            NumberMatcher::Equals(n) => write!(f, "== {}", n),
            NumberMatcher::NotEquals(n) => write!(f, "!= {}", n),
        }
    }
}

impl Display for Bound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The Bound is used within NumberMatcher which already handles the operator
        // This should never be called directly for query generation
        match self {
            Bound::Full(range) => write!(f, "{}..{}", range.start, range.end),
            Bound::Left(range) => write!(f, "{}..", range.start),
            Bound::Right(range) => write!(f, "..{}", range.end),
        }
    }
}

impl Display for TimeMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeMatcher::Before(dt) => write!(f, "< {}", quote_if_needed(&dt.to_rfc3339())),
            TimeMatcher::After(dt) => write!(f, "> {}", quote_if_needed(&dt.to_rfc3339())),
            TimeMatcher::Equals(dt) => write!(f, "== {}", quote_if_needed(&dt.to_rfc3339())),
            TimeMatcher::NotEquals(dt) => write!(f, "!= {}", quote_if_needed(&dt.to_rfc3339())),
        }
    }
}

impl MetadataPredicate {
    pub fn is_match(&self, metadata: &Metadata) -> bool {
        self.is_match_with_path(metadata, None, None)
    }

    pub fn is_match_with_path(
        &self,
        metadata: &Metadata,
        path: Option<&Path>,
        base_path: Option<&Path>,
    ) -> bool {
        match self {
            MetadataPredicate::Filesize(range) => range.is_match(metadata.size()),
            MetadataPredicate::Type(matcher) => {
                let ft: FileType = metadata.file_type();
                // Use the new DetectFileType enum for cleaner type checking
                if let Some(detect_type) = DetectFileType::from_fs_type(&ft) {
                    detect_type.aliases().iter().any(|&alias| matcher.is_match(alias))
                } else {
                    false
                }
            }
            MetadataPredicate::Modified(matcher) => matcher.is_match(metadata.mtime()),
            MetadataPredicate::Created(matcher) => matcher.is_match(metadata.ctime()),
            MetadataPredicate::Accessed(matcher) => matcher.is_match(metadata.atime()),
            MetadataPredicate::Depth(matcher) => {
                // Calculate depth based on path components
                if let Some(path) = path {
                    let depth = if let Some(base) = base_path {
                        // Calculate relative depth from base
                        match path.strip_prefix(base) {
                            Ok(relative) => relative.components().count() as u64,
                            Err(_) => path.components().count() as u64,
                        }
                    } else {
                        // Use absolute depth
                        path.components().count() as u64
                    };
                    matcher.is_match(depth)
                } else {
                    // No path provided, can't calculate depth
                    false
                }
            }
        }
    }

    pub fn is_match_git_tree(&self) -> bool {
        match self {
            MetadataPredicate::Filesize(_) => {
                // it's not a file
                false
            }
            MetadataPredicate::Type(matcher) => {
                // Check if matcher matches directory type
                DetectFileType::Directory.aliases().iter().any(|&alias| matcher.is_match(alias))
            }
            MetadataPredicate::Modified(_)
            | MetadataPredicate::Created(_)
            | MetadataPredicate::Accessed(_)
            | MetadataPredicate::Depth(_) => {
                // Git trees don't have timestamps or depth info without path
                false
            }
        }
    }

    pub fn is_match_git_blob(&self, entry: &Blob) -> bool {
        match self {
            MetadataPredicate::Filesize(range) => range.is_match(entry.size() as u64),
            MetadataPredicate::Type(matcher) => {
                // Check if matcher matches file type
                DetectFileType::File.aliases().iter().any(|&alias| matcher.is_match(alias))
            }
            MetadataPredicate::Modified(_)
            | MetadataPredicate::Created(_)
            | MetadataPredicate::Accessed(_)
            | MetadataPredicate::Depth(_) => {
                // Git blobs don't have timestamps or depth info without path
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
        // Always quote regex patterns to handle special characters and empty patterns
        write!(
            f,
            "contents ~= \"{}\"",
            self.source.replace('\\', "\\\\").replace('"', "\\\"")
        )
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
        // Always quote regex patterns to handle special characters and empty patterns
        write!(
            f,
            "contents ~= \"{}\"",
            self.source.replace('\\', "\\\\").replace('"', "\\\"")
        )
    }
}
