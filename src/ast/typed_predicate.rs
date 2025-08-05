//! TypedPredicate implementation for AST parsing

use pest::iterators::{Pair, Pairs};
use std::ops::{RangeFrom, RangeTo};

use super::operators::{NumericOp, StringOp, TemporalOp};
use super::parse_helpers::ParseIterExt;
use super::selectors::{NumericSelectorType, StringSelectorType, TemporalSelectorType};

use crate::expr::Expr;
use crate::parse_error::{ParseError, PredicateParseError, StructureErrorKind};
use crate::parser::pratt_parser::Rule;
use crate::predicate::{
    Bound, MetadataPredicate, NamePredicate, NumberMatcher, Predicate as DomainPredicate,
    StreamingCompiledContentPredicate, StringMatcher, TimeMatcher,
};

/// Represents a typed predicate parsed from the grammar
pub enum TypedPredicate {
    String {
        selector: StringSelectorType,
        op: StringOp,
        value: String,
    },
    Numeric {
        selector: NumericSelectorType,
        op: NumericOp,
        value: u64,
    },
    Temporal {
        selector: TemporalSelectorType,
        op: TemporalOp,
        value: String,
    },
    Set {
        selector: StringSelectorType,
        items: Vec<String>,
    },
}

impl TypedPredicate {
    /// Parse numeric operator and value from a pair
    fn parse_numeric_op_raw(
        pair: Pair<'_, Rule>,
    ) -> Result<(NumericOp, Option<Pair<'_, Rule>>), ParseError> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::numeric_eq => {
                inner.next(); // skip operator
                Ok((NumericOp::Equals, inner.next()))
            }
            Rule::numeric_ne => {
                inner.next();
                Ok((NumericOp::NotEquals, inner.next()))
            }
            Rule::numeric_comparison => {
                let op_pair = inner.next().ok_or(ParseError::Internal(
                    "grammar guarantees numeric_comparison has operator",
                ))?;
                let op = match op_pair.as_rule() {
                    Rule::gt => NumericOp::Greater,
                    Rule::gteq => NumericOp::GreaterOrEqual,
                    Rule::lt => NumericOp::Less,
                    Rule::lteq => NumericOp::LessOrEqual,
                    rule => return Err(ParseError::unexpected_rule(rule, None)),
                };
                Ok((op, inner.next()))
            }
            rule => Err(ParseError::unexpected_rule(rule, None)),
        }
    }

    /// Parse temporal operator and value from a pair
    fn parse_temporal_op_raw(
        pair: Pair<'_, Rule>,
    ) -> Result<(TemporalOp, Option<Pair<'_, Rule>>), ParseError> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::temporal_eq => {
                inner.next();
                Ok((TemporalOp::Equals, inner.next()))
            }
            Rule::temporal_ne => {
                inner.next();
                Ok((TemporalOp::NotEquals, inner.next()))
            }
            Rule::temporal_comparison => {
                let op_pair = inner.next().ok_or(ParseError::Internal(
                    "grammar guarantees temporal_comparison has operator",
                ))?;
                let op = match op_pair.as_rule() {
                    Rule::gt | Rule::gteq => TemporalOp::After,
                    Rule::lt | Rule::lteq => TemporalOp::Before,
                    rule => return Err(ParseError::unexpected_rule(rule, None)),
                };
                Ok((op, inner.next()))
            }
            rule => Err(ParseError::unexpected_rule(rule, None)),
        }
    }

    /// Parse set items from a set literal
    fn parse_set_items(mut pairs: Pairs<'_, Rule>) -> Result<Vec<String>, ParseError> {
        let mut items = Vec::new();
        if let Some(set_literal) = pairs.next() {
            for set_items in set_literal.into_inner() {
                for item in set_items.into_inner() {
                    if item.as_rule() == Rule::set_item {
                        let inner_item = item.into_inner().next().ok_or(ParseError::Internal(
                            "grammar guarantees set_item has value",
                        ))?;
                        let value = match inner_item.as_rule() {
                            Rule::quoted_string => Self::extract_string_value(inner_item)?,
                            Rule::set_token => inner_item.as_str().to_string(),
                            _ => continue,
                        };
                        items.push(value);
                    }
                }
            }
        }
        Ok(items)
    }

    /// Helper to convert path rules to selector types
    fn path_rule_to_selector(rule: Rule) -> Option<StringSelectorType> {
        match rule {
            Rule::path_full => Some(StringSelectorType::PathFull),
            Rule::path_parent => Some(StringSelectorType::PathParent),
            Rule::path_parent_dir => Some(StringSelectorType::PathParentDir),
            Rule::path_name => Some(StringSelectorType::PathName),
            Rule::path_stem => Some(StringSelectorType::PathStem),
            Rule::path_extension => Some(StringSelectorType::PathSuffix),
            _ => None,
        }
    }

    /// Parse a typed predicate from a Pest pair
    pub fn from_pair(pair: Pair<'_, Rule>) -> Result<Self, ParseError> {
        let span = pair.as_span();
        let mut inner = pair.into_inner();
        let inner = inner.expect_next("typed_predicate")?;

        match inner.as_rule() {
            Rule::string_predicate => Self::parse_string_predicate(inner),
            Rule::numeric_predicate => Self::parse_numeric_predicate(inner),
            Rule::temporal_predicate => Self::parse_temporal_predicate(inner),
            rule => Err(ParseError::unexpected_rule(
                rule,
                Some((span.start_pos().line_col().0, span.start_pos().line_col().1)),
            )),
        }
    }

    fn parse_string_predicate(pair: Pair<'_, Rule>) -> Result<Self, ParseError> {
        let mut inner = pair.into_inner();

        // Parse selector
        let selector_pair = inner.expect_next("string_predicate")?;
        let selector = Self::parse_string_selector(selector_pair)?;

        // Parse operator and value
        let op_value_pair = inner.expect_next("string_predicate")?;

        let (op, value, set_items) = Self::parse_string_op_value(op_value_pair)?;

        if let Some(items) = set_items {
            Ok(TypedPredicate::Set { selector, items })
        } else {
            Ok(TypedPredicate::String {
                selector,
                op,
                value,
            })
        }
    }

    fn parse_string_selector(pair: Pair<'_, Rule>) -> Result<StringSelectorType, ParseError> {
        match pair.as_rule() {
            Rule::string_selector => {
                // Unwrap the string_selector wrapper
                let mut inner_pairs = pair.into_inner();
                let inner = inner_pairs.expect_next("string_selector")?;
                Self::parse_string_selector(inner)
            }
            Rule::bare_path_shorthand => {
                // Handle bare path shorthands
                let mut inner = pair.into_inner();
                let shorthand = inner.expect_next("bare_path_shorthand")?;
                match shorthand.as_rule() {
                    Rule::bare_name => Ok(StringSelectorType::PathName),
                    Rule::bare_stem => Ok(StringSelectorType::PathStem),
                    Rule::bare_extension => Ok(StringSelectorType::PathSuffix), // Maps to PathSuffix internally
                    Rule::bare_parent => Ok(StringSelectorType::PathParent),
                    Rule::bare_full => Ok(StringSelectorType::PathFull),
                    rule => Err(ParseError::unexpected_rule(rule, None)),
                }
            }
            Rule::path_selector => {
                // Path selector has nested structure
                let mut path_inner = pair.into_inner();
                let first = path_inner.expect_next("path_selector")?;

                match first.as_rule() {
                    Rule::path_alias => Ok(StringSelectorType::PathFull),
                    Rule::path_with_component => {
                        // Find the component and convert it to selector type
                        first
                            .into_inner()
                            .find_map(|t| Self::path_rule_to_selector(t.as_rule()))
                            .ok_or(ParseError::Internal(
                                "grammar guarantees path_with_component has valid component",
                            ))
                    }
                    rule => Err(ParseError::Structure {
                        kind: StructureErrorKind::UnexpectedRule { rule },
                        location: None,
                    }),
                }
            }
            Rule::content_selector => {
                // Handle content selector with possible domain
                let mut inner = pair.into_inner();
                let content_part = inner.expect_next("content_selector")?;
                match content_part.as_rule() {
                    Rule::content_with_domain => Ok(StringSelectorType::Contents),
                    Rule::bare_content => Ok(StringSelectorType::Contents),
                    rule => Err(ParseError::unexpected_rule(rule, None)),
                }
            }
            Rule::type_selector => {
                // Handle type selector with possible meta domain
                let mut inner = pair.into_inner();
                let type_part = inner.expect_next("type_selector")?;
                match type_part.as_rule() {
                    Rule::meta_type => Ok(StringSelectorType::Type),
                    Rule::bare_type => {
                        // bare_type wraps type, need to unwrap
                        Ok(StringSelectorType::Type)
                    }
                    rule => Err(ParseError::unexpected_rule(rule, None)),
                }
            }
            Rule::r#type => Ok(StringSelectorType::Type), // Keep for backward compat
            rule => Err(ParseError::unexpected_rule(rule, None)),
        }
    }

    fn parse_string_op_value(
        pair: Pair<'_, Rule>,
    ) -> Result<(StringOp, String, Option<Vec<String>>), ParseError> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::string_eq => {
                inner.next(); // skip the eq operator
                let value = Self::extract_string_value_from_pairs(inner)?;
                Ok((StringOp::Equals, value, None))
            }
            Rule::string_ne => {
                inner.next(); // skip the ne operator
                let value = Self::extract_string_value_from_pairs(inner)?;
                Ok((StringOp::NotEquals, value, None))
            }
            Rule::string_contains => {
                inner.next(); // skip the contains operator
                let value = Self::extract_string_value_from_pairs(inner)?;
                Ok((StringOp::Contains, value, None))
            }
            Rule::string_regex => {
                inner.next(); // skip the regex operator
                let value = Self::extract_string_value_from_pairs(inner)?;
                Ok((StringOp::Regex, value, None))
            }
            Rule::string_in => {
                inner.next(); // skip the in operator
                let items = Self::parse_set_items(inner)?;
                Ok((StringOp::Equals, String::new(), Some(items)))
            }
            rule => Err(ParseError::unexpected_rule(rule, None)),
        }
    }

    fn extract_string_value_from_pairs(mut pairs: Pairs<'_, Rule>) -> Result<String, ParseError> {
        let pair = pairs.next().ok_or(ParseError::Internal(
            "grammar guarantees string value exists",
        ))?;
        Self::extract_string_value(pair)
    }

    fn extract_string_value(pair: Pair<'_, Rule>) -> Result<String, ParseError> {
        match pair.as_rule() {
            Rule::string_value => {
                let inner = pair.into_inner().next().ok_or(ParseError::Internal(
                    "grammar guarantees string_value has content",
                ))?;
                Self::extract_string_value(inner)
            }
            Rule::quoted_string => Self::extract_quoted_string(pair),
            Rule::bare_string => Ok(pair.as_str().to_string()),
            _ => Err(ParseError::unexpected_rule(pair.as_rule(), None)),
        }
    }

    fn extract_quoted_string(pair: Pair<'_, Rule>) -> Result<String, ParseError> {
        let quoted = pair.into_inner().next().ok_or(ParseError::Internal(
            "grammar guarantees quoted_string has content",
        ))?;
        let inner_str = quoted.into_inner().next().ok_or(ParseError::Internal(
            "grammar guarantees quoted content has string",
        ))?;
        Ok(inner_str.as_str().to_string())
    }

    fn parse_numeric_predicate(pair: Pair<'_, Rule>) -> Result<Self, ParseError> {
        let mut inner = pair.into_inner();

        // Parse selector (size or depth)
        let selector_pair = inner.next().ok_or(ParseError::Internal(
            "grammar guarantees numeric_predicate has selector",
        ))?;

        let selector = match selector_pair.as_rule() {
            Rule::numeric_selector => {
                // Unwrap the numeric_selector wrapper
                let inner = selector_pair
                    .into_inner()
                    .next()
                    .ok_or(ParseError::Internal(
                        "grammar guarantees numeric_selector has type",
                    ))?;
                match inner.as_rule() {
                    Rule::meta_size | Rule::bare_size => NumericSelectorType::Size,
                    Rule::meta_depth | Rule::bare_depth => NumericSelectorType::Depth,
                    rule => return Err(ParseError::unexpected_rule(rule, None)),
                }
            }
            Rule::size => NumericSelectorType::Size,
            Rule::depth => NumericSelectorType::Depth,
            rule => return Err(ParseError::unexpected_rule(rule, None)),
        };

        // Parse operator and value
        let op_value_pair = inner.next().ok_or(ParseError::Internal(
            "grammar guarantees numeric_predicate has op_value",
        ))?;

        let (op, value) = Self::parse_numeric_op_value(op_value_pair)?;
        Ok(TypedPredicate::Numeric {
            selector,
            op,
            value,
        })
    }

    fn parse_numeric_op_value(pair: Pair<'_, Rule>) -> Result<(NumericOp, u64), ParseError> {
        let (op, value_pair) = Self::parse_numeric_op_raw(pair)?;
        let value_pair = value_pair.ok_or(ParseError::Internal(
            "grammar guarantees numeric operator has value",
        ))?;
        let value = Self::parse_numeric_value(value_pair)?;
        Ok((op, value))
    }

    fn parse_numeric_value(pair: Pair<'_, Rule>) -> Result<u64, ParseError> {
        match pair.as_rule() {
            Rule::numeric_value => {
                let inner = pair.into_inner().next().ok_or(ParseError::Internal(
                    "grammar guarantees numeric_value has content",
                ))?;
                Self::parse_numeric_value(inner)
            }
            Rule::size_value => crate::parser::parse_size_value_as_bytes(pair),
            Rule::bare_number => pair
                .as_str()
                .parse()
                .map_err(|_| ParseError::invalid_token("numeric value", pair.as_str())),
            rule => Err(ParseError::unexpected_rule(rule, None)),
        }
    }

    fn parse_temporal_predicate(pair: Pair<'_, Rule>) -> Result<Self, ParseError> {
        let mut inner = pair.into_inner();

        // Parse selector
        let selector_pair = inner.next().ok_or(ParseError::Internal(
            "grammar guarantees temporal_predicate has selector",
        ))?;

        let selector = match selector_pair.as_rule() {
            Rule::temporal_selector => {
                // Unwrap the temporal_selector wrapper
                let inner = selector_pair
                    .into_inner()
                    .next()
                    .ok_or(ParseError::Internal(
                        "grammar guarantees temporal_selector has type",
                    ))?;
                match inner.as_rule() {
                    Rule::time_with_domain => {
                        // Extract the actual time component after "time."
                        let time_component =
                            inner.into_inner().next().ok_or(ParseError::Internal(
                                "grammar guarantees time_with_domain has component",
                            ))?;
                        match time_component.as_rule() {
                            Rule::modified => TemporalSelectorType::Modified,
                            Rule::created => TemporalSelectorType::Created,
                            Rule::accessed => TemporalSelectorType::Accessed,
                            rule => return Err(ParseError::unexpected_rule(rule, None)),
                        }
                    }
                    Rule::bare_time => {
                        // Handle bare time selector
                        let time_component = inner.into_inner().next().ok_or(
                            ParseError::Internal("grammar guarantees bare_time has component"),
                        )?;
                        match time_component.as_rule() {
                            Rule::modified => TemporalSelectorType::Modified,
                            Rule::created => TemporalSelectorType::Created,
                            Rule::accessed => TemporalSelectorType::Accessed,
                            rule => return Err(ParseError::unexpected_rule(rule, None)),
                        }
                    }
                    Rule::modified => TemporalSelectorType::Modified,
                    Rule::created => TemporalSelectorType::Created,
                    Rule::accessed => TemporalSelectorType::Accessed,
                    rule => return Err(ParseError::unexpected_rule(rule, None)),
                }
            }
            Rule::modified => TemporalSelectorType::Modified,
            Rule::created => TemporalSelectorType::Created,
            Rule::accessed => TemporalSelectorType::Accessed,
            rule => return Err(ParseError::unexpected_rule(rule, None)),
        };

        // Parse operator and value
        let op_value_pair = inner.next().ok_or(ParseError::Internal(
            "grammar guarantees temporal_predicate has op_value",
        ))?;

        let (op, value) = Self::parse_temporal_op_value(op_value_pair)?;
        Ok(TypedPredicate::Temporal {
            selector,
            op,
            value,
        })
    }

    fn parse_temporal_op_value(pair: Pair<'_, Rule>) -> Result<(TemporalOp, String), ParseError> {
        let (op, value_pair) = Self::parse_temporal_op_raw(pair)?;
        let value_pair = value_pair.ok_or(ParseError::Internal(
            "grammar guarantees temporal operator has value",
        ))?;
        let value = Self::parse_temporal_value(value_pair)?;
        Ok((op, value))
    }

    fn parse_temporal_value(pair: Pair<'_, Rule>) -> Result<String, ParseError> {
        match pair.as_rule() {
            Rule::temporal_value => {
                let inner = pair.into_inner().next().ok_or(ParseError::Internal(
                    "grammar guarantees temporal_value has content",
                ))?;
                Self::parse_temporal_value(inner)
            }
            Rule::absolute_date => Ok(pair.as_str().to_string()),
            Rule::relaxed_time_value => Ok(pair.as_str().to_string()),
            Rule::time_value => Ok(pair.as_str().to_string()),
            Rule::quoted_string => Self::extract_string_value(pair),
            Rule::time_keyword => Ok(pair.as_str().to_string()),
            rule => Err(ParseError::unexpected_rule(rule, None)),
        }
    }

    /// Convert to domain expression type
    pub fn into_expr(
        self,
    ) -> Result<
        Expr<DomainPredicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
        ParseError,
    > {
        match self {
            TypedPredicate::String {
                selector,
                op,
                value,
            } => {
                let string_matcher = match op {
                    StringOp::Equals => StringMatcher::Equals(value.clone()),
                    StringOp::NotEquals => StringMatcher::NotEquals(value.clone()),
                    StringOp::Contains => StringMatcher::Contains(value.clone()),
                    StringOp::Regex => {
                        // Special case for '*' which users commonly expect to work
                        let pattern = if value == "*" { ".*" } else { &value };
                        StringMatcher::regex(pattern).map_err(|e| ParseError::Predicate {
                            selector: crate::predicate::Selector::BaseName,
                            operator: crate::predicate::Op::Matches,
                            value: crate::predicate::RhsValue::String(value.clone()),
                            source: PredicateParseError::Regex(e),
                        })?
                    }
                };

                let predicate = match selector {
                    StringSelectorType::PathFull => {
                        DomainPredicate::name(NamePredicate::FullPath(string_matcher))
                    }
                    StringSelectorType::PathParent => {
                        DomainPredicate::name(NamePredicate::DirPath(string_matcher))
                    }
                    StringSelectorType::PathParentDir => {
                        DomainPredicate::name(NamePredicate::ParentDir(string_matcher))
                    }
                    StringSelectorType::PathName => {
                        DomainPredicate::name(NamePredicate::FileName(string_matcher))
                    }
                    StringSelectorType::PathStem => {
                        DomainPredicate::name(NamePredicate::BaseName(string_matcher))
                    }
                    StringSelectorType::PathSuffix => {
                        DomainPredicate::name(NamePredicate::Extension(string_matcher))
                    }
                    StringSelectorType::Type => {
                        DomainPredicate::meta(MetadataPredicate::Type(string_matcher))
                    }
                    StringSelectorType::Contents => {
                        let (pattern, negate) = match &string_matcher {
                            StringMatcher::Regex(r) => (r.as_str().to_string(), false),
                            StringMatcher::Equals(s) => (format!("^{}$", regex::escape(s)), false),
                            StringMatcher::Contains(s) => (regex::escape(s), false),
                            StringMatcher::NotEquals(s) => {
                                (format!("^{}$", regex::escape(s)), true)
                            }
                            _ => {
                                return Err(ParseError::invalid_token(
                                    "regex, equals, contains, or not-equals for contents",
                                    format!("{:?}", string_matcher),
                                ))
                            }
                        };
                        let content_pred =
                            StreamingCompiledContentPredicate::new_with_negate(pattern, negate)
                                .map_err(|_| {
                                    ParseError::invalid_token("valid regex pattern for DFA", value)
                                })?;
                        DomainPredicate::contents(content_pred)
                    }
                };
                Ok(Expr::Predicate(predicate))
            }
            TypedPredicate::Set { selector, items } => {
                let string_matcher = StringMatcher::In(items.into_iter().collect());
                let predicate = match selector {
                    StringSelectorType::PathFull => {
                        DomainPredicate::name(NamePredicate::FullPath(string_matcher))
                    }
                    StringSelectorType::PathParent => {
                        DomainPredicate::name(NamePredicate::DirPath(string_matcher))
                    }
                    StringSelectorType::PathParentDir => {
                        DomainPredicate::name(NamePredicate::ParentDir(string_matcher))
                    }
                    StringSelectorType::PathName => {
                        DomainPredicate::name(NamePredicate::FileName(string_matcher))
                    }
                    StringSelectorType::PathStem => {
                        DomainPredicate::name(NamePredicate::BaseName(string_matcher))
                    }
                    StringSelectorType::PathSuffix => {
                        DomainPredicate::name(NamePredicate::Extension(string_matcher))
                    }
                    StringSelectorType::Type => {
                        DomainPredicate::meta(MetadataPredicate::Type(string_matcher))
                    }
                    _ => {
                        return Err(ParseError::invalid_token(
                            "path or type selector for 'in' operator",
                            format!("{:?}", selector),
                        ))
                    }
                };
                Ok(Expr::Predicate(predicate))
            }
            TypedPredicate::Numeric {
                selector,
                op,
                value,
            } => {
                let number_matcher = match op {
                    NumericOp::Equals => NumberMatcher::Equals(value),
                    NumericOp::NotEquals => NumberMatcher::NotEquals(value),
                    NumericOp::Greater => {
                        NumberMatcher::In(Bound::Left(RangeFrom { start: value + 1 }))
                    }
                    NumericOp::GreaterOrEqual => {
                        NumberMatcher::In(Bound::Left(RangeFrom { start: value }))
                    }
                    NumericOp::Less => NumberMatcher::In(Bound::Right(RangeTo { end: value })),
                    NumericOp::LessOrEqual => {
                        NumberMatcher::In(Bound::Right(RangeTo { end: value + 1 }))
                    }
                };
                let predicate = match selector {
                    NumericSelectorType::Size => {
                        DomainPredicate::meta(MetadataPredicate::Filesize(number_matcher))
                    }
                    NumericSelectorType::Depth => {
                        DomainPredicate::meta(MetadataPredicate::Depth(number_matcher))
                    }
                };
                Ok(Expr::Predicate(predicate))
            }
            TypedPredicate::Temporal {
                selector,
                op,
                value,
            } => {
                let parsed_time = crate::predicate::parse_time_value(&value)
                    .map_err(|_| ParseError::invalid_token("valid time value", value))?;

                let time_matcher = match op {
                    TemporalOp::Equals => TimeMatcher::Equals(parsed_time),
                    TemporalOp::NotEquals => TimeMatcher::NotEquals(parsed_time),
                    TemporalOp::Before => TimeMatcher::Before(parsed_time),
                    TemporalOp::After => TimeMatcher::After(parsed_time),
                };

                let predicate = match selector {
                    TemporalSelectorType::Modified => {
                        DomainPredicate::meta(MetadataPredicate::Modified(time_matcher))
                    }
                    TemporalSelectorType::Created => {
                        DomainPredicate::meta(MetadataPredicate::Created(time_matcher))
                    }
                    TemporalSelectorType::Accessed => {
                        DomainPredicate::meta(MetadataPredicate::Accessed(time_matcher))
                    }
                };
                Ok(Expr::Predicate(predicate))
            }
        }
    }
}
