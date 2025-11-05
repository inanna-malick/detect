use regex_automata::dfa::dense::DFA;
use regex_automata::dfa::Automaton;
use regex_automata::Input;
use std::borrow::Cow;

/// Streaming regex engine that silently falls back from Rust DFA to PCRE2
/// Uses DFA for incremental/streaming matching on byte slices
#[derive(Debug)]
pub enum StreamingHybridRegex {
    /// Rust regex DFA for streaming (boxed to reduce size difference)
    RustDFA(Box<DFA<Vec<u32>>>),
    /// PCRE2 regex for compatibility
    Pcre2(pcre2::bytes::Regex),
}

impl StreamingHybridRegex {
    /// Create a new streaming hybrid regex, trying Rust DFA first, falling back to PCRE2
    pub fn new(pattern: &str) -> Result<Self, String> {
        // Try Rust regex DFA first (preferred for performance)
        match DFA::new(pattern) {
            Ok(dfa) => Ok(StreamingHybridRegex::RustDFA(Box::new(dfa))),
            Err(_rust_err) => {
                // Silent fallback to PCRE2
                match pcre2::bytes::Regex::new(pattern) {
                    Ok(pcre2_re) => Ok(StreamingHybridRegex::Pcre2(pcre2_re)),
                    Err(pcre2_err) => Err(format!("Invalid regex pattern: {}", pcre2_err)),
                }
            }
        }
    }

    pub fn is_match(&self, text: &[u8]) -> bool {
        match self {
            StreamingHybridRegex::RustDFA(dfa) => {
                let input = Input::new(text);
                matches!(dfa.try_search_fwd(&input), Ok(Some(_)))
            }
            StreamingHybridRegex::Pcre2(re) => re.is_match(text).unwrap_or(false),
        }
    }
}

/// Hybrid regex engine for in-memory strings with fallback to PCRE2
#[derive(Clone)]
pub enum HybridRegex {
    Rust(regex::Regex),
    Pcre2(pcre2::bytes::Regex),
}

// use string representation as a heuristic for regex equality
impl PartialEq for HybridRegex {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Rust(l0), Self::Rust(r0)) => l0.as_str() == r0.as_str(),
            (Self::Pcre2(l0), Self::Pcre2(r0)) => l0.as_str() == r0.as_str(),
            _ => false,
        }
    }
}

impl Eq for HybridRegex {}

impl HybridRegex {
    pub fn new(pattern: &str) -> Result<Self, regex::Error> {
        match regex::Regex::new(pattern) {
            Ok(re) => Ok(HybridRegex::Rust(re)),
            Err(_) => {
                // Silent fallback to PCRE2
                match pcre2::bytes::Regex::new(pattern) {
                    Ok(pcre2_re) => Ok(HybridRegex::Pcre2(pcre2_re)),
                    Err(pcre2_err) => {
                        Err(regex::Error::Syntax(format!("PCRE2: {}", pcre2_err)))
                    }
                }
            }
        }
    }

    pub fn is_match(&self, text: &str) -> bool {
        match self {
            HybridRegex::Rust(re) => re.is_match(text),
            HybridRegex::Pcre2(re) => re.is_match(text.as_bytes()).unwrap_or(false),
        }
    }

    pub fn as_str(&self) -> Cow<'_, str> {
        match self {
            HybridRegex::Rust(re) => Cow::Borrowed(re.as_str()),
            HybridRegex::Pcre2(_re) => Cow::Borrowed(_re.as_str()),
        }
    }
}

impl std::fmt::Debug for HybridRegex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Regex({})", self.as_str())
    }
}

impl std::fmt::Display for HybridRegex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
