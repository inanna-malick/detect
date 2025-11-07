use std::collections::HashSet;
use std::fmt::{self, Debug, Display};
use std::hash::Hash;

/// Generic matcher for enum-valued predicates with parse-time validation
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnumMatcher<E: EnumPredicate> {
    Equals(E),
    NotEquals(E),
    In(HashSet<E>),
}

/// Trait for enums usable as predicate values
///
/// Implementors provide parsing from string aliases, validation,
/// and display logic for enum-based selectors like `type`.
pub trait EnumPredicate: Sized + Eq + Hash + Clone + Debug {
    /// Parse from string, checking all aliases.
    ///
    /// Returns error message on failure (not a structured error type,
    /// since it gets wrapped in `DetectError::InvalidValue` immediately).
    fn from_str(s: &str) -> Result<Self, String>;

    /// All valid string representations (for error messages)
    fn all_valid_strings() -> &'static [&'static str];

    /// Canonical string representation for this variant
    fn as_str(&self) -> &'static str;

    /// All aliases that map to this variant
    fn aliases(&self) -> &'static [&'static str];
}

impl<E: EnumPredicate> EnumMatcher<E> {
    /// Check if a value matches this enum matcher
    pub fn is_match(&self, value: &E) -> bool {
        match self {
            EnumMatcher::Equals(v) => value == v,
            EnumMatcher::NotEquals(v) => value != v,
            EnumMatcher::In(set) => set.contains(value),
        }
    }
}

impl<E: EnumPredicate> Display for EnumMatcher<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnumMatcher::Equals(v) => write!(f, "== {}", v.as_str()),
            EnumMatcher::NotEquals(v) => write!(f, "!= {}", v.as_str()),
            EnumMatcher::In(set) => {
                write!(f, "in [")?;
                let mut items: Vec<_> = set.iter().map(EnumPredicate::as_str).collect();
                items.sort_unstable(); // Deterministic display order
                write!(f, "{}", items.join(", "))?;
                write!(f, "]")
            }
        }
    }
}
