//! Navigation and evaluation logic for structured data selectors (YAML/JSON/TOML)
//!
//! Provides zero-allocation, iterative traversal of parsed documents using path expressions.

use crate::parser::structured_path::PathComponent;

/// Navigate a YAML document using a path expression
///
/// Returns all matching values (may be multiple due to wildcards or recursive descent).
/// Uses iterative work queue algorithm - no recursion, no clones.
///
/// # Examples
/// ```ignore
/// let yaml = YamlLoader::load_from_str("port: 8080").unwrap();
/// let path = vec![PathComponent::Key("port".to_string())];
/// let results = navigate_yaml(&yaml[0], &path);
/// assert_eq!(results.len(), 1);
/// ```
pub fn navigate_yaml<'a>(
    root: &'a yaml_rust::Yaml,
    path: &[PathComponent],
) -> Vec<&'a yaml_rust::Yaml> {
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
                    if let yaml_rust::Yaml::Array(arr) = value {
                        if let Some(element) = arr.get(*idx) {
                            next_values.push(element);
                        }
                    }
                }
            }

            PathComponent::WildcardIndex => {
                for value in current_values {
                    if let yaml_rust::Yaml::Array(arr) = value {
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
    root: &'a yaml_rust::Yaml,
    key: &str,
    results: &mut Vec<&'a yaml_rust::Yaml>,
) {
    let mut work_queue = vec![root];

    while let Some(node) = work_queue.pop() {
        match node {
            yaml_rust::Yaml::Hash(map) => {
                // Check if this hash contains the target key
                for (k, v) in map.iter() {
                    if let yaml_rust::Yaml::String(key_str) = k {
                        if key_str == key {
                            results.push(v);
                        }
                    }
                    // Queue child for traversal
                    work_queue.push(v);
                }
            }
            yaml_rust::Yaml::Array(arr) => {
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
                // Check if this object contains the target key
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
pub fn navigate_toml<'a>(
    root: &'a toml::Value,
    path: &[PathComponent],
) -> Vec<&'a toml::Value> {
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
                // Check if this table contains the target key
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

#[cfg(test)]
mod tests {
    use super::*;
    use yaml_rust::YamlLoader;

    #[test]
    fn test_navigate_yaml_simple_key() {
        let yaml = YamlLoader::load_from_str("port: 8080").unwrap();
        let path = vec![PathComponent::Key("port".to_string())];
        let results = navigate_yaml(&yaml[0], &path);

        assert_eq!(results.len(), 1);
        assert!(matches!(results[0], yaml_rust::Yaml::Integer(8080)));
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
        assert!(matches!(results[0], yaml_rust::Yaml::Integer(3)));
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
        assert!(matches!(results[0], yaml_rust::Yaml::String(s) if s == "bar"));
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
            "database:\n  host: localhost\nnested:\n  database:\n    host: remote"
        ).unwrap();
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
}
