//! Integration tests for structured data evaluation (YAML/JSON/TOML)
//!
//! Tests end-to-end functionality including parsing, navigation, comparison,
//! type coercion, and the 2x2 match logic optimization.

use slog::{o, Discard, Logger};
use tempdir::TempDir;

// Load fixtures at compile time
const CONFIG_YAML: &str = include_str!("fixtures/structured/config.yaml");
const PACKAGE_JSON: &str = include_str!("fixtures/structured/package.json");
const CARGO_TOML: &str = include_str!("fixtures/structured/Cargo.toml");
const MULTI_DOC_YAML: &str = include_str!("fixtures/structured/multi_doc.yaml");
const INVALID_YAML: &str = include_str!("fixtures/structured/invalid.yaml");
const INVALID_JSON: &str = include_str!("fixtures/structured/invalid.json");
const INVALID_TOML: &str = include_str!("fixtures/structured/invalid.toml");
const EMPTY_YAML: &str = include_str!("fixtures/structured/empty.yaml");
const EMPTY_JSON: &str = include_str!("fixtures/structured/empty.json");
const EMPTY_TOML: &str = include_str!("fixtures/structured/empty.toml");
const TYPE_COERCION_YAML: &str = include_str!("fixtures/structured/type_coercion.yaml");
const NESTED_ARRAYS_JSON: &str = include_str!("fixtures/structured/nested_arrays.json");
const NUMERIC_EDGE_CASES_JSON: &str = include_str!("fixtures/structured/numeric_edge_cases.json");
const NULL_BOOLEAN_EDGE_CASES_YAML: &str = include_str!("fixtures/structured/null_boolean_edge_cases.yaml");
const DEEP_NESTING_YAML: &str = include_str!("fixtures/structured/deep_nesting.yaml");
const UNICODE_STRINGS_JSON: &str = include_str!("fixtures/structured/unicode_strings.json");
const EMPTY_STRUCTURES_YAML: &str = include_str!("fixtures/structured/empty_structures.yaml");
const TYPE_MISMATCH_COERCION_TOML: &str = include_str!("fixtures/structured/type_mismatch_coercion.toml");
const DATETIME_TOML: &str = include_str!("fixtures/structured/datetime.toml");
const FLOATS_YAML: &str = include_str!("fixtures/structured/floats.yaml");
const FLOATS_TOML: &str = include_str!("fixtures/structured/floats.toml");
const LARGE_CONFIG_YAML: &str = include_str!("fixtures/structured/large_config.yaml");

fn test_logger() -> Logger {
    Logger::root(Discard, o!())
}

/// Helper to run a structured query test
async fn run_structured_test(
    files: Vec<(&str, &str)>,
    expr: &str,
    expected_matches: Vec<&str>,
) {
    let t = TempDir::new("structured_test").unwrap();

    // Write fixture files
    for (filename, content) in files {
        std::fs::write(t.path().join(filename), content).unwrap();
    }

    // Run query
    let mut matches = Vec::new();
    detect::parse_and_run_fs(
        test_logger(),
        t.path(),
        false,
        expr.to_string(),
        detect::RuntimeConfig::default(),
        |p| {
            // Skip the root directory itself - only collect actual files
            if p != t.path() {
                let name = p.file_name().unwrap().to_str().unwrap().to_string();
                matches.push(name);
            }
        },
    )
    .await
    .unwrap();

    // Sort for order-independent comparison
    matches.sort();
    let mut expected = expected_matches.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    expected.sort();

    assert_eq!(
        expected, matches,
        "Query '{}' failed.\nExpected: {:?}\nGot: {:?}",
        expr, expected, matches
    );
}

// ============================================================================
// Group 1: Basic Queries - Simple field access, all formats
// ============================================================================

#[tokio::test]
async fn test_yaml_basic_field_access() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.server.port == 8080",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_yaml_nested_field() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.database.host == \"db.example.com\"",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_yaml_boolean_field() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.server.debug == true",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_json_string_field() {
    run_structured_test(
        vec![("package.json", PACKAGE_JSON)],
        "json:.name == \"my-app\"",
        vec!["package.json"],
    )
    .await;
}

#[tokio::test]
async fn test_json_version_field() {
    run_structured_test(
        vec![("package.json", PACKAGE_JSON)],
        "json:.version == \"1.2.3\"",
        vec!["package.json"],
    )
    .await;
}

#[tokio::test]
async fn test_toml_package_edition() {
    run_structured_test(
        vec![("Cargo.toml", CARGO_TOML)],
        "toml:.package.edition == \"2021\"",
        vec!["Cargo.toml"],
    )
    .await;
}

#[tokio::test]
async fn test_toml_package_name() {
    run_structured_test(
        vec![("Cargo.toml", CARGO_TOML)],
        "toml:.package.name == \"detect\"",
        vec!["Cargo.toml"],
    )
    .await;
}

// ============================================================================
// Group 2: Comparison Operators - >, <, >=, <=, !=
// ============================================================================

#[tokio::test]
async fn test_yaml_greater_than() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.server.port > 8000",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_yaml_less_than() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.server.port < 9000",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_yaml_greater_equal() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.server.port >= 8080",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_yaml_less_equal() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.server.port <= 8080",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_yaml_not_equals() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.server.port != 3000",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_yaml_database_port_comparison() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.database.port > 5000",
        vec!["config.yaml"],
    )
    .await;
}

// ============================================================================
// Group 3: Type Coercion - Int/string mismatches with fallback
// ============================================================================

#[tokio::test]
async fn test_type_coercion_int_field_string_query() {
    // port: 8080 (int) should match "8080" (string query) via coercion
    run_structured_test(
        vec![("type_coercion.yaml", TYPE_COERCION_YAML)],
        "yaml:.port == \"8080\"",
        vec!["type_coercion.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_type_coercion_string_field_int_query() {
    // count: "42" (string) should match 42 (int query) via coercion
    run_structured_test(
        vec![("type_coercion.yaml", TYPE_COERCION_YAML)],
        "yaml:.count == 42",
        vec!["type_coercion.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_type_coercion_version_string() {
    run_structured_test(
        vec![("type_coercion.yaml", TYPE_COERCION_YAML)],
        "yaml:.version == \"1.2.3\"",
        vec!["type_coercion.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_type_coercion_boolean() {
    run_structured_test(
        vec![("type_coercion.yaml", TYPE_COERCION_YAML)],
        "yaml:.flag == true",
        vec!["type_coercion.yaml"],
    )
    .await;
}

// ============================================================================
// Group 4: Advanced Navigation - Recursive descent, wildcards, array indexing
// ============================================================================

#[tokio::test]
async fn test_recursive_descent_port() {
    // Should find both server.port (8080) and database.port (5432)
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:..port > 5000",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_recursive_descent_host() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:..host == \"localhost\"",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_wildcard_array_access() {
    // features[*].enabled should match multiple elements
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.features[*].enabled == true",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_wildcard_array_name() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.features[*].name == \"auth\"",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_array_index_access() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.features[0].name == \"auth\"",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_nested_wildcard() {
    // services[*].ports[*] should flatten nested arrays
    run_structured_test(
        vec![("nested_arrays.json", NESTED_ARRAYS_JSON)],
        "json:.services[*].ports[*] == 8080",
        vec!["nested_arrays.json"],
    )
    .await;
}

#[tokio::test]
async fn test_wildcard_then_field() {
    run_structured_test(
        vec![("nested_arrays.json", NESTED_ARRAYS_JSON)],
        "json:.services[*].name == \"web\"",
        vec!["nested_arrays.json"],
    )
    .await;
}

// ============================================================================
// Group 5: Combined Predicates - Structured + content (tests single file read)
// ============================================================================

#[tokio::test]
async fn test_combined_yaml_and_content() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.server.debug == true AND content ~= \"localhost\"",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_combined_json_or_content() {
    run_structured_test(
        vec![("package.json", PACKAGE_JSON)],
        "json:.version == \"1.2.3\" OR content contains TODO",
        vec!["package.json"],
    )
    .await;
}

#[tokio::test]
async fn test_combined_structured_short_circuit() {
    // If structured predicate is false, content should not be evaluated
    // (though we can't observe this directly, it shouldn't crash)
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.server.port == 9999 AND content contains TODO",
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_combined_content_then_structured() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "content ~= \"database\" AND yaml:.database.port > 5000",
        vec!["config.yaml"],
    )
    .await;
}

// ============================================================================
// Group 6: Multi-Document YAML - OR semantics across documents
// ============================================================================

#[tokio::test]
async fn test_multi_doc_yaml_first_match() {
    // First doc has port: 8080
    run_structured_test(
        vec![("multi_doc.yaml", MULTI_DOC_YAML)],
        "yaml:.port == 8080",
        vec!["multi_doc.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_multi_doc_yaml_second_match() {
    // Second doc has port: 3000
    run_structured_test(
        vec![("multi_doc.yaml", MULTI_DOC_YAML)],
        "yaml:.port == 3000",
        vec!["multi_doc.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_multi_doc_yaml_any_match() {
    // Any doc with enabled: true
    run_structured_test(
        vec![("multi_doc.yaml", MULTI_DOC_YAML)],
        "yaml:.enabled == true",
        vec!["multi_doc.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_multi_doc_yaml_no_match() {
    // No doc has port: 9999
    run_structured_test(
        vec![("multi_doc.yaml", MULTI_DOC_YAML)],
        "yaml:.port == 9999",
        vec![],
    )
    .await;
}

// ============================================================================
// Group 7: Parse Errors - Invalid syntax handled gracefully
// ============================================================================

#[tokio::test]
async fn test_invalid_yaml_no_crash() {
    // Invalid YAML should not crash, just return no matches
    run_structured_test(
        vec![("invalid.yaml", INVALID_YAML)],
        "yaml:.field == \"value\"",
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_invalid_json_no_crash() {
    run_structured_test(
        vec![("invalid.json", INVALID_JSON)],
        "json:.field == \"value\"",
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_invalid_toml_no_crash() {
    run_structured_test(
        vec![("invalid.toml", INVALID_TOML)],
        "toml:.field == \"value\"",
        vec![],
    )
    .await;
}

// ============================================================================
// Group 8: Edge Cases - Empty files, missing fields, wrong format
// ============================================================================

#[tokio::test]
async fn test_empty_yaml_file() {
    run_structured_test(
        vec![("empty.yaml", EMPTY_YAML)],
        "yaml:.field == \"value\"",
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_empty_json_file() {
    run_structured_test(
        vec![("empty.json", EMPTY_JSON)],
        "json:.field == \"value\"",
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_empty_toml_file() {
    run_structured_test(
        vec![("empty.toml", EMPTY_TOML)],
        "toml:.field == \"value\"",
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_missing_nested_field() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.nonexistent.nested.field == 1",
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_wrong_format_yaml_on_json() {
    // YAML selector on JSON file should not match
    run_structured_test(
        vec![("package.json", PACKAGE_JSON)],
        "yaml:.name == \"my-app\"",
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_wrong_format_json_on_yaml() {
    // JSON selector on YAML file should not match
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "json:.server.port == 8080",
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_multiple_formats_correct_filtering() {
    // Should only match the correct format
    run_structured_test(
        vec![
            ("config.yaml", CONFIG_YAML),
            ("package.json", PACKAGE_JSON),
            ("Cargo.toml", CARGO_TOML),
        ],
        "yaml:.server.port == 8080",
        vec!["config.yaml"],
    )
    .await;
}

// ============================================================================
// Group 9: Boolean Logic with Structured
// ============================================================================

#[tokio::test]
async fn test_not_structured_predicate() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "NOT yaml:.server.port == 3000",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_or_multiple_values() {
    run_structured_test(
        vec![("multi_doc.yaml", MULTI_DOC_YAML)],
        "yaml:.port == 8080 OR yaml:.port == 3000",
        vec!["multi_doc.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_and_multiple_conditions() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.server.port == 8080 AND yaml:.server.debug == true",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_complex_boolean_logic() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "(yaml:.server.port > 8000 AND yaml:.server.host == \"localhost\") OR yaml:.database.port > 5000",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_mixed_formats_or_logic() {
    // Match files with YAML port=8080 OR JSON version
    run_structured_test(
        vec![
            ("config.yaml", CONFIG_YAML),
            ("package.json", PACKAGE_JSON),
        ],
        "yaml:.server.port == 8080 OR json:.version == \"1.2.3\"",
        vec!["config.yaml", "package.json"],
    )
    .await;
}

// ============================================================================
// Group 10: Non-UTF8 Handling
// ============================================================================

#[tokio::test]
async fn test_binary_yaml_file_no_crash() {
    // Binary file with .yaml extension should not crash
    // Structured predicate should return false gracefully
    let binary_content = include_bytes!("fixtures/structured/binary.yaml");

    let t = TempDir::new("binary_test").unwrap();
    std::fs::write(t.path().join("binary.yaml"), binary_content).unwrap();

    let mut matches = Vec::new();
    detect::parse_and_run_fs(
        test_logger(),
        t.path(),
        false,
        "yaml:.field == \"value\"".to_string(),
        detect::RuntimeConfig::default(),
        |p| {
            matches.push(p.file_name().unwrap().to_str().unwrap().to_string());
        },
    )
    .await
    .unwrap();

    assert_eq!(matches.len(), 0, "Binary file should not match YAML selector");
}

#[tokio::test]
async fn test_binary_file_content_predicate_works() {
    // Even if structured predicate fails, content predicate on bytes should work
    let binary_content = include_bytes!("fixtures/structured/binary.yaml");

    let t = TempDir::new("binary_test").unwrap();
    std::fs::write(t.path().join("binary.yaml"), binary_content).unwrap();

    let mut matches = Vec::new();
    detect::parse_and_run_fs(
        test_logger(),
        t.path(),
        false,
        "content ~= \"binary\"".to_string(),
        detect::RuntimeConfig::default(),
        |p| {
            matches.push(p.file_name().unwrap().to_str().unwrap().to_string());
        },
    )
    .await
    .unwrap();

    assert_eq!(matches, vec!["binary.yaml"], "Content predicate should work on binary files");
}

// ============================================================================
// Regex String Matchers
// ============================================================================

#[tokio::test]
async fn test_yaml_string_regex() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.server.host ~= \"local.*\"",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_json_string_regex() {
    run_structured_test(
        vec![("package.json", PACKAGE_JSON)],
        "json:.name ~= \"^my-\"",
        vec!["package.json"],
    )
    .await;
}

#[tokio::test]
async fn test_yaml_string_contains() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.database.host contains \"example\"",
        vec!["config.yaml"],
    )
    .await;
}

// ============================================================================
// Group 11: NotEquals with Type Coercion
// ============================================================================

#[tokio::test]
async fn test_not_equals_type_coercion_toml() {
    run_structured_test(
        vec![("coercion.toml", TYPE_MISMATCH_COERCION_TOML)],
        "toml:.port != \"8080\"",
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_not_equals_type_coercion_json() {
    run_structured_test(
        vec![("numeric.json", NUMERIC_EDGE_CASES_JSON)],
        "json:.int_value != \"42\"",
        vec![],
    )
    .await;
}

// ============================================================================
// Group 12: JSON Float Comparisons
// ============================================================================

#[tokio::test]
async fn test_json_float_comparisons() {
    run_structured_test(
        vec![("numeric.json", NUMERIC_EDGE_CASES_JSON)],
        "json:.float_value > 1",
        vec!["numeric.json"],
    )
    .await;

    run_structured_test(
        vec![("numeric.json", NUMERIC_EDGE_CASES_JSON)],
        "json:.float_value < 2",
        vec!["numeric.json"],
    )
    .await;

    run_structured_test(
        vec![("numeric.json", NUMERIC_EDGE_CASES_JSON)],
        "json:.float_value == 1.5",
        vec!["numeric.json"],
    )
    .await;

    run_structured_test(
        vec![("numeric.json", NUMERIC_EDGE_CASES_JSON)],
        "json:.negative_float < -2",
        vec!["numeric.json"],
    )
    .await;
}

// ============================================================================
// Group 13: Numeric Edge Cases
// ============================================================================

#[tokio::test]
async fn test_numeric_edge_cases() {
    run_structured_test(
        vec![("numeric.json", NUMERIC_EDGE_CASES_JSON)],
        "json:.negative_int < 0",
        vec!["numeric.json"],
    )
    .await;

    run_structured_test(
        vec![("numeric.json", NUMERIC_EDGE_CASES_JSON)],
        "json:.zero == 0",
        vec!["numeric.json"],
    )
    .await;

    run_structured_test(
        vec![("numeric.json", NUMERIC_EDGE_CASES_JSON)],
        "json:.zero > -1",
        vec!["numeric.json"],
    )
    .await;

    run_structured_test(
        vec![("numeric.json", NUMERIC_EDGE_CASES_JSON)],
        "json:.large_int > 9223372036854775805",
        vec!["numeric.json"],
    )
    .await;
}

#[tokio::test]
async fn test_numeric_type_coercion() {
    run_structured_test(
        vec![("numeric.json", NUMERIC_EDGE_CASES_JSON)],
        "json:.int_value == \"42\"",
        vec!["numeric.json"],
    )
    .await;

    run_structured_test(
        vec![("numeric.json", NUMERIC_EDGE_CASES_JSON)],
        "json:.float_value == \"1.5\"",
        vec!["numeric.json"],
    )
    .await;
}

// ============================================================================
// Group 14: null/Boolean Edge Cases
// ============================================================================

#[tokio::test]
async fn test_null_equals_null() {
    run_structured_test(
        vec![("null_bool.yaml", NULL_BOOLEAN_EDGE_CASES_YAML)],
        "yaml:.null_value == null",
        vec!["null_bool.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_null_vs_null_string() {
    run_structured_test(
        vec![("null_bool.yaml", NULL_BOOLEAN_EDGE_CASES_YAML)],
        "yaml:.null_string == \"null\"",
        vec!["null_bool.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_bool_true_equals() {
    run_structured_test(
        vec![("null_bool.yaml", NULL_BOOLEAN_EDGE_CASES_YAML)],
        "yaml:.bool_true == true",
        vec!["null_bool.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_bool_false_equals() {
    run_structured_test(
        vec![("null_bool.yaml", NULL_BOOLEAN_EDGE_CASES_YAML)],
        "yaml:.bool_false == false",
        vec!["null_bool.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_bool_vs_string_true() {
    run_structured_test(
        vec![("null_bool.yaml", NULL_BOOLEAN_EDGE_CASES_YAML)],
        "yaml:.bool_true == \"true\"",
        vec!["null_bool.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_string_true_equals() {
    run_structured_test(
        vec![("null_bool.yaml", NULL_BOOLEAN_EDGE_CASES_YAML)],
        "yaml:.string_true == \"true\"",
        vec!["null_bool.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_empty_string_vs_null() {
    run_structured_test(
        vec![("null_bool.yaml", NULL_BOOLEAN_EDGE_CASES_YAML)],
        "yaml:.empty_string == \"\"",
        vec!["null_bool.yaml"],
    )
    .await;
}

// ============================================================================
// Group 15: Empty Structures
// ============================================================================

#[tokio::test]
async fn test_empty_array_comparison() {
    run_structured_test(
        vec![("empty.yaml", EMPTY_STRUCTURES_YAML)],
        "yaml:.empty_array == \"[]\"",
        vec!["empty.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_empty_object_comparison() {
    run_structured_test(
        vec![("empty.yaml", EMPTY_STRUCTURES_YAML)],
        "yaml:.empty_object == \"{}\"",
        vec!["empty.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_nested_empty_array() {
    run_structured_test(
        vec![("empty.yaml", EMPTY_STRUCTURES_YAML)],
        "yaml:.nested_empty.inner_array == \"[]\"",
        vec!["empty.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_wildcard_on_empty_array() {
    run_structured_test(
        vec![("empty.yaml", EMPTY_STRUCTURES_YAML)],
        "yaml:.empty_array[*] == \"anything\"",
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_index_on_empty_array() {
    run_structured_test(
        vec![("empty.yaml", EMPTY_STRUCTURES_YAML)],
        "yaml:.empty_array[0] == \"anything\"",
        vec![],
    )
    .await;
}

// ============================================================================
// Group 16: Navigation Edge Cases
// ============================================================================

#[tokio::test]
async fn test_deep_nesting_six_levels() {
    run_structured_test(
        vec![("deep.yaml", DEEP_NESTING_YAML)],
        "yaml:.a.b.c.d.e.f == \"deeply_nested_value\"",
        vec!["deep.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_deep_nesting_numeric() {
    run_structured_test(
        vec![("deep.yaml", DEEP_NESTING_YAML)],
        "yaml:.a.b.c.d.e.g == 123",
        vec!["deep.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_index_on_string_field() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.server.host[0] == \"l\"",
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_recursive_descent_through_deep_nesting() {
    run_structured_test(
        vec![("deep.yaml", DEEP_NESTING_YAML)],
        "yaml:..f == \"deeply_nested_value\"",
        vec!["deep.yaml"],
    )
    .await;
}

// ============================================================================
// Group 17: String/Unicode Edge Cases
// ============================================================================

#[tokio::test]
async fn test_emoji_field_value() {
    run_structured_test(
        vec![("unicode.json", UNICODE_STRINGS_JSON)],
        "json:.emoji_field == \"🚀\"",
        vec!["unicode.json"],
    )
    .await;
}

#[tokio::test]
async fn test_emoji_in_string_value() {
    run_structured_test(
        vec![("unicode.json", UNICODE_STRINGS_JSON)],
        "json:.emoji_value contains \"🎉\"",
        vec!["unicode.json"],
    )
    .await;
}

#[tokio::test]
async fn test_unicode_characters() {
    run_structured_test(
        vec![("unicode.json", UNICODE_STRINGS_JSON)],
        "json:.unicode_chars contains \"中文\"",
        vec!["unicode.json"],
    )
    .await;
}

#[tokio::test]
async fn test_multiline_string() {
    run_structured_test(
        vec![("unicode.json", UNICODE_STRINGS_JSON)],
        "json:.multiline contains \"line2\"",
        vec!["unicode.json"],
    )
    .await;
}

#[tokio::test]
async fn test_very_long_string() {
    run_structured_test(
        vec![("unicode.json", UNICODE_STRINGS_JSON)],
        "json:.long_string contains \"Lorem ipsum\"",
        vec!["unicode.json"],
    )
    .await;
}

// ============================================================================
// Group 18: Type Coercion with String Matchers
// ============================================================================

#[tokio::test]
async fn test_regex_on_coerced_int() {
    run_structured_test(
        vec![("coercion.toml", TYPE_MISMATCH_COERCION_TOML)],
        "toml:.port ~= \"80.*\"",
        vec!["coercion.toml"],
    )
    .await;
}

#[tokio::test]
async fn test_contains_on_coerced_bool() {
    run_structured_test(
        vec![("coercion.toml", TYPE_MISMATCH_COERCION_TOML)],
        "toml:.enabled contains \"true\"",
        vec!["coercion.toml"],
    )
    .await;
}

#[tokio::test]
async fn test_regex_on_coerced_numeric_array() {
    run_structured_test(
        vec![("numeric.json", NUMERIC_EDGE_CASES_JSON)],
        "json:.mixed_array[*] ~= \"^[0-9]\"",
        vec!["numeric.json"],
    )
    .await;
}

#[tokio::test]
async fn test_not_equals_with_string_matcher_coercion() {
    run_structured_test(
        vec![("coercion.toml", TYPE_MISMATCH_COERCION_TOML)],
        "toml:.count != \"41\"",
        vec!["coercion.toml"],
    )
    .await;
}

// ============================================================================
// Group 19: Array Bounds Edge Cases
// ============================================================================

#[tokio::test]
async fn test_out_of_bounds_positive_index() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.features[999].name == \"anything\"",
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_out_of_bounds_on_empty_array() {
    run_structured_test(
        vec![("empty.yaml", EMPTY_STRUCTURES_YAML)],
        "yaml:.empty_array[0] == \"anything\"",
        vec![],
    )
    .await;
}

#[tokio::test]
async fn test_out_of_bounds_json() {
    run_structured_test(
        vec![("package.json", PACKAGE_JSON)],
        "json:.keywords[999] == \"anything\"",
        vec![],
    )
    .await;
}

// ============================================================================
// Group 20: TOML Datetime Support
// ============================================================================

#[tokio::test]
async fn test_toml_datetime_equality() {
    run_structured_test(
        vec![("datetime.toml", DATETIME_TOML)],
        "toml:.timestamp == \"2024-01-15T10:30:00Z\"",
        vec!["datetime.toml"],
    )
    .await;
}

#[tokio::test]
async fn test_toml_datetime_comparison_greater() {
    run_structured_test(
        vec![("datetime.toml", DATETIME_TOML)],
        "toml:.timestamp > \"2024-01-01T00:00:00Z\"",
        vec!["datetime.toml"],
    )
    .await;
}

#[tokio::test]
async fn test_toml_datetime_comparison_less() {
    run_structured_test(
        vec![("datetime.toml", DATETIME_TOML)],
        "toml:.created_date < \"2024-12-31T23:59:59Z\"",
        vec!["datetime.toml"],
    )
    .await;
}

#[tokio::test]
async fn test_toml_datetime_nested_field() {
    run_structured_test(
        vec![("datetime.toml", DATETIME_TOML)],
        "toml:.event.start == \"2024-06-15T09:00:00Z\"",
        vec!["datetime.toml"],
    )
    .await;
}

// ============================================================================
// Group 21: Document Caching (multiple structured predicates)
// ============================================================================

#[tokio::test]
async fn test_multiple_yaml_selectors_same_file() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.server.port == 8080 AND yaml:.server.host == \"localhost\"",
        vec!["config.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_multiple_json_selectors_same_file() {
    run_structured_test(
        vec![("package.json", PACKAGE_JSON)],
        "json:.name == \"my-app\" AND json:.version == \"1.2.3\"",
        vec!["package.json"],
    )
    .await;
}

#[tokio::test]
async fn test_multiple_toml_selectors_same_file() {
    run_structured_test(
        vec![("Cargo.toml", CARGO_TOML)],
        "toml:.package.name == \"detect\" AND toml:.package.edition == \"2021\"",
        vec!["Cargo.toml"],
    )
    .await;
}

#[tokio::test]
async fn test_three_yaml_selectors_different_paths() {
    run_structured_test(
        vec![("config.yaml", CONFIG_YAML)],
        "yaml:.server.port > 8000 AND yaml:.database.port > 5000 AND yaml:.server.debug == true",
        vec!["config.yaml"],
    )
    .await;
}

// ============================================================================
// Group 22: YAML/TOML Float Comparisons
// ============================================================================

#[tokio::test]
async fn test_yaml_float_equality() {
    run_structured_test(
        vec![("floats.yaml", FLOATS_YAML)],
        "yaml:.value == 1.5",
        vec!["floats.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_yaml_float_greater_than() {
    run_structured_test(
        vec![("floats.yaml", FLOATS_YAML)],
        "yaml:.value > 1.0",
        vec!["floats.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_yaml_float_less_than() {
    run_structured_test(
        vec![("floats.yaml", FLOATS_YAML)],
        "yaml:.value < 2.0",
        vec!["floats.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_yaml_negative_float() {
    run_structured_test(
        vec![("floats.yaml", FLOATS_YAML)],
        "yaml:.negative < 0",
        vec!["floats.yaml"],
    )
    .await;
}

#[tokio::test]
async fn test_toml_float_equality() {
    run_structured_test(
        vec![("floats.toml", FLOATS_TOML)],
        "toml:.value == 2.5",
        vec!["floats.toml"],
    )
    .await;
}

#[tokio::test]
async fn test_toml_float_greater_than() {
    run_structured_test(
        vec![("floats.toml", FLOATS_TOML)],
        "toml:.value > 2.0",
        vec!["floats.toml"],
    )
    .await;
}

#[tokio::test]
async fn test_toml_float_less_than() {
    run_structured_test(
        vec![("floats.toml", FLOATS_TOML)],
        "toml:.value < 3.0",
        vec!["floats.toml"],
    )
    .await;
}

#[tokio::test]
async fn test_toml_negative_float() {
    run_structured_test(
        vec![("floats.toml", FLOATS_TOML)],
        "toml:.negative < 0",
        vec!["floats.toml"],
    )
    .await;
}

// ============================================================================
// Group 23: Synthetic Size Predicate
// ============================================================================

#[tokio::test]
async fn test_structured_skipped_when_file_exceeds_size_limit() {
    let t = TempDir::new("structured_test").unwrap();

    // Write file that would match if parsed
    std::fs::write(t.path().join("large.yaml"), LARGE_CONFIG_YAML).unwrap();

    // Use artificially small size limit (50 bytes)
    // large_config.yaml is >200 bytes, so it should be skipped
    let config = detect::RuntimeConfig {
        max_structured_size: 50,
    };

    // Query that would match if file were parsed
    let mut matches = Vec::new();
    detect::parse_and_run_fs(
        test_logger(),
        t.path(),
        false,
        "yaml:.server.port == 9999".to_string(),
        config,
        |p| {
            if p != t.path() {
                let name = p.file_name().unwrap().to_str().unwrap().to_string();
                matches.push(name);
            }
        },
    )
    .await
    .unwrap();

    // File should NOT match because it exceeds size limit
    assert_eq!(matches.len(), 0, "File should be skipped due to size limit");
}
