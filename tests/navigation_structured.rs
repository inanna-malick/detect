//! Comprehensive tests for structured data navigation (YAML/JSON/TOML)
//!
//! Tests the iterative, zero-clone traversal of parsed documents.

use detect::eval::structured::{navigate_json, navigate_toml, navigate_yaml};
use detect::parser::structured_path::parse_path;

// ============================================================================
// YAML Test Helpers
// ============================================================================

/// Helper to construct YAML integer
fn yaml_int(i: i64) -> yaml_rust::Yaml {
    yaml_rust::Yaml::Integer(i)
}

/// Helper to construct YAML string
fn yaml_str(s: &str) -> yaml_rust::Yaml {
    yaml_rust::Yaml::String(s.to_string())
}

/// Helper to construct YAML boolean
fn yaml_bool(b: bool) -> yaml_rust::Yaml {
    yaml_rust::Yaml::Boolean(b)
}

/// Helper to construct YAML array
fn yaml_array(items: Vec<yaml_rust::Yaml>) -> yaml_rust::Yaml {
    yaml_rust::Yaml::Array(items)
}

/// Helper to construct YAML hash/object
fn yaml_hash(pairs: Vec<(&str, yaml_rust::Yaml)>) -> yaml_rust::Yaml {
    use yaml_rust::yaml::Hash;
    let mut map = Hash::new();
    for (k, v) in pairs {
        map.insert(yaml_rust::Yaml::String(k.to_string()), v);
    }
    yaml_rust::Yaml::Hash(map)
}

/// YAML navigation test case
struct YamlNavCase {
    name: &'static str,
    document: yaml_rust::Yaml,
    path: &'static str,
    expected: Vec<yaml_rust::Yaml>,
}

/// Check if two slices contain the same elements (order-insensitive)
fn yaml_sets_equal(a: &[&yaml_rust::Yaml], b: &[yaml_rust::Yaml]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    // Check every element in 'a' exists in 'b'
    for item_a in a {
        if !b.iter().any(|item_b| item_a == &item_b) {
            return false;
        }
    }

    // Check every element in 'b' exists in 'a'
    for item_b in b {
        if !a.contains(&item_b) {
            return false;
        }
    }

    true
}

/// Run a batch of YAML navigation test cases
fn run_yaml_tests(test_cases: &[YamlNavCase]) {
    for case in test_cases {
        let path = parse_path(case.path).unwrap_or_else(|e| {
            panic!(
                "Test '{}': Failed to parse path '{}': {:?}",
                case.name, case.path, e
            )
        });

        let results = navigate_yaml(&case.document, &path);

        // Order-insensitive comparison
        if !yaml_sets_equal(&results, &case.expected) {
            panic!(
                "Test '{}' failed:\n  Path: {}\n  Expected {} results: {:?}\n  Got {} results: {:?}",
                case.name,
                case.path,
                case.expected.len(),
                case.expected,
                results.len(),
                results
            );
        }
    }
}

// ============================================================================
// JSON Test Helpers
// ============================================================================

/// Helper to construct JSON integer
fn json_int(i: i64) -> serde_json::Value {
    serde_json::Value::Number(serde_json::Number::from(i))
}

/// Helper to construct JSON string
fn json_str(s: &str) -> serde_json::Value {
    serde_json::Value::String(s.to_string())
}

/// Helper to construct JSON array
fn json_array(items: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::Value::Array(items)
}

/// Helper to construct JSON object
fn json_object(pairs: Vec<(&str, serde_json::Value)>) -> serde_json::Value {
    let map: serde_json::Map<String, serde_json::Value> =
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    serde_json::Value::Object(map)
}

/// JSON navigation test case
struct JsonNavCase {
    name: &'static str,
    document: serde_json::Value,
    path: &'static str,
    expected: Vec<serde_json::Value>,
}

/// Check if two slices contain the same JSON elements (order-insensitive)
fn json_sets_equal(a: &[&serde_json::Value], b: &[serde_json::Value]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    for item_a in a {
        if !b.iter().any(|item_b| item_a == &item_b) {
            return false;
        }
    }

    for item_b in b {
        if !a.contains(&item_b) {
            return false;
        }
    }

    true
}

/// Run a batch of JSON navigation test cases
fn run_json_tests(test_cases: &[JsonNavCase]) {
    for case in test_cases {
        let path = parse_path(case.path).unwrap_or_else(|e| {
            panic!(
                "Test '{}': Failed to parse path '{}': {:?}",
                case.name, case.path, e
            )
        });

        let results = navigate_json(&case.document, &path);

        if !json_sets_equal(&results, &case.expected) {
            panic!(
                "Test '{}' failed:\n  Path: {}\n  Expected {} results: {:?}\n  Got {} results: {:?}",
                case.name,
                case.path,
                case.expected.len(),
                case.expected,
                results.len(),
                results
            );
        }
    }
}

// ============================================================================
// TOML Test Helpers
// ============================================================================

/// Helper to construct TOML integer
fn toml_int(i: i64) -> toml::Value {
    toml::Value::Integer(i)
}

/// Helper to construct TOML string
fn toml_str(s: &str) -> toml::Value {
    toml::Value::String(s.to_string())
}

/// Helper to construct TOML array
fn toml_array(items: Vec<toml::Value>) -> toml::Value {
    toml::Value::Array(items)
}

/// Helper to construct TOML table
fn toml_table(pairs: Vec<(&str, toml::Value)>) -> toml::Value {
    let map: toml::map::Map<String, toml::Value> =
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    toml::Value::Table(map)
}

/// TOML navigation test case
struct TomlNavCase {
    name: &'static str,
    document: toml::Value,
    path: &'static str,
    expected: Vec<toml::Value>,
}

/// Check if two slices contain the same TOML elements (order-insensitive)
fn toml_sets_equal(a: &[&toml::Value], b: &[toml::Value]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    for item_a in a {
        if !b.iter().any(|item_b| item_a == &item_b) {
            return false;
        }
    }

    for item_b in b {
        if !a.contains(&item_b) {
            return false;
        }
    }

    true
}

/// Run a batch of TOML navigation test cases
fn run_toml_tests(test_cases: &[TomlNavCase]) {
    for case in test_cases {
        let path = parse_path(case.path).unwrap_or_else(|e| {
            panic!(
                "Test '{}': Failed to parse path '{}': {:?}",
                case.name, case.path, e
            )
        });

        let results = navigate_toml(&case.document, &path);

        if !toml_sets_equal(&results, &case.expected) {
            panic!(
                "Test '{}' failed:\n  Path: {}\n  Expected {} results: {:?}\n  Got {} results: {:?}",
                case.name,
                case.path,
                case.expected.len(),
                case.expected,
                results.len(),
                results
            );
        }
    }
}

// ============================================================================
// YAML Navigation Tests
// ============================================================================

#[test]
fn test_yaml_basic_navigation() {
    let test_cases = vec![
        YamlNavCase {
            name: "simple key",
            document: yaml_hash(vec![("port", yaml_int(8080))]),
            path: ".port",
            expected: vec![yaml_int(8080)],
        },
        YamlNavCase {
            name: "nested keys",
            document: yaml_hash(vec![("server", yaml_hash(vec![("port", yaml_int(8080))]))]),
            path: ".server.port",
            expected: vec![yaml_int(8080)],
        },
        YamlNavCase {
            name: "deep nesting",
            document: yaml_hash(vec![(
                "a",
                yaml_hash(vec![(
                    "b",
                    yaml_hash(vec![("c", yaml_hash(vec![("d", yaml_str("deep"))]))]),
                )]),
            )]),
            path: ".a.b.c.d",
            expected: vec![yaml_str("deep")],
        },
        YamlNavCase {
            name: "missing key returns empty",
            document: yaml_hash(vec![("port", yaml_int(8080))]),
            path: ".missing",
            expected: vec![],
        },
        YamlNavCase {
            name: "missing nested key returns empty",
            document: yaml_hash(vec![("server", yaml_hash(vec![("port", yaml_int(8080))]))]),
            path: ".server.missing",
            expected: vec![],
        },
    ];

    run_yaml_tests(&test_cases);
}

#[test]
fn test_yaml_array_navigation() {
    let test_cases = vec![
        YamlNavCase {
            name: "array index 0",
            document: yaml_hash(vec![(
                "items",
                yaml_array(vec![
                    yaml_str("first"),
                    yaml_str("second"),
                    yaml_str("third"),
                ]),
            )]),
            path: ".items[0]",
            expected: vec![yaml_str("first")],
        },
        YamlNavCase {
            name: "array index middle",
            document: yaml_hash(vec![(
                "items",
                yaml_array(vec![
                    yaml_str("first"),
                    yaml_str("second"),
                    yaml_str("third"),
                ]),
            )]),
            path: ".items[1]",
            expected: vec![yaml_str("second")],
        },
        YamlNavCase {
            name: "array index last",
            document: yaml_hash(vec![(
                "items",
                yaml_array(vec![
                    yaml_str("first"),
                    yaml_str("second"),
                    yaml_str("third"),
                ]),
            )]),
            path: ".items[2]",
            expected: vec![yaml_str("third")],
        },
        YamlNavCase {
            name: "array out of bounds returns empty",
            document: yaml_hash(vec![("items", yaml_array(vec![yaml_str("first")]))]),
            path: ".items[999]",
            expected: vec![],
        },
        YamlNavCase {
            name: "chained array access",
            document: yaml_hash(vec![(
                "matrix",
                yaml_array(vec![
                    yaml_array(vec![yaml_int(1), yaml_int(2)]),
                    yaml_array(vec![yaml_int(3), yaml_int(4)]),
                ]),
            )]),
            path: ".matrix[1][0]",
            expected: vec![yaml_int(3)],
        },
        YamlNavCase {
            name: "array then key",
            document: yaml_hash(vec![(
                "users",
                yaml_array(vec![
                    yaml_hash(vec![("name", yaml_str("alice"))]),
                    yaml_hash(vec![("name", yaml_str("bob"))]),
                ]),
            )]),
            path: ".users[1].name",
            expected: vec![yaml_str("bob")],
        },
    ];

    run_yaml_tests(&test_cases);
}

#[test]
fn test_yaml_wildcard_navigation() {
    let test_cases = vec![
        YamlNavCase {
            name: "wildcard all array elements",
            document: yaml_hash(vec![(
                "items",
                yaml_array(vec![yaml_int(1), yaml_int(2), yaml_int(3)]),
            )]),
            path: ".items[*]",
            expected: vec![yaml_int(1), yaml_int(2), yaml_int(3)],
        },
        YamlNavCase {
            name: "wildcard with mixed types",
            document: yaml_hash(vec![(
                "mixed",
                yaml_array(vec![yaml_int(42), yaml_str("hello"), yaml_bool(true)]),
            )]),
            path: ".mixed[*]",
            expected: vec![yaml_int(42), yaml_str("hello"), yaml_bool(true)],
        },
        YamlNavCase {
            name: "wildcard then field",
            document: yaml_hash(vec![(
                "users",
                yaml_array(vec![
                    yaml_hash(vec![("id", yaml_int(1))]),
                    yaml_hash(vec![("id", yaml_int(2))]),
                    yaml_hash(vec![("id", yaml_int(3))]),
                ]),
            )]),
            path: ".users[*].id",
            expected: vec![yaml_int(1), yaml_int(2), yaml_int(3)],
        },
        YamlNavCase {
            name: "wildcard on empty array",
            document: yaml_hash(vec![("empty", yaml_array(vec![]))]),
            path: ".empty[*]",
            expected: vec![],
        },
    ];

    run_yaml_tests(&test_cases);
}

#[test]
fn test_yaml_recursive_descent() {
    let test_cases = vec![
        YamlNavCase {
            name: "recursive finds single occurrence",
            document: yaml_hash(vec![(
                "config",
                yaml_hash(vec![(
                    "database",
                    yaml_hash(vec![("host", yaml_str("localhost"))]),
                )]),
            )]),
            path: "..host",
            expected: vec![yaml_str("localhost")],
        },
        YamlNavCase {
            name: "recursive finds multiple occurrences",
            document: yaml_hash(vec![
                ("db1", yaml_hash(vec![("host", yaml_str("server1"))])),
                ("db2", yaml_hash(vec![("host", yaml_str("server2"))])),
            ]),
            path: "..host",
            expected: vec![yaml_str("server1"), yaml_str("server2")],
        },
        YamlNavCase {
            name: "recursive descent then field",
            document: yaml_hash(vec![
                (
                    "app1",
                    yaml_hash(vec![(
                        "database",
                        yaml_hash(vec![("host", yaml_str("db1"))]),
                    )]),
                ),
                (
                    "app2",
                    yaml_hash(vec![(
                        "database",
                        yaml_hash(vec![("host", yaml_str("db2"))]),
                    )]),
                ),
            ]),
            path: "..database.host",
            expected: vec![yaml_str("db1"), yaml_str("db2")],
        },
        YamlNavCase {
            name: "recursive in nested arrays",
            document: yaml_hash(vec![(
                "items",
                yaml_array(vec![
                    yaml_hash(vec![("id", yaml_int(1))]),
                    yaml_hash(vec![("id", yaml_int(2))]),
                ]),
            )]),
            path: "..id",
            expected: vec![yaml_int(1), yaml_int(2)],
        },
        YamlNavCase {
            name: "recursive descent with wildcard",
            document: yaml_hash(vec![(
                "services",
                yaml_array(vec![
                    yaml_hash(vec![(
                        "ports",
                        yaml_array(vec![yaml_int(8080), yaml_int(8081)]),
                    )]),
                    yaml_hash(vec![("ports", yaml_array(vec![yaml_int(9090)]))]),
                ]),
            )]),
            path: "..ports[*]",
            expected: vec![yaml_int(8080), yaml_int(8081), yaml_int(9090)],
        },
    ];

    run_yaml_tests(&test_cases);
}

// ============================================================================
// JSON Navigation Tests
// ============================================================================

#[test]
fn test_json_basic_navigation() {
    let test_cases = vec![
        JsonNavCase {
            name: "simple key",
            document: json_object(vec![("port", json_int(8080))]),
            path: ".port",
            expected: vec![json_int(8080)],
        },
        JsonNavCase {
            name: "nested keys",
            document: json_object(vec![(
                "server",
                json_object(vec![("port", json_int(8080))]),
            )]),
            path: ".server.port",
            expected: vec![json_int(8080)],
        },
        JsonNavCase {
            name: "array index",
            document: json_object(vec![(
                "items",
                json_array(vec![json_str("a"), json_str("b")]),
            )]),
            path: ".items[1]",
            expected: vec![json_str("b")],
        },
        JsonNavCase {
            name: "wildcard",
            document: json_object(vec![("nums", json_array(vec![json_int(1), json_int(2)]))]),
            path: ".nums[*]",
            expected: vec![json_int(1), json_int(2)],
        },
    ];

    run_json_tests(&test_cases);
}

// ============================================================================
// TOML Navigation Tests
// ============================================================================

#[test]
fn test_toml_basic_navigation() {
    let test_cases = vec![
        TomlNavCase {
            name: "simple key",
            document: toml_table(vec![("port", toml_int(8080))]),
            path: ".port",
            expected: vec![toml_int(8080)],
        },
        TomlNavCase {
            name: "nested table",
            document: toml_table(vec![("server", toml_table(vec![("port", toml_int(8080))]))]),
            path: ".server.port",
            expected: vec![toml_int(8080)],
        },
        TomlNavCase {
            name: "array index",
            document: toml_table(vec![(
                "items",
                toml_array(vec![toml_str("a"), toml_str("b")]),
            )]),
            path: ".items[1]",
            expected: vec![toml_str("b")],
        },
        TomlNavCase {
            name: "wildcard",
            document: toml_table(vec![("nums", toml_array(vec![toml_int(1), toml_int(2)]))]),
            path: ".nums[*]",
            expected: vec![toml_int(1), toml_int(2)],
        },
    ];

    run_toml_tests(&test_cases);
}
