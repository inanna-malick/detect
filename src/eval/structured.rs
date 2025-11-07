//! Navigation and evaluation logic for structured data selectors (YAML/JSON/TOML)
//!
//! Provides zero-allocation, iterative traversal of parsed documents using path expressions.
//! Also provides value comparison with type coercion fallback.

use crate::parser::structured_path::PathComponent;
use crate::parser::typed::StructuredOperator;
use crate::predicate::StringMatcher;

/// Navigate a YAML document using a path expression
///
/// Returns all matching values (may be multiple due to wildcards or recursive descent).
/// Uses iterative work queue algorithm - no recursion, no clones.

pub fn navigate_yaml<'a>(
    root: &'a yaml_rust2::Yaml,
    path: &[PathComponent],
) -> Vec<&'a yaml_rust2::Yaml> {
    let mut current_values = vec![root];

    for component in path {
        if current_values.is_empty() {
            return Vec::new(); // Early exit - no matches
        }

        let mut next_values = Vec::new();

        match component {
            PathComponent::Key(key) => {
                for value in current_values {
                    // yaml-rust implements Index trait: yaml["key"] returns &Yaml or BadValue
                    let field = &value[key.as_str()];
                    if !field.is_badvalue() {
                        next_values.push(field);
                    }
                }
            }

            PathComponent::Index(idx) => {
                for value in current_values {
                    // yaml-rust implements Index<usize>: yaml[0] returns &Yaml or BadValue
                    if let yaml_rust2::Yaml::Array(arr) = value {
                        if let Some(element) = arr.get(*idx) {
                            next_values.push(element);
                        }
                    }
                }
            }

            PathComponent::WildcardIndex => {
                for value in current_values {
                    if let yaml_rust2::Yaml::Array(arr) = value {
                        next_values.extend(arr.iter());
                    }
                }
            }

            PathComponent::RecursiveKey(key) => {
                // Collect all occurrences of this key at any depth in the subtree
                for value in current_values {
                    collect_recursive_yaml_key(value, key, &mut next_values);
                }
            }
        }

        current_values = next_values;
    }

    current_values
}

/// Iteratively collect all values for a given key in the YAML subtree
fn collect_recursive_yaml_key<'a>(
    root: &'a yaml_rust2::Yaml,
    key: &str,
    results: &mut Vec<&'a yaml_rust2::Yaml>,
) {
    let mut work_queue = vec![root];

    while let Some(node) = work_queue.pop() {
        match node {
            yaml_rust2::Yaml::Hash(map) => {
                for (k, v) in map {
                    if let yaml_rust2::Yaml::String(key_str) = k {
                        if key_str == key {
                            results.push(v);
                        }
                    }
                    // Queue child for traversal
                    work_queue.push(v);
                }
            }
            yaml_rust2::Yaml::Array(arr) => {
                // Traverse array elements
                work_queue.extend(arr.iter());
            }
            _ => {} // Scalars have no children
        }
    }
}

/// Navigate a JSON document using a path expression
///
/// Returns all matching values (may be multiple due to wildcards or recursive descent).
/// Uses iterative work queue algorithm - no recursion, no clones.

pub fn navigate_json<'a>(
    root: &'a serde_json::Value,
    path: &[PathComponent],
) -> Vec<&'a serde_json::Value> {
    let mut current_values = vec![root];

    for component in path {
        if current_values.is_empty() {
            return Vec::new(); // Early exit - no matches
        }

        let mut next_values = Vec::new();

        match component {
            PathComponent::Key(key) => {
                for value in current_values {
                    // serde_json provides get() method returning Option<&Value>
                    if let Some(field) = value.get(key.as_str()) {
                        next_values.push(field);
                    }
                }
            }

            PathComponent::Index(idx) => {
                for value in current_values {
                    if let serde_json::Value::Array(arr) = value {
                        if let Some(element) = arr.get(*idx) {
                            next_values.push(element);
                        }
                    }
                }
            }

            PathComponent::WildcardIndex => {
                for value in current_values {
                    if let serde_json::Value::Array(arr) = value {
                        next_values.extend(arr.iter());
                    }
                }
            }

            PathComponent::RecursiveKey(key) => {
                // Collect all occurrences of this key at any depth in the subtree
                for value in current_values {
                    collect_recursive_json_key(value, key, &mut next_values);
                }
            }
        }

        current_values = next_values;
    }

    current_values
}

/// Iteratively collect all values for a given key in the JSON subtree
fn collect_recursive_json_key<'a>(
    root: &'a serde_json::Value,
    key: &str,
    results: &mut Vec<&'a serde_json::Value>,
) {
    let mut work_queue = vec![root];

    while let Some(node) = work_queue.pop() {
        match node {
            serde_json::Value::Object(map) => {
                if let Some(v) = map.get(key) {
                    results.push(v);
                }
                // Queue all children for traversal
                work_queue.extend(map.values());
            }
            serde_json::Value::Array(arr) => {
                // Traverse array elements
                work_queue.extend(arr.iter());
            }
            _ => {} // Scalars have no children
        }
    }
}

/// Navigate a TOML document using a path expression
///
/// Returns all matching values (may be multiple due to wildcards or recursive descent).
/// Uses iterative work queue algorithm - no recursion, no clones.

pub fn navigate_toml<'a>(root: &'a toml::Value, path: &[PathComponent]) -> Vec<&'a toml::Value> {
    let mut current_values = vec![root];

    for component in path {
        if current_values.is_empty() {
            return Vec::new(); // Early exit - no matches
        }

        let mut next_values = Vec::new();

        match component {
            PathComponent::Key(key) => {
                for value in current_values {
                    // toml provides get() method returning Option<&Value>
                    if let Some(field) = value.get(key.as_str()) {
                        next_values.push(field);
                    }
                }
            }

            PathComponent::Index(idx) => {
                for value in current_values {
                    if let toml::Value::Array(arr) = value {
                        if let Some(element) = arr.get(*idx) {
                            next_values.push(element);
                        }
                    }
                }
            }

            PathComponent::WildcardIndex => {
                for value in current_values {
                    if let toml::Value::Array(arr) = value {
                        next_values.extend(arr.iter());
                    }
                }
            }

            PathComponent::RecursiveKey(key) => {
                // Collect all occurrences of this key at any depth in the subtree
                for value in current_values {
                    collect_recursive_toml_key(value, key, &mut next_values);
                }
            }
        }

        current_values = next_values;
    }

    current_values
}

/// Iteratively collect all values for a given key in the TOML subtree
fn collect_recursive_toml_key<'a>(
    root: &'a toml::Value,
    key: &str,
    results: &mut Vec<&'a toml::Value>,
) {
    let mut work_queue = vec![root];

    while let Some(node) = work_queue.pop() {
        match node {
            toml::Value::Table(map) => {
                if let Some(v) = map.get(key) {
                    results.push(v);
                }
                // Queue all children for traversal
                work_queue.extend(map.values());
            }
            toml::Value::Array(arr) => {
                // Traverse array elements
                work_queue.extend(arr.iter());
            }
            _ => {} // Scalars have no children
        }
    }
}

// ============================================================================
// Value Comparison
// ============================================================================

/// Convert YAML value to string for fallback comparison
/// Returns None for complex types (arrays, objects) that aren't comparable
fn yaml_to_string(value: &yaml_rust2::Yaml) -> Option<String> {
    match value {
        yaml_rust2::Yaml::Integer(i) => Some(i.to_string()),
        yaml_rust2::Yaml::String(s) => Some(s.clone()),
        yaml_rust2::Yaml::Boolean(b) => Some(b.to_string()),
        yaml_rust2::Yaml::Real(s) => Some(s.clone()),
        yaml_rust2::Yaml::Null => Some("null".to_string()),
        _ => None,
    }
}

/// Convert JSON value to string for fallback comparison
fn json_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        serde_json::Value::Null => Some("null".to_string()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => None,
    }
}

/// Convert TOML value to string for fallback comparison
fn toml_to_string(value: &toml::Value) -> Option<String> {
    match value {
        toml::Value::Integer(i) => Some(i.to_string()),
        toml::Value::Float(f) => Some(f.to_string()),
        toml::Value::String(s) => Some(s.clone()),
        toml::Value::Boolean(b) => Some(b.to_string()),
        toml::Value::Array(_) | toml::Value::Table(_) => None,
        toml::Value::Datetime(d) => Some(d.to_string()),
    }
}

/// Compare two i64 values with the given operator
fn compare_i64(a: i64, b: i64, operator: StructuredOperator) -> bool {
    match operator {
        StructuredOperator::Equals => a == b,
        StructuredOperator::NotEquals => a != b,
        StructuredOperator::Greater => a > b,
        StructuredOperator::GreaterOrEqual => a >= b,
        StructuredOperator::Less => a < b,
        StructuredOperator::LessOrEqual => a <= b,
    }
}

/// Compare two strings with the given operator (lexicographic ordering)
fn compare_strings(a: &str, b: &str, operator: StructuredOperator) -> bool {
    match operator {
        StructuredOperator::Equals => a == b,
        StructuredOperator::NotEquals => a != b,
        StructuredOperator::Greater => a > b,
        StructuredOperator::GreaterOrEqual => a >= b,
        StructuredOperator::Less => a < b,
        StructuredOperator::LessOrEqual => a <= b,
    }
}

/// Compare two f64 values with the given operator
fn compare_f64(a: f64, b: f64, operator: StructuredOperator) -> bool {
    match operator {
        StructuredOperator::Equals => (a - b).abs() < f64::EPSILON,
        StructuredOperator::NotEquals => (a - b).abs() >= f64::EPSILON,
        StructuredOperator::Greater => a > b,
        StructuredOperator::GreaterOrEqual => a >= b,
        StructuredOperator::Less => a < b,
        StructuredOperator::LessOrEqual => a <= b,
    }
}

/// Compare a single YAML value with expected value and operator
fn compare_yaml_value(
    actual: &yaml_rust2::Yaml,
    operator: StructuredOperator,
    expected: &yaml_rust2::Yaml,
    raw_string: &str,
) -> bool {
    match operator {
        StructuredOperator::Equals | StructuredOperator::NotEquals => {
            // Centralized equality logic with type coercion fallback
            let equals =
                actual == expected || yaml_to_string(actual).is_some_and(|s| s == raw_string);

            // NotEquals is simply the negation of Equals
            match operator {
                StructuredOperator::Equals => equals,
                StructuredOperator::NotEquals => !equals,
                _ => unreachable!(),
            }
        }

        // Ordering operators require same types or string fallback
        _ => match (actual, expected) {
            (yaml_rust2::Yaml::Integer(a), yaml_rust2::Yaml::Integer(e)) => {
                compare_i64(*a, *e, operator)
            }
            (yaml_rust2::Yaml::String(a), yaml_rust2::Yaml::String(e)) => {
                compare_strings(a, e, operator)
            }
            _ => {
                // Try string fallback for ordering
                yaml_to_string(actual).is_some_and(|s| compare_strings(&s, raw_string, operator))
            }
        },
    }
}

/// Compare a single JSON value with expected value and operator
fn compare_json_value(
    actual: &serde_json::Value,
    operator: StructuredOperator,
    expected: &serde_json::Value,
    raw_string: &str,
) -> bool {
    match operator {
        StructuredOperator::Equals | StructuredOperator::NotEquals => {
            // Centralized equality logic with type coercion fallback
            let equals =
                actual == expected || json_to_string(actual).is_some_and(|s| s == raw_string);

            // NotEquals is simply the negation of Equals
            match operator {
                StructuredOperator::Equals => equals,
                StructuredOperator::NotEquals => !equals,
                _ => unreachable!(),
            }
        }

        _ => match (actual, expected) {
            (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
                // Try integer comparison first (both integers)
                if let (Some(a_int), Some(b_int)) = (a.as_i64(), b.as_i64()) {
                    return compare_i64(a_int, b_int, operator);
                }
                // Fall back to float comparison (handles floats and mixed int/float)
                if let (Some(a_float), Some(b_float)) = (a.as_f64(), b.as_f64()) {
                    return compare_f64(a_float, b_float, operator);
                }
                false
            }
            (serde_json::Value::String(a), serde_json::Value::String(e)) => {
                compare_strings(a, e, operator)
            }
            _ => json_to_string(actual).is_some_and(|s| compare_strings(&s, raw_string, operator)),
        },
    }
}

/// Compare a single TOML value with expected value and operator
fn compare_toml_value(
    actual: &toml::Value,
    operator: StructuredOperator,
    expected: &toml::Value,
    raw_string: &str,
) -> bool {
    match operator {
        StructuredOperator::Equals | StructuredOperator::NotEquals => {
            // Centralized equality logic with type coercion fallback
            let equals =
                actual == expected || toml_to_string(actual).is_some_and(|s| s == raw_string);

            // NotEquals is simply the negation of Equals
            match operator {
                StructuredOperator::Equals => equals,
                StructuredOperator::NotEquals => !equals,
                _ => unreachable!(),
            }
        }

        _ => match (actual, expected) {
            (toml::Value::Integer(a), toml::Value::Integer(e)) => compare_i64(*a, *e, operator),
            (toml::Value::String(a), toml::Value::String(e)) => compare_strings(a, e, operator),
            _ => toml_to_string(actual).is_some_and(|s| compare_strings(&s, raw_string, operator)),
        },
    }
}

/// Compare YAML values (OR semantics: returns true if ANY value matches)

pub fn compare_yaml_values(
    actual_values: &[&yaml_rust2::Yaml],
    operator: StructuredOperator,
    expected: &yaml_rust2::Yaml,
    raw_string: &str,
) -> bool {
    actual_values
        .iter()
        .any(|actual| compare_yaml_value(actual, operator, expected, raw_string))
}

/// Compare JSON values (OR semantics: returns true if ANY value matches)

pub fn compare_json_values(
    actual_values: &[&serde_json::Value],
    operator: StructuredOperator,
    expected: &serde_json::Value,
    raw_string: &str,
) -> bool {
    actual_values
        .iter()
        .any(|actual| compare_json_value(actual, operator, expected, raw_string))
}

/// Compare TOML values (OR semantics: returns true if ANY value matches)

pub fn compare_toml_values(
    actual_values: &[&toml::Value],
    operator: StructuredOperator,
    expected: &toml::Value,
    raw_string: &str,
) -> bool {
    actual_values
        .iter()
        .any(|actual| compare_toml_value(actual, operator, expected, raw_string))
}

/// Match YAML values against string matcher (OR semantics)

pub fn match_yaml_strings(actual_values: &[&yaml_rust2::Yaml], matcher: &StringMatcher) -> bool {
    if actual_values.is_empty() {
        return false;
    }
    actual_values.iter().any(|actual| {
        if let Some(s) = yaml_to_string(actual) {
            matcher.is_match(&s)
        } else {
            false
        }
    })
}

/// Match JSON values against string matcher (OR semantics)

pub fn match_json_strings(actual_values: &[&serde_json::Value], matcher: &StringMatcher) -> bool {
    if actual_values.is_empty() {
        return false;
    }
    actual_values.iter().any(|actual| {
        if let Some(s) = json_to_string(actual) {
            matcher.is_match(&s)
        } else {
            false
        }
    })
}

/// Match TOML values against string matcher (OR semantics)

pub fn match_toml_strings(actual_values: &[&toml::Value], matcher: &StringMatcher) -> bool {
    if actual_values.is_empty() {
        return false;
    }
    actual_values.iter().any(|actual| {
        if let Some(s) = toml_to_string(actual) {
            matcher.is_match(&s)
        } else {
            false
        }
    })
}

/// Evaluate a structured data predicate against file contents
/// Uses the `ParsedDocuments` cache to avoid re-parsing the same format
pub fn eval_structured_predicate(
    predicate: &crate::predicate::StructuredDataPredicate,
    contents: &str,
    cache: &mut ParsedDocuments,
) -> Result<bool, String> {
    use crate::predicate::StructuredDataPredicate;

    match predicate {
        StructuredDataPredicate::YamlValue {
            path,
            operator,
            value,
            raw_string,
        } => {
            let docs = match cache.get_or_parse_yaml(contents) {
                Ok(docs) => docs,
                Err(e) => return Err(e.clone()),
            };
            // Check all documents with OR semantics (any match = true)
            for doc in docs {
                let values = navigate_yaml(doc, path);
                if compare_yaml_values(&values, *operator, value, raw_string) {
                    return Ok(true);
                }
            }
            Ok(false)
        }

        StructuredDataPredicate::YamlString { path, matcher } => {
            let docs = match cache.get_or_parse_yaml(contents) {
                Ok(docs) => docs,
                Err(e) => return Err(e.clone()),
            };
            for doc in docs {
                let values = navigate_yaml(doc, path);
                if match_yaml_strings(&values, matcher) {
                    return Ok(true);
                }
            }
            Ok(false)
        }

        StructuredDataPredicate::JsonValue {
            path,
            operator,
            value,
            raw_string,
        } => {
            let doc = match cache.get_or_parse_json(contents) {
                Ok(doc) => doc,
                Err(e) => return Err(e.clone()),
            };
            let values = navigate_json(doc, path);
            Ok(compare_json_values(&values, *operator, value, raw_string))
        }

        StructuredDataPredicate::JsonString { path, matcher } => {
            let doc = match cache.get_or_parse_json(contents) {
                Ok(doc) => doc,
                Err(e) => return Err(e.clone()),
            };
            let values = navigate_json(doc, path);
            Ok(match_json_strings(&values, matcher))
        }

        StructuredDataPredicate::TomlValue {
            path,
            operator,
            value,
            raw_string,
        } => {
            let doc = match cache.get_or_parse_toml(contents) {
                Ok(doc) => doc,
                Err(e) => return Err(e.clone()),
            };
            let values = navigate_toml(doc, path);
            Ok(compare_toml_values(&values, *operator, value, raw_string))
        }

        StructuredDataPredicate::TomlString { path, matcher } => {
            let doc = match cache.get_or_parse_toml(contents) {
                Ok(doc) => doc,
                Err(e) => return Err(e.clone()),
            };
            let values = navigate_toml(doc, path);
            Ok(match_toml_strings(&values, matcher))
        }
    }
}

/// Lazy parse cache for structured data formats
/// Ensures each format is only parsed once per file evaluation
pub struct ParsedDocuments {
    yaml: Option<Result<Vec<yaml_rust2::Yaml>, String>>,
    json: Option<Result<serde_json::Value, String>>,
    toml: Option<Result<toml::Value, String>>,
}

impl Default for ParsedDocuments {
    fn default() -> Self {
        Self::new()
    }
}

impl ParsedDocuments {
    pub fn new() -> Self {
        Self {
            yaml: None,
            json: None,
            toml: None,
        }
    }

    pub fn get_or_parse_yaml(&mut self, contents: &str) -> &Result<Vec<yaml_rust2::Yaml>, String> {
        if self.yaml.is_none() {
            self.yaml = Some(
                yaml_rust2::YamlLoader::load_from_str(contents)
                    .map_err(|e| format!("YAML parse error: {e}")),
            );
        }
        self.yaml.as_ref().unwrap()
    }

    pub fn get_or_parse_json(&mut self, contents: &str) -> &Result<serde_json::Value, String> {
        if self.json.is_none() {
            self.json =
                Some(serde_json::from_str(contents).map_err(|e| format!("JSON parse error: {e}")));
        }
        self.json.as_ref().unwrap()
    }

    pub fn get_or_parse_toml(&mut self, contents: &str) -> &Result<toml::Value, String> {
        if self.toml.is_none() {
            self.toml =
                Some(toml::from_str(contents).map_err(|e| format!("TOML parse error: {e}")));
        }
        self.toml.as_ref().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yaml_rust2::YamlLoader;

    #[test]
    fn test_navigate_yaml_simple_key() {
        let yaml = YamlLoader::load_from_str("port: 8080").unwrap();
        let path = vec![PathComponent::Key("port".to_string())];
        let results = navigate_yaml(&yaml[0], &path);

        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], yaml_rust2::Yaml::Integer(8080)));
    }

    #[test]
    fn test_navigate_yaml_nested_keys() {
        let yaml = YamlLoader::load_from_str("spec:\n  replicas: 3").unwrap();
        let path = vec![
            PathComponent::Key("spec".to_string()),
            PathComponent::Key("replicas".to_string()),
        ];
        let results = navigate_yaml(&yaml[0], &path);

        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], yaml_rust2::Yaml::Integer(3)));
    }

    #[test]
    fn test_navigate_yaml_array_index() {
        let yaml = YamlLoader::load_from_str("items:\n  - foo\n  - bar\n  - baz").unwrap();
        let path = vec![
            PathComponent::Key("items".to_string()),
            PathComponent::Index(1),
        ];
        let results = navigate_yaml(&yaml[0], &path);

        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], yaml_rust2::Yaml::String(s) if s == "bar"));
    }

    #[test]
    fn test_navigate_yaml_wildcard() {
        let yaml = YamlLoader::load_from_str("items:\n  - a\n  - b\n  - c").unwrap();
        let path = vec![
            PathComponent::Key("items".to_string()),
            PathComponent::WildcardIndex,
        ];
        let results = navigate_yaml(&yaml[0], &path);

        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_navigate_yaml_recursive_key() {
        let yaml = YamlLoader::load_from_str(
            "database:\n  host: localhost\nnested:\n  database:\n    host: remote",
        )
        .unwrap();
        let path = vec![
            PathComponent::RecursiveKey("database".to_string()),
            PathComponent::Key("host".to_string()),
        ];
        let results = navigate_yaml(&yaml[0], &path);

        // Should find both "localhost" and "remote"
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_navigate_json_simple() {
        let json: serde_json::Value = serde_json::from_str(r#"{"port": 8080}"#).unwrap();
        let path = vec![PathComponent::Key("port".to_string())];
        let results = navigate_json(&json, &path);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_i64(), Some(8080));
    }

    #[test]
    fn test_navigate_toml_simple() {
        let toml: toml::Value = toml::from_str("port = 8080").unwrap();
        let path = vec![PathComponent::Key("port".to_string())];
        let results = navigate_toml(&toml, &path);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_integer(), Some(8080));
    }

    #[test]
    fn test_navigate_yaml_no_match() {
        let yaml = YamlLoader::load_from_str("port: 8080").unwrap();
        let path = vec![PathComponent::Key("missing".to_string())];
        let results = navigate_yaml(&yaml[0], &path);

        assert_eq!(results.len(), 0);
    }

    // ========================================================================
    // Comparison Tests
    // ========================================================================

    #[test]
    fn test_yaml_integer_equals() {
        let yaml = YamlLoader::load_from_str("port: 8080").unwrap();
        let expected = yaml_rust2::Yaml::Integer(8080);
        let path = vec![PathComponent::Key("port".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(compare_yaml_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "8080"
        ));
    }

    #[test]
    fn test_yaml_integer_greater() {
        let yaml = YamlLoader::load_from_str("count: 100").unwrap();
        let expected = yaml_rust2::Yaml::Integer(50);
        let path = vec![PathComponent::Key("count".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(compare_yaml_values(
            &values,
            StructuredOperator::Greater,
            &expected,
            "50"
        ));
    }

    #[test]
    fn test_yaml_integer_less() {
        let yaml = YamlLoader::load_from_str("count: 10").unwrap();
        let expected = yaml_rust2::Yaml::Integer(50);
        let path = vec![PathComponent::Key("count".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(compare_yaml_values(
            &values,
            StructuredOperator::Less,
            &expected,
            "50"
        ));
    }

    #[test]
    fn test_yaml_string_equals() {
        let yaml = YamlLoader::load_from_str("name: test").unwrap();
        let expected = yaml_rust2::Yaml::String("test".to_string());
        let path = vec![PathComponent::Key("name".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(compare_yaml_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "test"
        ));
    }

    #[test]
    fn test_yaml_string_greater_lexicographic() {
        let yaml = YamlLoader::load_from_str("name: zebra").unwrap();
        let expected = yaml_rust2::Yaml::String("apple".to_string());
        let path = vec![PathComponent::Key("name".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(compare_yaml_values(
            &values,
            StructuredOperator::Greater,
            &expected,
            "apple"
        ));
    }

    #[test]
    fn test_yaml_boolean_equals() {
        let yaml = YamlLoader::load_from_str("enabled: true").unwrap();
        let expected = yaml_rust2::Yaml::Boolean(true);
        let path = vec![PathComponent::Key("enabled".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(compare_yaml_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "true"
        ));
    }

    #[test]
    fn test_yaml_type_mismatch_fallback() {
        // File has string "8080", query expects integer 8080
        let yaml = YamlLoader::load_from_str("port: \"8080\"").unwrap();
        let expected = yaml_rust2::Yaml::Integer(8080);
        let path = vec![PathComponent::Key("port".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        // Should match via string fallback: "8080" == "8080"
        assert!(compare_yaml_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "8080"
        ));
    }

    #[test]
    fn test_yaml_type_mismatch_no_match() {
        // File has string "wrong", query expects integer 8080
        let yaml = YamlLoader::load_from_str("port: wrong").unwrap();
        let expected = yaml_rust2::Yaml::Integer(8080);
        let path = vec![PathComponent::Key("port".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        // Should not match: "wrong" != "8080"
        assert!(!compare_yaml_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "8080"
        ));
    }

    #[test]
    fn test_yaml_null_equals() {
        let yaml = YamlLoader::load_from_str("value: null").unwrap();
        let expected = yaml_rust2::Yaml::Null;
        let path = vec![PathComponent::Key("value".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(compare_yaml_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "null"
        ));
    }

    #[test]
    fn test_yaml_array_no_match() {
        // Arrays can't be compared with scalars
        let yaml = YamlLoader::load_from_str("items: [1, 2, 3]").unwrap();
        let expected = yaml_rust2::Yaml::Integer(5);
        let path = vec![PathComponent::Key("items".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(!compare_yaml_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "5"
        ));
    }

    #[test]
    fn test_yaml_empty_results_no_match() {
        let yaml = YamlLoader::load_from_str("port: 8080").unwrap();
        let expected = yaml_rust2::Yaml::Integer(8080);
        let path = vec![PathComponent::Key("missing".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        // Empty results should return false
        assert_eq!(values.len(), 0);
        assert!(!compare_yaml_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "8080"
        ));
    }

    #[test]
    fn test_yaml_wildcard_or_semantics() {
        // Array with multiple values - ANY match returns true
        let yaml = YamlLoader::load_from_str("ports: [8080, 9090, 3000]").unwrap();
        let expected = yaml_rust2::Yaml::Integer(9090);
        let path = vec![
            PathComponent::Key("ports".to_string()),
            PathComponent::WildcardIndex,
        ];
        let values = navigate_yaml(&yaml[0], &path);

        // Should find 9090 in the array
        assert_eq!(values.len(), 3);
        assert!(compare_yaml_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "9090"
        ));
    }

    #[test]
    fn test_yaml_wildcard_no_match() {
        // Array with multiple values - none match
        let yaml = YamlLoader::load_from_str("ports: [8080, 9090, 3000]").unwrap();
        let expected = yaml_rust2::Yaml::Integer(5000);
        let path = vec![
            PathComponent::Key("ports".to_string()),
            PathComponent::WildcardIndex,
        ];
        let values = navigate_yaml(&yaml[0], &path);

        assert_eq!(values.len(), 3);
        assert!(!compare_yaml_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "5000"
        ));
    }

    #[test]
    fn test_yaml_string_matcher_regex() {
        let yaml = YamlLoader::load_from_str("name: test-app").unwrap();
        let matcher = StringMatcher::regex("test-.*").unwrap();
        let path = vec![PathComponent::Key("name".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(match_yaml_strings(&values, &matcher));
    }

    #[test]
    fn test_yaml_string_matcher_contains() {
        let yaml = YamlLoader::load_from_str("description: This is a test").unwrap();
        let matcher = StringMatcher::contains("test");
        let path = vec![PathComponent::Key("description".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(match_yaml_strings(&values, &matcher));
    }

    #[test]
    fn test_yaml_string_matcher_integer_coercion() {
        // Integer value should be converted to string for matching
        let yaml = YamlLoader::load_from_str("port: 8080").unwrap();
        let matcher = StringMatcher::regex("80.*").unwrap();
        let path = vec![PathComponent::Key("port".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(match_yaml_strings(&values, &matcher));
    }

    #[test]
    fn test_json_integer_equals() {
        let json: serde_json::Value = serde_json::from_str(r#"{"port": 8080}"#).unwrap();
        let expected = serde_json::Value::Number(serde_json::Number::from(8080));
        let path = vec![PathComponent::Key("port".to_string())];
        let values = navigate_json(&json, &path);

        assert!(compare_json_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "8080"
        ));
    }

    #[test]
    fn test_json_string_type_mismatch_fallback() {
        // File has string "8080", query expects number 8080
        let json: serde_json::Value = serde_json::from_str(r#"{"port": "8080"}"#).unwrap();
        let expected = serde_json::Value::Number(serde_json::Number::from(8080));
        let path = vec![PathComponent::Key("port".to_string())];
        let values = navigate_json(&json, &path);

        // Should match via string fallback
        assert!(compare_json_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "8080"
        ));
    }

    #[test]
    fn test_json_string_matcher() {
        let json: serde_json::Value = serde_json::from_str(r#"{"name": "test-app"}"#).unwrap();
        let matcher = StringMatcher::regex("test-.*").unwrap();
        let path = vec![PathComponent::Key("name".to_string())];
        let values = navigate_json(&json, &path);

        assert!(match_json_strings(&values, &matcher));
    }

    #[test]
    fn test_toml_integer_equals() {
        let toml: toml::Value = toml::from_str("port = 8080").unwrap();
        let expected = toml::Value::Integer(8080);
        let path = vec![PathComponent::Key("port".to_string())];
        let values = navigate_toml(&toml, &path);

        assert!(compare_toml_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "8080"
        ));
    }

    #[test]
    fn test_toml_string_type_mismatch_fallback() {
        // File has string "8080", query expects integer 8080
        let toml: toml::Value = toml::from_str(r#"port = "8080""#).unwrap();
        let expected = toml::Value::Integer(8080);
        let path = vec![PathComponent::Key("port".to_string())];
        let values = navigate_toml(&toml, &path);

        // Should match via string fallback
        assert!(compare_toml_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "8080"
        ));
    }

    #[test]
    fn test_toml_string_matcher() {
        let toml: toml::Value = toml::from_str(r#"name = "test-app""#).unwrap();
        let matcher = StringMatcher::regex("test-.*").unwrap();
        let path = vec![PathComponent::Key("name".to_string())];
        let values = navigate_toml(&toml, &path);

        assert!(match_toml_strings(&values, &matcher));
    }

    // ========================================================================
    // NotEquals Operator Tests
    // ========================================================================

    #[test]
    fn test_yaml_integer_not_equals() {
        let yaml = YamlLoader::load_from_str("port: 8080").unwrap();
        let expected = yaml_rust2::Yaml::Integer(9090);
        let path = vec![PathComponent::Key("port".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(compare_yaml_values(
            &values,
            StructuredOperator::NotEquals,
            &expected,
            "9090"
        ));
    }

    #[test]
    fn test_yaml_string_not_equals() {
        let yaml = YamlLoader::load_from_str("name: foo").unwrap();
        let expected = yaml_rust2::Yaml::String("bar".to_string());
        let path = vec![PathComponent::Key("name".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(compare_yaml_values(
            &values,
            StructuredOperator::NotEquals,
            &expected,
            "bar"
        ));
    }

    #[test]
    fn test_yaml_not_equals_type_coercion() {
        let yaml = YamlLoader::load_from_str("port: 8080").unwrap();
        let expected = yaml_rust2::Yaml::String("8080".to_string());
        let path = vec![PathComponent::Key("port".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(!compare_yaml_values(
            &values,
            StructuredOperator::NotEquals,
            &expected,
            "8080"
        ));
    }

    #[test]
    fn test_json_not_equals_type_coercion() {
        let json: serde_json::Value = serde_json::from_str(r#"{"port": 8080}"#).unwrap();
        let expected = serde_json::Value::String("8080".to_string());
        let path = vec![PathComponent::Key("port".to_string())];
        let values = navigate_json(&json, &path);

        assert!(!compare_json_values(
            &values,
            StructuredOperator::NotEquals,
            &expected,
            "8080"
        ));
    }

    #[test]
    fn test_toml_not_equals_type_coercion() {
        let toml: toml::Value = toml::from_str("port = 8080").unwrap();
        let expected = toml::Value::String("8080".to_string());
        let path = vec![PathComponent::Key("port".to_string())];
        let values = navigate_toml(&toml, &path);

        assert!(!compare_toml_values(
            &values,
            StructuredOperator::NotEquals,
            &expected,
            "8080"
        ));
    }

    #[test]
    fn test_yaml_not_equals_null() {
        let yaml = YamlLoader::load_from_str("value: null").unwrap();
        let expected = yaml_rust2::Yaml::String("something".to_string());
        let path = vec![PathComponent::Key("value".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(compare_yaml_values(
            &values,
            StructuredOperator::NotEquals,
            &expected,
            "something"
        ));
    }

    // ========================================================================
    // Float Comparison Tests
    // ========================================================================

    #[test]
    fn test_json_float_equals() {
        let json: serde_json::Value = serde_json::from_str(r#"{"value": 1.5}"#).unwrap();
        let expected = serde_json::Value::Number(serde_json::Number::from_f64(1.5).unwrap());
        let path = vec![PathComponent::Key("value".to_string())];
        let values = navigate_json(&json, &path);

        assert!(compare_json_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "1.5"
        ));
    }

    #[test]
    fn test_json_float_greater_than() {
        let json: serde_json::Value = serde_json::from_str(r#"{"value": 1.5}"#).unwrap();
        let expected = serde_json::Value::Number(serde_json::Number::from_f64(1.0).unwrap());
        let path = vec![PathComponent::Key("value".to_string())];
        let values = navigate_json(&json, &path);

        assert!(compare_json_values(
            &values,
            StructuredOperator::Greater,
            &expected,
            "1.0"
        ));
    }

    #[test]
    fn test_json_float_less_than() {
        let json: serde_json::Value = serde_json::from_str(r#"{"value": 1.5}"#).unwrap();
        let expected = serde_json::Value::Number(serde_json::Number::from_f64(2.0).unwrap());
        let path = vec![PathComponent::Key("value".to_string())];
        let values = navigate_json(&json, &path);

        assert!(compare_json_values(
            &values,
            StructuredOperator::Less,
            &expected,
            "2.0"
        ));
    }

    #[test]
    fn test_json_float_string_fallback() {
        let json: serde_json::Value = serde_json::from_str(r#"{"value": 1.5}"#).unwrap();
        let expected = serde_json::Value::String("1.5".to_string());
        let path = vec![PathComponent::Key("value".to_string())];
        let values = navigate_json(&json, &path);

        assert!(compare_json_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "1.5"
        ));
    }

    #[test]
    fn test_json_negative_float_comparison() {
        let json: serde_json::Value = serde_json::from_str(r#"{"value": -2.7}"#).unwrap();
        let expected = serde_json::Value::Number(serde_json::Number::from_f64(-2.0).unwrap());
        let path = vec![PathComponent::Key("value".to_string())];
        let values = navigate_json(&json, &path);

        assert!(compare_json_values(
            &values,
            StructuredOperator::Less,
            &expected,
            "-2.0"
        ));
    }

    // ========================================================================
    // Edge Case Tests
    // ========================================================================

    #[test]
    fn test_json_zero_equals() {
        let json: serde_json::Value = serde_json::from_str(r#"{"value": 0}"#).unwrap();
        let expected = serde_json::Value::Number(serde_json::Number::from(0));
        let path = vec![PathComponent::Key("value".to_string())];
        let values = navigate_json(&json, &path);

        assert!(compare_json_values(
            &values,
            StructuredOperator::Equals,
            &expected,
            "0"
        ));
    }

    #[test]
    fn test_json_zero_greater_than_negative() {
        let json: serde_json::Value = serde_json::from_str(r#"{"value": 0}"#).unwrap();
        let expected = serde_json::Value::Number(serde_json::Number::from(-1));
        let path = vec![PathComponent::Key("value".to_string())];
        let values = navigate_json(&json, &path);

        assert!(compare_json_values(
            &values,
            StructuredOperator::Greater,
            &expected,
            "-1"
        ));
    }

    #[test]
    fn test_json_large_integer() {
        let large = i64::MAX - 1;
        let json: serde_json::Value =
            serde_json::from_str(&format!(r#"{{"value": {}}}"#, large)).unwrap();
        let expected = serde_json::Value::Number(serde_json::Number::from(large - 1));
        let path = vec![PathComponent::Key("value".to_string())];
        let values = navigate_json(&json, &path);

        assert!(compare_json_values(
            &values,
            StructuredOperator::Greater,
            &expected,
            &(large - 1).to_string()
        ));
    }

    #[test]
    fn test_yaml_boolean_not_equals() {
        let yaml = YamlLoader::load_from_str("enabled: true").unwrap();
        let expected = yaml_rust2::Yaml::Boolean(false);
        let path = vec![PathComponent::Key("enabled".to_string())];
        let values = navigate_yaml(&yaml[0], &path);

        assert!(compare_yaml_values(
            &values,
            StructuredOperator::NotEquals,
            &expected,
            "false"
        ));
    }

    #[test]
    fn test_navigate_yaml_out_of_bounds_index() {
        let yaml = YamlLoader::load_from_str("items:\n  - a\n  - b\n  - c").unwrap();
        let path = vec![
            PathComponent::Key("items".to_string()),
            PathComponent::Index(999),
        ];
        let values = navigate_yaml(&yaml[0], &path);
        assert_eq!(values.len(), 0);
    }

    #[test]
    fn test_navigate_yaml_out_of_bounds_on_empty_array() {
        let yaml = YamlLoader::load_from_str("items: []").unwrap();
        let path = vec![
            PathComponent::Key("items".to_string()),
            PathComponent::Index(0),
        ];
        let values = navigate_yaml(&yaml[0], &path);
        assert_eq!(values.len(), 0);
    }

    #[test]
    fn test_navigate_json_out_of_bounds() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"items": ["a", "b", "c"]}"#).unwrap();
        let path = vec![
            PathComponent::Key("items".to_string()),
            PathComponent::Index(999),
        ];
        let values = navigate_json(&json, &path);
        assert_eq!(values.len(), 0);
    }

    #[test]
    fn test_navigate_toml_out_of_bounds() {
        let toml: toml::Value = toml::from_str(r#"items = ["a", "b", "c"]"#).unwrap();
        let path = vec![
            PathComponent::Key("items".to_string()),
            PathComponent::Index(999),
        ];
        let values = navigate_toml(&toml, &path);
        assert_eq!(values.len(), 0);
    }

    #[test]
    fn test_toml_datetime_equality() {
        let toml: toml::Value = toml::from_str(r#"timestamp = 2024-01-15T10:30:00Z"#).unwrap();
        let expected: toml::Value = toml::from_str(r#"timestamp = 2024-01-15T10:30:00Z"#).unwrap();
        let path = vec![PathComponent::Key("timestamp".to_string())];
        let values = navigate_toml(&toml, &path);

        assert!(compare_toml_values(
            &values,
            StructuredOperator::Equals,
            &expected["timestamp"],
            "2024-01-15T10:30:00Z"
        ));
    }

    #[test]
    fn test_toml_datetime_string_coercion() {
        let toml: toml::Value = toml::from_str(r#"timestamp = 2024-01-15T10:30:00Z"#).unwrap();
        let expected_string = toml::Value::String("2024-01-15T10:30:00Z".to_string());
        let path = vec![PathComponent::Key("timestamp".to_string())];
        let values = navigate_toml(&toml, &path);

        assert!(compare_toml_values(
            &values,
            StructuredOperator::Equals,
            &expected_string,
            "2024-01-15T10:30:00Z"
        ));
    }

    #[test]
    fn test_toml_datetime_comparison_lexicographic() {
        let toml: toml::Value = toml::from_str(r#"timestamp = 2024-01-15T10:30:00Z"#).unwrap();
        let expected = toml::Value::String("2024-01-01T00:00:00Z".to_string());
        let path = vec![PathComponent::Key("timestamp".to_string())];
        let values = navigate_toml(&toml, &path);

        assert!(compare_toml_values(
            &values,
            StructuredOperator::Greater,
            &expected,
            "2024-01-01T00:00:00Z"
        ));
    }
}
