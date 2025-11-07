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
fn test_alias_with_word_form_operators() {
    // Test word-form AND operator
    let result = parse_and_typecheck("file AND size > 10kb");
    assert!(
        result.is_ok(),
        "Alias with word-form AND should parse, got: {:?}",
        result.err()
    );

    // Test word-form OR operator
    let result = parse_and_typecheck("dir OR file");
    assert!(
        result.is_ok(),
        "Alias with word-form OR should parse, got: {:?}",
        result.err()
    );

    // Test case-insensitive word operators
    let result = parse_and_typecheck("file and size > 1mb");
    assert!(
        result.is_ok(),
        "Alias with lowercase 'and' should parse, got: {:?}",
        result.err()
    );

    let result = parse_and_typecheck("file or dir");
    assert!(
        result.is_ok(),
        "Alias with lowercase 'or' should parse, got: {:?}",
        result.err()
    );

    // Mixed case
    let result = parse_and_typecheck("file And size > 1kb");
    assert!(
        result.is_ok(),
        "Alias with mixed-case 'And' should parse, got: {:?}",
        result.err()
    );

    // Complex expressions with multiple word operators
    let result = parse_and_typecheck("file AND size > 1mb OR dir AND depth < 3");
    assert!(
        result.is_ok(),
        "Complex expression with multiple word operators should parse, got: {:?}",
        result.err()
    );

    // Parenthesized expressions
    let result = parse_and_typecheck("(file OR dir) AND size > 100kb");
    assert!(
        result.is_ok(),
        "Parenthesized expression with word operators should parse, got: {:?}",
        result.err()
    );

    // With NOT
    let result = parse_and_typecheck("NOT file AND size > 1kb");
    assert!(
        result.is_ok(),
        "NOT with word-form AND should parse, got: {:?}",
        result.err()
    );
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

// ============================================================================
// Structured Selector Alias Tests
// ============================================================================

#[test]
fn test_structured_selector_alias_parsing() {
    // YAML structured selectors should parse
    assert!(parse_and_typecheck("yaml:.field").is_ok());
    assert!(parse_and_typecheck("yaml:.server.port").is_ok());
    assert!(parse_and_typecheck("yaml:.items[0]").is_ok());
    assert!(parse_and_typecheck("yaml:.items[*].name").is_ok());
    assert!(parse_and_typecheck("yaml:..recursive").is_ok());

    // JSON structured selectors should parse
    assert!(parse_and_typecheck("json:.version").is_ok());
    assert!(parse_and_typecheck("json:.dependencies.lodash").is_ok());

    // TOML structured selectors should parse
    assert!(parse_and_typecheck("toml:.package.name").is_ok());
    assert!(parse_and_typecheck("toml:.dependencies.serde").is_ok());
}

#[test]
fn test_structured_selector_case_insensitive() {
    // Format prefix should be case-insensitive
    assert!(parse_and_typecheck("YAML:.field").is_ok());
    assert!(parse_and_typecheck("Json:.field").is_ok());
    assert!(parse_and_typecheck("toml:.field").is_ok());
    assert!(parse_and_typecheck("YaML:.field").is_ok());
}

#[test]
fn test_structured_selector_with_boolean_logic() {
    // Structured selectors should work in boolean expressions
    assert!(parse_and_typecheck("yaml:.field AND size > 1kb").is_ok());
    assert!(parse_and_typecheck("json:.version OR toml:.version").is_ok());
    assert!(parse_and_typecheck("NOT yaml:.field").is_ok());
    assert!(parse_and_typecheck("(yaml:.a OR json:.b) && file").is_ok());
}

#[test]
fn test_structured_selector_complex_and_logic() {
    // Complex AND expressions with structured selectors
    assert!(parse_and_typecheck("yaml:.field AND json:.field").is_ok());
    assert!(parse_and_typecheck("yaml:.a AND yaml:.b AND yaml:.c").is_ok());
    assert!(parse_and_typecheck("yaml:.field && json:.field && toml:.field").is_ok());

    // Mixed word-form and symbol operators
    assert!(parse_and_typecheck("yaml:.field AND json:.field && toml:.field").is_ok());
    assert!(parse_and_typecheck("yaml:.a && yaml:.b AND yaml:.c").is_ok());
}

#[test]
fn test_structured_selector_complex_or_logic() {
    // Complex OR expressions with structured selectors
    assert!(parse_and_typecheck("yaml:.field OR json:.field").is_ok());
    assert!(parse_and_typecheck("yaml:.a OR yaml:.b OR yaml:.c").is_ok());
    assert!(parse_and_typecheck("yaml:.field || json:.field || toml:.field").is_ok());

    // Mixed word-form and symbol operators
    assert!(parse_and_typecheck("yaml:.field OR json:.field || toml:.field").is_ok());
    assert!(parse_and_typecheck("yaml:.a || yaml:.b OR yaml:.c").is_ok());
}

#[test]
fn test_structured_selector_negation_variants() {
    // All negation forms should work
    assert!(parse_and_typecheck("NOT yaml:.field").is_ok());
    assert!(parse_and_typecheck("not yaml:.field").is_ok());
    assert!(parse_and_typecheck("! yaml:.field").is_ok());
    assert!(parse_and_typecheck("\\! yaml:.field").is_ok());

    // Double negation
    assert!(parse_and_typecheck("NOT NOT yaml:.field").is_ok());
    assert!(parse_and_typecheck("!! yaml:.field").is_ok());

    // Negation with multiple selectors
    assert!(parse_and_typecheck("NOT (yaml:.a OR yaml:.b)").is_ok());
    assert!(parse_and_typecheck("!(yaml:.a AND yaml:.b)").is_ok());
}

#[test]
fn test_structured_selector_precedence_and_grouping() {
    // Test operator precedence with parentheses
    assert!(parse_and_typecheck("yaml:.a AND (yaml:.b OR yaml:.c)").is_ok());
    assert!(parse_and_typecheck("(yaml:.a OR yaml:.b) AND yaml:.c").is_ok());
    assert!(parse_and_typecheck("yaml:.a OR yaml:.b AND yaml:.c").is_ok());

    // Complex nested grouping
    assert!(parse_and_typecheck("((yaml:.a OR yaml:.b) AND yaml:.c) OR yaml:.d").is_ok());
    assert!(parse_and_typecheck("yaml:.a AND (yaml:.b OR (yaml:.c AND yaml:.d))").is_ok());

    // Negation with grouping
    assert!(parse_and_typecheck("NOT (yaml:.a AND yaml:.b)").is_ok());
    assert!(parse_and_typecheck("!(yaml:.a || yaml:.b) AND yaml:.c").is_ok());
}

#[test]
fn test_structured_selector_mixed_with_aliases() {
    // Structured selectors combined with file type aliases
    assert!(parse_and_typecheck("yaml:.field AND file").is_ok());
    assert!(parse_and_typecheck("file AND yaml:.field").is_ok());
    assert!(parse_and_typecheck("dir OR yaml:.config").is_ok());
    assert!(parse_and_typecheck("yaml:.field AND NOT symlink").is_ok());

    // Complex combinations
    assert!(parse_and_typecheck("(file OR dir) AND yaml:.field").is_ok());
    assert!(parse_and_typecheck("yaml:.a AND (file OR symlink) AND json:.b").is_ok());
    assert!(parse_and_typecheck("NOT file OR yaml:.config").is_ok());
}

#[test]
fn test_structured_selector_mixed_with_predicates() {
    // Structured selectors with other predicate types
    assert!(parse_and_typecheck("yaml:.field AND size > 10kb").is_ok());
    assert!(parse_and_typecheck("yaml:.field AND name == test.yaml").is_ok());
    assert!(parse_and_typecheck("yaml:.field AND ext == yaml").is_ok());
    assert!(parse_and_typecheck("yaml:.field AND modified > -7d").is_ok());

    // Complex mixed predicates
    assert!(parse_and_typecheck("yaml:.field AND size > 1mb AND ext == yaml").is_ok());
    assert!(parse_and_typecheck("(yaml:.field OR json:.field) AND size < 10mb").is_ok());
    assert!(parse_and_typecheck("yaml:.a AND yaml:.b AND name ~= \"config.*\"").is_ok());

    // With negation
    assert!(parse_and_typecheck("yaml:.field AND NOT (size > 1gb)").is_ok());
    assert!(parse_and_typecheck("NOT yaml:.field AND ext == yaml").is_ok());
}

#[test]
fn test_structured_selector_de_morgan_laws() {
    // Test De Morgan's law equivalences parse correctly
    // NOT (A AND B) is equivalent to (NOT A) OR (NOT B)
    assert!(parse_and_typecheck("NOT (yaml:.a AND yaml:.b)").is_ok());
    assert!(parse_and_typecheck("NOT yaml:.a OR NOT yaml:.b").is_ok());

    // NOT (A OR B) is equivalent to (NOT A) AND (NOT B)
    assert!(parse_and_typecheck("NOT (yaml:.a OR yaml:.b)").is_ok());
    assert!(parse_and_typecheck("NOT yaml:.a AND NOT yaml:.b").is_ok());
}

#[test]
fn test_structured_selector_all_formats_combined() {
    // All three formats in complex expressions
    assert!(parse_and_typecheck("yaml:.a AND json:.b AND toml:.c").is_ok());
    assert!(parse_and_typecheck("yaml:.a OR json:.b OR toml:.c").is_ok());
    assert!(parse_and_typecheck("(yaml:.a AND json:.b) OR toml:.c").is_ok());
    assert!(parse_and_typecheck("yaml:.a AND (json:.b OR toml:.c)").is_ok());

    // With negation
    assert!(parse_and_typecheck("yaml:.a AND NOT json:.b AND toml:.c").is_ok());
    assert!(parse_and_typecheck("NOT (yaml:.a OR json:.b OR toml:.c)").is_ok());

    // Mixed with other predicates
    assert!(parse_and_typecheck("yaml:.a AND json:.b AND toml:.c AND file").is_ok());
    assert!(parse_and_typecheck("(yaml:.a OR json:.b OR toml:.c) AND size > 1kb").is_ok());
}

#[test]
fn test_structured_selector_invalid_format() {
    // Invalid format prefixes should produce UnknownStructuredFormat error
    let result = parse_and_typecheck("xml:.field");
    assert!(result.is_err());

    if let Err(err) = result {
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("UnknownStructuredFormat") || err_str.contains("xml"),
            "Expected UnknownStructuredFormat error, got: {:?}",
            err
        );
    }

    // Other invalid formats
    assert!(parse_and_typecheck("csv:.column").is_err());
    assert!(parse_and_typecheck("ini:.section").is_err());
}

#[test]
fn test_structured_selector_invalid_path() {
    // Empty path should produce error
    let result = parse_and_typecheck("yaml:");
    assert!(result.is_err(), "Empty path should fail");

    // Invalid path syntax should produce error
    let result = parse_and_typecheck("yaml:[");
    assert!(result.is_err(), "Unclosed bracket should fail");
}

#[test]
fn test_structured_selector_constructs_exists_predicate() {
    // Verify that structured selectors construct StructuredDataPredicate::*Exists
    use detect::predicate::StructuredDataPredicate;

    let typed = parse_and_typecheck("yaml:.field").unwrap();

    match typed {
        Expr::Predicate(Predicate::Structured(predicate)) => match predicate {
            StructuredDataPredicate::YamlExists { path } => {
                assert_eq!(path.len(), 1, "Should have one path component");
            }
            _ => panic!("Expected YamlExists predicate, got: {:?}", predicate),
        },
        _ => panic!("Expected Predicate::Structured, got: {:?}", typed),
    }

    let typed = parse_and_typecheck("json:.version").unwrap();

    match typed {
        Expr::Predicate(Predicate::Structured(StructuredDataPredicate::JsonExists { .. })) => {
            // Success
        }
        _ => panic!("Expected JsonExists predicate"),
    }

    let typed = parse_and_typecheck("toml:.package").unwrap();

    match typed {
        Expr::Predicate(Predicate::Structured(StructuredDataPredicate::TomlExists { .. })) => {
            // Success
        }
        _ => panic!("Expected TomlExists predicate"),
    }
}

