//! Single-word predicate aliases
//!
//! Provides shorthand syntax like `dir` instead of `type == dir`,
//! enabling more natural queries: `dir && depth > 0`
//!
//! Also handles structured data selectors like `yaml:.field` as existence predicates.

use std::sync::Arc;

use crate::predicate::{DetectFileType, EnumMatcher, EnumPredicate, MetadataPredicate, Predicate};
use super::typed::{AliasError, parse_structured_selector};

/// Resolve a single-word alias to a predicate
///
/// Supports:
/// - File type aliases: `file`, `dir`, `symlink`, etc.
/// - Structured data selectors: `yaml:.field`, `json:.path`, `toml:.key` (TODO: existence predicates)
///
/// Example: `resolve_alias("dir")` is equivalent to `type == dir`
pub fn resolve_alias(word: &str) -> Result<Predicate, AliasError> {
    // Check if it's a structured selector
    match parse_structured_selector(word) {
        Ok(Some((_format, _components))) => {
            unimplemented!("Existence predicates for structured selectors not yet implemented");
        }
        Ok(None) => {
            // Not a structured selector, try file type alias
        }
        Err(e) => {
            return Err(AliasError::Structured(e));
        }
    }

    // Try to resolve as file type alias
    match DetectFileType::from_str(word) {
        Ok(file_type) => Ok(Predicate::Metadata(Arc::new(MetadataPredicate::Type(
            EnumMatcher::Equals(file_type),
        )))),
        Err(_) => Err(AliasError::UnknownAlias(word.to_string())),
    }
}

/// Suggest similar aliases for an unknown word
///
/// Uses simple edit distance to find close matches

pub fn suggest_aliases(word: &str) -> Vec<String> {
    let all_aliases = DetectFileType::all_valid_strings();

    all_aliases
        .iter()
        .filter(|&&alias| levenshtein_distance(word, alias) <= 2)
        .map(|&s| s.to_string())
        .collect()
}

/// Simple Levenshtein distance implementation for fuzzy matching
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row = vec![0; b_len + 1];

    for (i, a_char) in a_chars.iter().enumerate() {
        curr_row[0] = i + 1;

        for (j, b_char) in b_chars.iter().enumerate() {
            let cost = usize::from(a_char != b_char);
            curr_row[j + 1] = (curr_row[j] + 1) // insertion
                .min(prev_row[j + 1] + 1) // deletion
                .min(prev_row[j] + cost); // substitution
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_aliases() {
        // All file type aliases should resolve
        assert!(resolve_alias("file").is_ok());
        assert!(resolve_alias("dir").is_ok());
        assert!(resolve_alias("directory").is_ok());
        assert!(resolve_alias("symlink").is_ok());
        assert!(resolve_alias("link").is_ok());
        assert!(resolve_alias("socket").is_ok());
        assert!(resolve_alias("sock").is_ok());
        assert!(resolve_alias("fifo").is_ok());
        assert!(resolve_alias("pipe").is_ok());
        assert!(resolve_alias("block").is_ok());
        assert!(resolve_alias("blockdev").is_ok());
        assert!(resolve_alias("char").is_ok());
        assert!(resolve_alias("chardev").is_ok());
    }

    #[test]
    fn test_unknown_alias() {
        let result = resolve_alias("unknown");
        assert!(matches!(result, Err(AliasError::UnknownAlias(_))));
    }

    #[test]
    fn test_case_insensitive() {
        // DetectFileType::from_str is case-insensitive
        assert!(resolve_alias("FILE").is_ok());
        assert!(resolve_alias("Dir").is_ok());
        assert!(resolve_alias("SYMLINK").is_ok());
    }

    #[test]
    fn test_suggestions() {
        // Close matches should be suggested
        let suggestions = suggest_aliases("fil");
        assert!(suggestions.contains(&"file".to_string()));

        let suggestions = suggest_aliases("direktory");
        assert!(suggestions.contains(&"directory".to_string()));
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("file", "file"), 0);
        assert_eq!(levenshtein_distance("file", "fil"), 1);
        assert_eq!(levenshtein_distance("directory", "dir"), 6);
        assert_eq!(levenshtein_distance("", "test"), 4);
        assert_eq!(levenshtein_distance("test", ""), 4);
    }
}
