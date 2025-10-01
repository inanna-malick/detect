use regex_automata::dfa::dense::DFA;
use regex_automata::dfa::Automaton;
use regex_automata::Input;
use std::borrow::Cow;

/// Hybrid regex engine that silently falls back from Rust regex to PCRE2
#[derive(Debug)]
pub enum HybridRegex {
    /// Rust regex DFA for streaming (boxed to reduce size difference)
    RustDFA(Box<DFA<Vec<u32>>>),
    /// PCRE2 regex for compatibility
    Pcre2(pcre2::bytes::Regex),
}

impl HybridRegex {
    /// Create a new hybrid regex, trying Rust first, falling back to PCRE2
    pub fn new(pattern: &str) -> Result<Self, String> {
        // Try Rust regex DFA first (preferred for performance)
        match DFA::new(pattern) {
            Ok(dfa) => Ok(HybridRegex::RustDFA(Box::new(dfa))),
            Err(_rust_err) => {
                // Silent fallback to PCRE2
                match pcre2::bytes::Regex::new(pattern) {
                    Ok(pcre2_re) => Ok(HybridRegex::Pcre2(pcre2_re)),
                    Err(pcre2_err) => Err(format!("Invalid regex pattern: {}", pcre2_err)),
                }
            }
        }
    }

    /// Get a borrowed reference version for streaming
    pub fn as_ref(&self) -> HybridRegexRef<'_> {
        match self {
            HybridRegex::RustDFA(dfa) => HybridRegexRef::RustDFA(dfa.as_ref().as_ref()),
            HybridRegex::Pcre2(re) => HybridRegexRef::Pcre2(re),
        }
    }
}

/// Borrowed version of HybridRegex for streaming operations
#[derive(Clone, Debug)]
pub enum HybridRegexRef<'a> {
    RustDFA(DFA<&'a [u32]>),
    Pcre2(&'a pcre2::bytes::Regex),
}

impl<'a> HybridRegexRef<'a> {
    pub fn is_match(&self, text: &[u8]) -> bool {
        match self {
            HybridRegexRef::RustDFA(dfa) => {
                let input = Input::new(text);
                matches!(dfa.try_search_fwd(&input), Ok(Some(_)))
            }
            HybridRegexRef::Pcre2(re) => re.is_match(text).unwrap_or(false),
        }
    }
}

/// Helper for StringMatcher to support both engines
#[derive(Clone)]
pub enum HybridStringRegex {
    Rust(regex::Regex),
    Pcre2(pcre2::bytes::Regex),
}

impl HybridStringRegex {
    pub fn new(pattern: &str) -> Result<Self, regex::Error> {
        // Try Rust regex first
        match regex::Regex::new(pattern) {
            Ok(re) => Ok(HybridStringRegex::Rust(re)),
            Err(_) => {
                // Silent fallback to PCRE2
                match pcre2::bytes::Regex::new(pattern) {
                    Ok(pcre2_re) => Ok(HybridStringRegex::Pcre2(pcre2_re)),
                    Err(pcre2_err) => {
                        // Convert PCRE2 error to regex::Error for compatibility
                        Err(regex::Error::Syntax(format!("PCRE2: {}", pcre2_err)))
                    }
                }
            }
        }
    }

    pub fn is_match(&self, text: &str) -> bool {
        match self {
            HybridStringRegex::Rust(re) => re.is_match(text),
            HybridStringRegex::Pcre2(re) => re.is_match(text.as_bytes()).unwrap_or(false),
        }
    }

    pub fn as_str(&self) -> Cow<'_, str> {
        match self {
            HybridStringRegex::Rust(re) => Cow::Borrowed(re.as_str()),
            HybridStringRegex::Pcre2(_re) => {
                // PCRE2 doesn't provide pattern access, return placeholder
                Cow::Borrowed("<pcre2 pattern>")
            }
        }
    }
}

impl std::fmt::Debug for HybridStringRegex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Regex({})", self.as_str())
    }
}

impl std::fmt::Display for HybridStringRegex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
