//! Tests for single-word predicate aliases
//!
//! Verifies that file type aliases like `dir`, `file`, `symlink` work correctly
//! in parsing and typechecking.

use detect::{
    expr::Expr,
    parser::{RawParser, Typechecker},
    predicate::{DetectFileType, EnumMatcher, MetadataPredicate, Predicate},
};

/// Helper to parse and typecheck an expression
fn parse_and_typecheck(input: &str) -> Result<Expr<Predicate>, detect::parser::error::DetectError> {
    let raw = RawParser::parse_raw_expr(input)?;
    Typechecker::typecheck(raw, input, &detect::RuntimeConfig::default())
}

#[test]
fn test_all_file_type_aliases_parse() {
    // All file type aliases should parse and typecheck successfully
    let aliases = [
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
    ];

    for alias in &aliases {
        let result = parse_and_typecheck(alias);
        assert!(
            result.is_ok(),
            "Alias '{}' should parse successfully, got: {:?}",
            alias,
            result.err()
        );
    }
}

#[test]
fn test_alias_case_insensitive() {
    // Aliases should be case-insensitive
    assert!(parse_and_typecheck("FILE").is_ok());
    assert!(parse_and_typecheck("Dir").is_ok());
    assert!(parse_and_typecheck("DIRECTORY").is_ok());
    assert!(parse_and_typecheck("SyMlInK").is_ok());
}

#[test]
fn test_alias_equivalence_to_explicit_predicate() {
    // `dir` should be equivalent to `type == dir`
    let alias_result = parse_and_typecheck("dir").unwrap();
    let explicit_result = parse_and_typecheck("type == dir").unwrap();

    // Both should produce MetadataPredicate::Type with Equals matcher
    match (&alias_result, &explicit_result) {
        (Expr::Predicate(Predicate::Metadata(a)), Expr::Predicate(Predicate::Metadata(e))) => {
            assert_eq!(a, e, "Alias and explicit predicate should be equal");
        }
        _ => panic!("Both should be Predicate::Metadata"),
    }
}

#[test]
fn test_alias_in_boolean_expression() {
    // Aliases should work in boolean expressions
    let result = parse_and_typecheck("dir && depth > 0");
    assert!(result.is_ok(), "Boolean expression with alias should parse");

    let result = parse_and_typecheck("file || symlink");
    assert!(result.is_ok(), "OR with aliases should parse");

    let result = parse_and_typecheck("NOT dir");
    assert!(result.is_ok(), "NOT with alias should parse");
}

#[test]
fn test_unknown_alias_error() {
    // Unknown aliases should produce helpful errors
    let result = parse_and_typecheck("unknownalias");
    assert!(result.is_err());

    if let Err(err) = result {
        assert!(
            matches!(err, detect::parser::error::DetectError::UnknownAlias { .. }),
            "Should produce UnknownAlias error"
        );
    }
}

#[test]
fn test_alias_suggestions() {
    // Close typos should get suggestions
    let result = parse_and_typecheck("fil");
    assert!(result.is_err());

    if let Err(detect::parser::error::DetectError::UnknownAlias { suggestions, .. }) = result {
        assert!(
            suggestions.is_some(),
            "Should provide suggestions for typos"
        );
        let sugg = suggestions.unwrap();
        assert!(
            sugg.contains("file"),
            "Should suggest 'file' for 'fil', got: {}",
            sugg
        );
    }
}

#[test]
fn test_wildcard_rejected() {
    // Wildcards should no longer parse as single words
    let result = RawParser::parse_raw_expr("*.rs");
    assert!(
        result.is_err(),
        "Wildcards should be rejected by new grammar"
    );

    let result = RawParser::parse_raw_expr("**/*.js");
    assert!(result.is_err(), "Complex glob patterns should be rejected");
}

// Filesystem evaluation is tested in integration.rs
// This file focuses on parsing and typechecking of aliases

#[test]
fn test_complex_alias_expressions() {
    // Test complex boolean logic with aliases
    let result = parse_and_typecheck("(file || dir) && depth < 5");
    assert!(result.is_ok(), "Complex alias expression should parse");

    let result = parse_and_typecheck("NOT (symlink || socket) && file");
    assert!(result.is_ok(), "Complex negation with aliases should parse");
}

#[test]
fn test_alias_constructed_predicates() {
    // Verify that aliases construct the correct predicate internally
    let typed = parse_and_typecheck("file").unwrap();

    match typed {
        Expr::Predicate(Predicate::Metadata(meta)) => match meta.as_ref() {
            MetadataPredicate::Type(EnumMatcher::Equals(file_type)) => {
                assert_eq!(
                    file_type,
                    &DetectFileType::File,
                    "Alias 'file' should construct DetectFileType::File"
                );
            }
            _ => panic!("Expected Type predicate with Equals matcher"),
        },
        _ => panic!("Expected Predicate::Metadata"),
    }
}
