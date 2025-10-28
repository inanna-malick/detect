/// Type-safe typechecker using the new typed selector/operator system
use crate::{
    expr::Expr,
    parser::{
        error::{DetectError, SpanExt},
        typed::{
            self, EnumOperator, EnumSelector, NumericOperator, NumericSelector, PathComponent,
            StringOperator, StringSelector, TemporalOperator, TemporalSelector, TypedSelector,
        },
        RawExpr, RawPredicate, RawValue,
    },
    predicate::{
        parse_time_value, Bound, DetectFileType, EnumMatcher, EnumPredicate, MetadataPredicate,
        NamePredicate, NumberMatcher, Predicate, StreamingCompiledContentPredicate, StringMatcher,
        TimeMatcher,
    },
};

// Re-export DetectError as TypecheckError for compatibility
pub use crate::parser::error::DetectError as TypecheckError;

/// Parse size values like "1mb", "100kb", etc. into bytes
fn parse_size_value(s: &str, value_span: pest::Span, source: &str) -> Result<u64, TypecheckError> {
    crate::util::parse_size(s).map_err(|err_msg| TypecheckError::InvalidValue {
        expected: "size with unit (e.g., 1mb, 100kb)".to_string(),
        found: err_msg,
        span: value_span.to_source_span(),
        src: source.to_string(),
    })
}

/// Check if an operator is an ordering comparison (>, >=, <, <=)
fn is_ordering_operator(op: typed::StructuredOperator) -> bool {
    matches!(
        op,
        typed::StructuredOperator::Greater
            | typed::StructuredOperator::GreaterOrEqual
            | typed::StructuredOperator::Less
            | typed::StructuredOperator::LessOrEqual
    )
}

/// Main typechecker that transforms raw AST to typed expressions
pub struct Typechecker;

impl Typechecker {
    /// Transform a raw expression into a typed expression
    ///
    /// # Errors
    /// Returns `DetectError` for syntax errors, unknown selectors, incompatible operators, or invalid values.
    pub fn typecheck(
        raw_expr: RawExpr<'_>,
        source: &str,
        config: &crate::RuntimeConfig,
    ) -> Result<Expr<Predicate>, DetectError> {
        Self::typecheck_inner(raw_expr, source, config)
    }

    fn typecheck_inner(
        raw_expr: RawExpr<'_>,
        source: &str,
        config: &crate::RuntimeConfig,
    ) -> Result<Expr<Predicate>, DetectError> {
        match raw_expr {
            RawExpr::Predicate(pred) => Self::typecheck_predicate(pred, source, config),
            RawExpr::And(lhs, rhs) => {
                let typed_lhs = Self::typecheck_inner(*lhs, source, config)?;
                let typed_rhs = Self::typecheck_inner(*rhs, source, config)?;
                Ok(Expr::and(typed_lhs, typed_rhs))
            }
            RawExpr::Or(lhs, rhs) => {
                let typed_lhs = Self::typecheck_inner(*lhs, source, config)?;
                let typed_rhs = Self::typecheck_inner(*rhs, source, config)?;
                Ok(Expr::or(typed_lhs, typed_rhs))
            }
            RawExpr::Not(expr) => {
                let typed_expr = Self::typecheck_inner(*expr, source, config)?;
                Ok(Expr::negate(typed_expr))
            }
            RawExpr::SingleWord(span) => {
                let word = span.as_str();

                // Try to resolve as alias
                match crate::parser::resolve_alias(word) {
                    Ok(predicate) => Ok(Expr::Predicate(predicate)),
                    Err(_) => {
                        // Generate suggestions
                        let suggestions = crate::parser::suggest_aliases(word);
                        let suggestions_msg = if !suggestions.is_empty() {
                            Some(format!("Did you mean: {}?", suggestions.join(", ")))
                        } else {
                            Some(format!(
                                "Valid aliases: {}",
                                crate::predicate::DetectFileType::all_valid_strings().join(", ")
                            ))
                        };

                        Err(DetectError::UnknownAlias {
                            word: word.to_string(),
                            span: span.to_source_span(),
                            src: source.to_string(),
                            suggestions: suggestions_msg,
                        })
                    }
                }
            }
        }
    }

    /// Build synthetic precondition for structured data predicates
    /// Wraps actual predicate in: (ext in [exts]) AND (size < max) AND actual_predicate
    fn build_synthetic_precondition(
        format: typed::DataFormat,
        config: &crate::RuntimeConfig,
        actual_predicate: Predicate,
    ) -> Expr<Predicate> {
        use std::collections::HashSet;
        use typed::DataFormat;

        // Map format to file extensions
        let extensions: Vec<&str> = match format {
            DataFormat::Yaml => vec!["yaml", "yml"],
            DataFormat::Json => vec!["json"],
            DataFormat::Toml => vec!["toml"],
        };

        // Build: ext in [extensions]
        let ext_set: HashSet<String> = extensions.iter().map(|s| s.to_string()).collect();
        let ext_predicate = Predicate::name(NamePredicate::Extension(StringMatcher::In(ext_set)));

        // Build: size < max_structured_size
        let size_predicate = Predicate::meta(MetadataPredicate::Filesize(NumberMatcher::In(
            Bound::Right(..config.max_structured_size),
        )));

        // Construct: ext_check AND size_check AND actual_predicate
        Expr::and(
            Expr::and(
                Expr::Predicate(ext_predicate),
                Expr::Predicate(size_predicate),
            ),
            Expr::Predicate(actual_predicate),
        )
    }

    /// Transform a raw predicate into a typed predicate using the new type system
    fn typecheck_predicate(
        pred: RawPredicate<'_>,
        source: &str,
        config: &crate::RuntimeConfig,
    ) -> Result<Expr<Predicate>, DetectError> {
        // Parse selector and operator together with type safety, passing spans
        let typed_selector = typed::parse_selector_operator(
            pred.selector,
            pred.selector_span,
            pred.operator,
            pred.operator_span,
            source,
        )?;

        match typed_selector {
            TypedSelector::String(selector, operator) => {
                let predicate = Self::build_string_predicate(
                    selector,
                    operator,
                    &pred.value,
                    pred.value_span,
                    source,
                )?;
                Ok(Expr::Predicate(predicate))
            }
            TypedSelector::Numeric(selector, operator) => {
                let predicate = Self::build_numeric_predicate(
                    selector,
                    operator,
                    &pred.value,
                    pred.value_span,
                    source,
                )?;
                Ok(Expr::Predicate(predicate))
            }
            TypedSelector::Temporal(selector, operator) => {
                let predicate = Self::build_temporal_predicate(
                    selector,
                    operator,
                    &pred.value,
                    pred.value_span,
                    source,
                )?;
                Ok(Expr::Predicate(predicate))
            }
            TypedSelector::Enum(selector, operator) => {
                let predicate = Self::build_enum_predicate(
                    selector,
                    operator,
                    &pred.value,
                    pred.value_span,
                    source,
                )?;
                Ok(Expr::Predicate(predicate))
            }
            TypedSelector::StructuredData(format, path, operator) => {
                let predicate = Self::build_structured_predicate(
                    format,
                    path,
                    operator,
                    &pred.value,
                    pred.value_span,
                    source,
                )?;
                Ok(Self::build_synthetic_precondition(
                    format, config, predicate,
                ))
            }
            TypedSelector::StructuredDataString(format, path, string_operator) => {
                let predicate = Self::build_structured_string_predicate(
                    format,
                    path,
                    string_operator,
                    &pred.value,
                    pred.value_span,
                    source,
                )?;
                Ok(Self::build_synthetic_precondition(
                    format, config, predicate,
                ))
            }
        }
    }

    /// Build a string-type predicate
    fn build_string_predicate(
        selector: StringSelector,
        operator: StringOperator,
        value: &RawValue,
        value_span: pest::Span,
        source: &str,
    ) -> Result<Predicate, DetectError> {
        let string_matcher = Self::parse_string_value(value, operator, value_span, source)?;

        match selector {
            StringSelector::Path(component) => {
                let name_pred = match component {
                    PathComponent::Full => NamePredicate::FullPath(string_matcher),
                    PathComponent::Name => NamePredicate::FileName(string_matcher),
                    PathComponent::Stem => NamePredicate::BaseName(string_matcher),
                    PathComponent::Extension => NamePredicate::Extension(string_matcher),
                    PathComponent::Parent => NamePredicate::DirPath(string_matcher),
                };
                Ok(Predicate::name(name_pred))
            }
            StringSelector::Contents => {
                let pattern = Self::build_content_pattern(value, operator, value_span, source)?;
                let content_pred =
                    StreamingCompiledContentPredicate::new(pattern).map_err(|e| {
                        DetectError::InvalidValue {
                            expected: "valid regex pattern".to_string(),
                            found: format!("{:?}", e),
                            span: value_span.to_source_span(),
                            src: source.to_string(),
                        }
                    })?;
                Ok(Predicate::contents(content_pred))
            }
        }
    }

    /// Build a numeric-type predicate
    fn build_numeric_predicate(
        selector: NumericSelector,
        operator: NumericOperator,
        value: &RawValue,
        value_span: pest::Span,
        source: &str,
    ) -> Result<Predicate, DetectError> {
        let number_value = Self::parse_numeric_value(value, &selector, value_span, source)?;
        let number_matcher = Self::build_number_matcher(operator, number_value);

        let meta_pred = match selector {
            NumericSelector::Size => MetadataPredicate::Filesize(number_matcher),
            NumericSelector::Depth => MetadataPredicate::Depth(number_matcher),
        };
        Ok(Predicate::meta(meta_pred))
    }

    /// Build a temporal-type predicate
    fn build_temporal_predicate(
        selector: TemporalSelector,
        operator: TemporalOperator,
        value: &RawValue,
        value_span: pest::Span,
        source: &str,
    ) -> Result<Predicate, DetectError> {
        let time_value = Self::parse_temporal_value(value, value_span, source)?;
        let time_matcher = Self::build_time_matcher(operator, time_value);

        let meta_pred = match selector {
            TemporalSelector::Modified => MetadataPredicate::Modified(time_matcher),
            TemporalSelector::Created => MetadataPredicate::Created(time_matcher),
            TemporalSelector::Accessed => MetadataPredicate::Accessed(time_matcher),
        };
        Ok(Predicate::meta(meta_pred))
    }

    /// Build an enum-type predicate with parse-time validation
    fn build_enum_predicate(
        selector: EnumSelector,
        operator: EnumOperator,
        value: &RawValue,
        value_span: pest::Span,
        source: &str,
    ) -> Result<Predicate, DetectError> {
        match selector {
            EnumSelector::Type => {
                let enum_matcher =
                    Self::parse_enum_value::<DetectFileType>(value, operator, value_span, source)?;
                Ok(Predicate::meta(MetadataPredicate::Type(enum_matcher)))
            }
        }
    }

    /// Build a structured data value predicate (yaml/json/toml with value operations)
    fn build_structured_predicate(
        format: typed::DataFormat,
        path: Vec<super::structured_path::PathComponent>,
        operator: typed::StructuredOperator,
        value: &RawValue,
        value_span: pest::Span,
        source: &str,
    ) -> Result<Predicate, DetectError> {
        use crate::predicate::StructuredDataPredicate;
        use typed::DataFormat;

        // Capture raw string for string coercion fallback (preserves original formatting)
        let raw_string = value.as_string().to_string();

        // Build native value based on format
        let predicate = match format {
            DataFormat::Yaml => {
                let yaml_value = Self::build_yaml_rhs(value);

                // Validate comparison operators require numeric values
                if is_ordering_operator(operator) && !Self::is_comparable_yaml(&yaml_value)
                {
                    return Err(DetectError::InvalidValue {
                        expected: "numeric or date value".to_string(),
                        found: format!("{:?}", yaml_value),
                        span: value_span.to_source_span(),
                        src: source.to_string(),
                    });
                }

                StructuredDataPredicate::YamlValue {
                    path,
                    operator,
                    value: yaml_value,
                    raw_string,
                }
            }

            DataFormat::Json => {
                let json_value = Self::build_json_rhs(value);

                if is_ordering_operator(operator) && !Self::is_comparable_json(&json_value)
                {
                    return Err(DetectError::InvalidValue {
                        expected: "numeric or date value".to_string(),
                        found: format!("{:?}", json_value),
                        span: value_span.to_source_span(),
                        src: source.to_string(),
                    });
                }

                StructuredDataPredicate::JsonValue {
                    path,
                    operator,
                    value: json_value,
                    raw_string,
                }
            }

            DataFormat::Toml => {
                let toml_value = Self::build_toml_rhs(value);

                if is_ordering_operator(operator) && !Self::is_comparable_toml(&toml_value)
                {
                    return Err(DetectError::InvalidValue {
                        expected: "numeric or date value".to_string(),
                        found: format!("{:?}", toml_value),
                        span: value_span.to_source_span(),
                        src: source.to_string(),
                    });
                }

                StructuredDataPredicate::TomlValue {
                    path,
                    operator,
                    value: toml_value,
                    raw_string,
                }
            }
        };

        Ok(Predicate::structured(predicate))
    }

    /// Build a structured data string predicate (yaml/json/toml with string operations like regex/contains)
    fn build_structured_string_predicate(
        format: typed::DataFormat,
        path: Vec<super::structured_path::PathComponent>,
        string_operator: typed::StringOperator,
        value: &RawValue,
        value_span: pest::Span,
        source: &str,
    ) -> Result<Predicate, DetectError> {
        use crate::predicate::StructuredDataPredicate;
        use typed::DataFormat;

        // Build StringMatcher using existing logic
        let matcher = Self::parse_string_value(value, string_operator, value_span, source)?;

        let predicate = match format {
            DataFormat::Yaml => StructuredDataPredicate::YamlString { path, matcher },
            DataFormat::Json => StructuredDataPredicate::JsonString { path, matcher },
            DataFormat::Toml => StructuredDataPredicate::TomlString { path, matcher },
        };

        Ok(Predicate::structured(predicate))
    }

    /// Build a YAML value from RHS, always attempting parse with string fallback
    ///
    /// Quotes are transparent (only for shell escaping).
    /// Always tries to parse content as YAML, falls back to string literal.
    fn build_yaml_rhs(value: &RawValue) -> yaml_rust::Yaml {
        let content = value.as_string();

        yaml_rust::YamlLoader::load_from_str(content)
            .ok()
            .and_then(|mut docs| docs.pop())
            .unwrap_or_else(|| yaml_rust::Yaml::String(content.to_string()))
    }

    /// Build a JSON value from RHS, always attempting parse with string fallback
    fn build_json_rhs(value: &RawValue) -> serde_json::Value {
        let content = value.as_string();

        serde_json::from_str(content)
            .unwrap_or_else(|_| serde_json::Value::String(content.to_string()))
    }

    /// Build a TOML value from RHS, always attempting parse with string fallback
    fn build_toml_rhs(value: &RawValue) -> toml::Value {
        let content = value.as_string();

        // Try 1: Synthetic document approach
        // Construct synthetic TOML document and parse it
        // This lets TOML parser handle all types (bool, int, float, datetime, arrays, inline tables, etc)
        let synthetic_doc = format!("_v = {}", content);
        if let Ok(parsed) = toml::from_str::<toml::Table>(&synthetic_doc) {
            if let Some(value) = parsed.get("_v") {
                return value.clone();
            }
        }

        // Try 2: Direct TOML value parsing
        // Handles edge cases where synthetic approach fails
        // Example: toml:.foo == "v = 1" → synthetic produces "_v = v = 1" (invalid)
        //          but direct parsing can handle it as a complete document
        if let Ok(value) = toml::from_str::<toml::Value>(content) {
            return value;
        }

        // Try 3: Fallback to string
        toml::Value::String(content.to_string())
    }

    /// Check if a YAML value is comparable (numeric or date-like)
    fn is_comparable_yaml(value: &yaml_rust::Yaml) -> bool {
        matches!(
            value,
            yaml_rust::Yaml::Integer(_) | yaml_rust::Yaml::Real(_)
        )
    }

    /// Check if a JSON value is comparable
    fn is_comparable_json(value: &serde_json::Value) -> bool {
        value.is_number()
    }

    /// Check if a TOML value is comparable
    fn is_comparable_toml(value: &toml::Value) -> bool {
        matches!(
            value,
            toml::Value::Integer(_) | toml::Value::Float(_) | toml::Value::Datetime(_)
        )
    }

    /// Parse string value based on operator type
    fn parse_string_value(
        value: &RawValue,
        operator: StringOperator,
        value_span: pest::Span,
        source: &str,
    ) -> Result<StringMatcher, DetectError> {
        // Extract the raw string regardless of Quoted or Raw variant
        let value_str = match value {
            RawValue::Quoted(s) | RawValue::Raw(s) => s,
        };

        // For 'in' operator, parse as set
        if matches!(operator, StringOperator::In) {
            return Self::parse_as_set(value_str, value_span, source);
        }

        // For other operators, use as string pattern (literal or regex)
        match operator {
            StringOperator::Equals => Ok(StringMatcher::Equals(value_str.to_string())),
            StringOperator::NotEquals => Ok(StringMatcher::NotEquals(value_str.to_string())),
            StringOperator::Matches => {
                StringMatcher::regex(value_str).map_err(|e| DetectError::InvalidValue {
                    expected: "valid regex pattern".to_string(),
                    found: format!("{}: {}", value_str, e),
                    span: value_span.to_source_span(),
                    src: source.to_string(),
                })
            }
            StringOperator::Contains => Ok(StringMatcher::Contains(value_str.to_string())),
            StringOperator::In => unreachable!("Handled above"),
        }
    }

    /// Parse a value as a set - handles bracketed syntax and comma separation
    ///
    /// Uses dedicated Pest parser to properly handle:
    /// - Quoted items with commas: `["foo, bar", baz]`
    /// - Escaped quotes: `["foo\"bar", 'baz\'qux']`
    /// - Trailing commas: `[rs, js,]`
    /// - Bare comma-separated: `rs,js,ts`
    fn parse_as_set(
        value_str: &str,
        value_span: pest::Span,
        source: &str,
    ) -> Result<StringMatcher, DetectError> {
        // Strip brackets if present, otherwise use whole string
        let inner = if value_str.starts_with('[') && value_str.ends_with(']') {
            &value_str[1..value_str.len() - 1]
        } else {
            value_str
        };

        // Use dedicated set parser for proper handling
        use crate::parser::RawParser;
        let items =
            RawParser::parse_set_contents(inner).map_err(|e| DetectError::InvalidValue {
                expected: "valid set items (e.g., [rs, js] or \"foo, bar\", baz)".to_string(),
                found: format!("parse error: {}", e),
                span: value_span.to_source_span(),
                src: source.to_string(),
            })?;

        let set: std::collections::HashSet<String> = items.into_iter().collect();
        Ok(StringMatcher::In(set))
    }

    /// Build content pattern based on operator
    fn build_content_pattern(
        value: &RawValue,
        operator: StringOperator,
        _value_span: pest::Span,
        source: &str,
    ) -> Result<String, DetectError> {
        // Extract string value
        let s = match value {
            RawValue::Quoted(s) | RawValue::Raw(s) => s,
        };

        let pattern = match operator {
            StringOperator::Equals => format!("^{}$", regex::escape(s)),
            StringOperator::Matches => s.to_string(),
            StringOperator::Contains => regex::escape(s),
            _ => {
                return Err(DetectError::Internal {
                    message: "Invalid operator for contents".to_string(),
                    src: source.to_string(),
                })
            }
        };

        Ok(pattern)
    }

    /// Parse numeric value, handling size units if applicable
    fn parse_numeric_value(
        value: &RawValue,
        selector: &NumericSelector,
        value_span: pest::Span,
        source: &str,
    ) -> Result<u64, DetectError> {
        let s = match value {
            RawValue::Quoted(s) | RawValue::Raw(s) => s,
        };

        if matches!(selector, NumericSelector::Size) && s.chars().any(|c| c.is_alphabetic()) {
            // Parse as size with unit
            parse_size_value(s, value_span, source)
        } else {
            // Parse as plain number
            s.parse().map_err(|_| DetectError::InvalidValue {
                expected: "numeric value".to_string(),
                found: s.to_string(),
                span: value_span.to_source_span(),
                src: source.to_string(),
            })
        }
    }

    /// Build number matcher from operator and value
    fn build_number_matcher(operator: NumericOperator, value: u64) -> NumberMatcher {
        match operator {
            NumericOperator::Equals => NumberMatcher::Equals(value),
            NumericOperator::NotEquals => NumberMatcher::NotEquals(value),
            NumericOperator::Greater => NumberMatcher::In(Bound::Left((value + 1)..)),
            NumericOperator::GreaterOrEqual => NumberMatcher::In(Bound::Left(value..)),
            NumericOperator::Less => NumberMatcher::In(Bound::Right(..value)),
            NumericOperator::LessOrEqual => NumberMatcher::In(Bound::Right(..(value + 1))),
        }
    }

    /// Parse temporal value
    fn parse_temporal_value(
        value: &RawValue,
        value_span: pest::Span,
        source: &str,
    ) -> Result<chrono::DateTime<chrono::Local>, DetectError> {
        let s = match value {
            RawValue::Quoted(s) | RawValue::Raw(s) => s,
        };

        parse_time_value(s).map_err(|e| DetectError::InvalidValue {
            expected: "valid time value".to_string(),
            found: format!("{}: {:?}", s, e),
            span: value_span.to_source_span(),
            src: source.to_string(),
        })
    }

    /// Build time matcher from operator and value
    fn build_time_matcher(
        operator: TemporalOperator,
        value: chrono::DateTime<chrono::Local>,
    ) -> TimeMatcher {
        match operator {
            TemporalOperator::Equals => TimeMatcher::Equals(value),
            TemporalOperator::NotEquals => TimeMatcher::NotEquals(value),
            TemporalOperator::After => TimeMatcher::After(value),
            TemporalOperator::Before => TimeMatcher::Before(value),
            TemporalOperator::AfterOrEqual => TimeMatcher::AfterOrEqual(value),
            TemporalOperator::BeforeOrEqual => TimeMatcher::BeforeOrEqual(value),
        }
    }

    /// Parse and validate enum values at parse time using the EnumPredicate trait
    fn parse_enum_value<E: EnumPredicate>(
        value: &RawValue,
        operator: EnumOperator,
        value_span: pest::Span,
        source: &str,
    ) -> Result<EnumMatcher<E>, DetectError> {
        let value_str = match value {
            RawValue::Quoted(s) | RawValue::Raw(s) => s,
        };

        match operator {
            EnumOperator::Equals => {
                let variant =
                    E::from_str(value_str).map_err(|_err_msg| DetectError::InvalidValue {
                        expected: format!("one of: {}", E::all_valid_strings().join(", ")),
                        found: value_str.to_string(),
                        span: value_span.to_source_span(),
                        src: source.to_string(),
                    })?;
                Ok(EnumMatcher::Equals(variant))
            }

            EnumOperator::NotEquals => {
                let variant =
                    E::from_str(value_str).map_err(|_err_msg| DetectError::InvalidValue {
                        expected: format!("one of: {}", E::all_valid_strings().join(", ")),
                        found: value_str.to_string(),
                        span: value_span.to_source_span(),
                        src: source.to_string(),
                    })?;
                Ok(EnumMatcher::NotEquals(variant))
            }

            EnumOperator::In => {
                // Reuse existing set parsing logic, then validate each item
                let string_matcher = Self::parse_as_set(value_str, value_span, source)?;

                // Extract strings from the StringMatcher::In variant
                let items = match string_matcher {
                    StringMatcher::In(set) => set,
                    _ => unreachable!("parse_as_set should return StringMatcher::In"),
                };

                // Validate each string and convert to enum variant
                let mut variant_set = std::collections::HashSet::new();
                for item in items {
                    let variant =
                        E::from_str(&item).map_err(|_err_msg| DetectError::InvalidValue {
                            expected: format!("one of: {}", E::all_valid_strings().join(", ")),
                            found: item.clone(),
                            span: value_span.to_source_span(),
                            src: source.to_string(),
                        })?;
                    variant_set.insert(variant);
                }
                Ok(EnumMatcher::In(variant_set))
            }
        }
    }
}
