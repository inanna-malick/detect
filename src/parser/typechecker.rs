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
    let s = s.trim().to_lowercase();

    // Find where the unit starts
    let mut unit_start = 0;
    for (i, ch) in s.char_indices() {
        if !ch.is_ascii_digit() && ch != '.' {
            unit_start = i;
            break;
        }
    }

    if unit_start == 0 {
        return Err(TypecheckError::InvalidValue {
            expected: "size with unit (e.g., 1mb, 100kb)".to_string(),
            found: s,
            span: value_span.to_source_span(),
            src: source.to_string(),
        });
    }

    let number_str = &s[..unit_start];
    let unit_str = &s[unit_start..];

    let number: f64 = number_str
        .parse()
        .map_err(|_| TypecheckError::InvalidValue {
            expected: "numeric value".to_string(),
            found: number_str.to_string(),
            span: value_span.to_source_span(),
            src: source.to_string(),
        })?;

    let multiplier = match unit_str {
        "b" | "byte" | "bytes" => 1.0,
        "k" | "kb" | "kilobyte" | "kilobytes" => 1024.0,
        "m" | "mb" | "megabyte" | "megabytes" => 1024.0 * 1024.0,
        "g" | "gb" | "gigabyte" | "gigabytes" => 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" | "terabyte" | "terabytes" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => {
            return Err(TypecheckError::InvalidValue {
                expected: "size unit (b, kb, mb, gb, tb)".to_string(),
                found: unit_str.to_string(),
                span: value_span.to_source_span(),
                src: source.to_string(),
            })
        }
    };

    Ok((number * multiplier) as u64)
}

/// Main typechecker that transforms raw AST to typed expressions
pub struct Typechecker;

impl Typechecker {
    /// Transform a raw expression into a typed expression
    ///
    /// # Errors
    /// Returns `DetectError` for syntax errors, unknown selectors, incompatible operators, or invalid values.
    pub fn typecheck(raw_expr: RawExpr<'_>, source: &str) -> Result<Expr<Predicate>, DetectError> {
        Self::typecheck_inner(raw_expr, source)
    }

    fn typecheck_inner(
        raw_expr: RawExpr<'_>,
        source: &str,
    ) -> Result<Expr<Predicate>, DetectError> {
        match raw_expr {
            RawExpr::Predicate(pred) => {
                let typed_pred = Self::typecheck_predicate(pred, source)?;
                Ok(Expr::Predicate(typed_pred))
            }
            RawExpr::And(lhs, rhs) => {
                let typed_lhs = Self::typecheck_inner(*lhs, source)?;
                let typed_rhs = Self::typecheck_inner(*rhs, source)?;
                Ok(Expr::and(typed_lhs, typed_rhs))
            }
            RawExpr::Or(lhs, rhs) => {
                let typed_lhs = Self::typecheck_inner(*lhs, source)?;
                let typed_rhs = Self::typecheck_inner(*rhs, source)?;
                Ok(Expr::or(typed_lhs, typed_rhs))
            }
            RawExpr::Not(expr) => {
                let typed_expr = Self::typecheck_inner(*expr, source)?;
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

    /// Transform a raw predicate into a typed predicate using the new type system
    fn typecheck_predicate(pred: RawPredicate<'_>, source: &str) -> Result<Predicate, DetectError> {
        // Parse selector and operator together with type safety, passing spans
        let typed_selector = typed::parse_selector_operator(
            pred.selector,
            pred.selector_span,
            pred.operator,
            pred.operator_span,
            source,
        )?;

        match typed_selector {
            TypedSelector::String(selector, operator) => Self::build_string_predicate(
                selector,
                operator,
                &pred.value,
                pred.value_span,
                source,
            ),
            TypedSelector::Numeric(selector, operator) => Self::build_numeric_predicate(
                selector,
                operator,
                &pred.value,
                pred.value_span,
                source,
            ),
            TypedSelector::Temporal(selector, operator) => Self::build_temporal_predicate(
                selector,
                operator,
                &pred.value,
                pred.value_span,
                source,
            ),
            TypedSelector::Enum(selector, operator) => {
                Self::build_enum_predicate(selector, operator, &pred.value, pred.value_span, source)
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
