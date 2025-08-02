//! AST types for detect expression language
//!
//! This module uses a hybrid approach:
//! - Simple structures are parsed with pest-ast derives
//! - Complex predicates are parsed manually
//! - Expressions use PrattParser for precedence

use pest::iterators::{Pair, Pairs};
use pest::Span;
use pest_ast::*;
use std::ops::{RangeFrom, RangeTo};

use crate::expr::Expr;
use crate::parse_error::{ParseError, PredicateParseError, StructureErrorKind};
use crate::parser::pratt_parser::Rule;
use crate::predicate::{
    Bound, MetadataPredicate, NamePredicate, NumberMatcher, Predicate as DomainPredicate,
    StreamingCompiledContentPredicate, StringMatcher, TimeMatcher,
};

// Extension trait for better error handling with iterators
trait ParseIterExt<'i> {
    fn expect_next(&mut self, context: &'static str) -> Result<Pair<'i, Rule>, ParseError>;
}

impl<'i> ParseIterExt<'i> for Pairs<'i, Rule> {
    fn expect_next(&mut self, context: &'static str) -> Result<Pair<'i, Rule>, ParseError> {
        self.next().ok_or(ParseError::Structure {
            kind: StructureErrorKind::MissingToken {
                expected: "token",
                context,
            },
            location: None,
        })
    }
}

// Helper for extracting string from span
fn span_to_string(span: Span) -> String {
    span.as_str().to_string()
}

// ============================================================================
// Simple Value Types (can use pest-ast)
// ============================================================================

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::bare_string))]
pub struct BareString {
    #[pest_ast(outer(with(span_to_string)))]
    pub value: String,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::bare_number))]
pub struct BareNumber {
    #[pest_ast(outer(with(span_to_string)))]
    pub value: String,
}

#[derive(Debug, FromPest)]
#[pest_ast(rule(Rule::time_keyword))]
pub struct TimeKeyword {
    #[pest_ast(outer(with(span_to_string)))]
    pub value: String,
}

// ============================================================================
// Manual implementations for complex predicates
// ============================================================================

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringSelectorType {
    PathFull,
    PathParent,
    PathName,
    PathStem,
    PathSuffix,
    Contents,
    Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumericSelectorType {
    Size,
    Depth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringOp {
    Equals,
    NotEquals,
    Contains,
    Regex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericOp {
    Equals,
    NotEquals,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalSelectorType {
    Modified,
    Created,
    Accessed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalOp {
    Equals,
    NotEquals,
    Before,
    After,
}

impl TypedPredicate {
    /// Helper to convert path rules to selector types
    fn path_rule_to_selector(rule: Rule) -> Option<StringSelectorType> {
        match rule {
            Rule::path_full => Some(StringSelectorType::PathFull),
            Rule::path_parent => Some(StringSelectorType::PathParent),
            Rule::path_name => Some(StringSelectorType::PathName),
            Rule::path_stem => Some(StringSelectorType::PathStem),
            Rule::path_extension | Rule::path_suffix => Some(StringSelectorType::PathSuffix),
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
            rule => Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: Some((span.start_pos().line_col().0, span.start_pos().line_col().1)),
            }),
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
                            .ok_or(ParseError::Structure {
                                kind: StructureErrorKind::MissingToken {
                                    expected: "path component",
                                    context: "path_with_component",
                                },
                                location: None,
                            })
                    }
                    rule => Err(ParseError::Structure {
                        kind: StructureErrorKind::UnexpectedRule { rule },
                        location: None,
                    }),
                }
            }
            Rule::contents => Ok(StringSelectorType::Contents),
            Rule::r#type => Ok(StringSelectorType::Type),
            rule => Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: None,
            }),
        }
    }

    fn parse_string_op_value(
        pair: Pair<'_, Rule>,
    ) -> Result<(StringOp, String, Option<Vec<String>>), ParseError> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        match rule {
            Rule::string_eq => {
                let _ = inner.next(); // skip the eq operator
                let value = Self::extract_string_value_from_pairs(inner)?;
                Ok((StringOp::Equals, value, None))
            }
            Rule::string_ne => {
                let _ = inner.next(); // skip the ne operator
                let value = Self::extract_string_value_from_pairs(inner)?;
                Ok((StringOp::NotEquals, value, None))
            }
            Rule::string_contains => {
                let _ = inner.next(); // skip the contains operator
                let value = Self::extract_string_value_from_pairs(inner)?;
                Ok((StringOp::Contains, value, None))
            }
            Rule::string_regex => {
                let _ = inner.next(); // skip the regex operator
                let value = Self::extract_string_value_from_pairs(inner)?;
                Ok((StringOp::Regex, value, None))
            }
            Rule::string_in => {
                let _ = inner.next(); // skip the in operator
                                      // Parse set literal
                let mut items = Vec::new();
                if let Some(set_literal) = inner.next() {
                    for set_items in set_literal.into_inner() {
                        for item in set_items.into_inner() {
                            let value = if item.as_rule() == Rule::set_item {
                                let inner_item = item.into_inner().next().ok_or({
                                    ParseError::Structure {
                                        kind: StructureErrorKind::MissingToken {
                                            expected: "set item value",
                                            context: "set item",
                                        },
                                        location: None,
                                    }
                                })?;
                                match inner_item.as_rule() {
                                    Rule::quoted_string => Self::extract_string_value(inner_item)?,
                                    Rule::set_token => inner_item.as_str().to_string(),
                                    _ => continue,
                                }
                            } else {
                                continue;
                            };
                            items.push(value);
                        }
                    }
                }
                Ok((StringOp::Equals, String::new(), Some(items)))
            }
            rule => Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: None,
            }),
        }
    }

    fn extract_string_value_from_pairs(mut pairs: Pairs<'_, Rule>) -> Result<String, ParseError> {
        let pair = pairs.next().ok_or(ParseError::Structure {
            kind: StructureErrorKind::MissingToken {
                expected: "string value",
                context: "string value extraction",
            },
            location: None,
        })?;
        Self::extract_string_value(pair)
    }

    fn extract_string_value(pair: Pair<'_, Rule>) -> Result<String, ParseError> {
        match pair.as_rule() {
            Rule::string_value => {
                let inner = pair.into_inner().next().ok_or(ParseError::Structure {
                    kind: StructureErrorKind::MissingToken {
                        expected: "string content",
                        context: "string_value",
                    },
                    location: None,
                })?;
                Self::extract_string_value(inner)
            }
            Rule::quoted_string => Self::extract_quoted_string(pair),
            Rule::bare_string => Ok(pair.as_str().to_string()),
            _ => Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule {
                    rule: pair.as_rule(),
                },
                location: None,
            }),
        }
    }

    fn extract_quoted_string(pair: Pair<'_, Rule>) -> Result<String, ParseError> {
        let quoted = pair.into_inner().next().ok_or(ParseError::Structure {
            kind: StructureErrorKind::MissingToken {
                expected: "quoted content",
                context: "quoted string",
            },
            location: None,
        })?;
        let inner_str = quoted.into_inner().next().ok_or(ParseError::Structure {
            kind: StructureErrorKind::MissingToken {
                expected: "string content",
                context: "quoted string",
            },
            location: None,
        })?;
        Ok(inner_str.as_str().to_string())
    }

    fn parse_numeric_predicate(pair: Pair<'_, Rule>) -> Result<Self, ParseError> {
        let mut inner = pair.into_inner();

        // Parse selector (size or depth)
        let selector_pair = inner.next().ok_or(ParseError::Structure {
            kind: StructureErrorKind::MissingToken {
                expected: "numeric selector",
                context: "numeric_predicate",
            },
            location: None,
        })?;
        
        let selector = match selector_pair.as_rule() {
            Rule::size => NumericSelectorType::Size,
            Rule::depth => NumericSelectorType::Depth,
            rule => return Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: None,
            }),
        };

        // Parse operator and value
        let op_value_pair = inner.next().ok_or(ParseError::Structure {
            kind: StructureErrorKind::MissingToken {
                expected: "operator and value",
                context: "numeric_predicate",
            },
            location: None,
        })?;

        let (op, value) = Self::parse_numeric_op_value(op_value_pair)?;
        Ok(TypedPredicate::Numeric { selector, op, value })
    }

    fn parse_numeric_op_value(pair: Pair<'_, Rule>) -> Result<(NumericOp, u64), ParseError> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        let (op, value_pair) = match rule {
            Rule::numeric_eq => {
                let _ = inner.next(); // skip operator
                (NumericOp::Equals, inner.next())
            }
            Rule::numeric_ne => {
                let _ = inner.next();
                (NumericOp::NotEquals, inner.next())
            }
            Rule::numeric_comparison => {
                let op_pair = inner.next().ok_or(ParseError::Structure {
                    kind: StructureErrorKind::MissingToken {
                        expected: "comparison operator",
                        context: "numeric_comparison",
                    },
                    location: None,
                })?;
                let op = match op_pair.as_rule() {
                    Rule::gt => NumericOp::Greater,
                    Rule::gteq => NumericOp::GreaterOrEqual,
                    Rule::lt => NumericOp::Less,
                    Rule::lteq => NumericOp::LessOrEqual,
                    rule => {
                        return Err(ParseError::Structure {
                            kind: StructureErrorKind::UnexpectedRule { rule },
                            location: None,
                        })
                    }
                };
                (op, inner.next())
            }
            rule => {
                return Err(ParseError::Structure {
                    kind: StructureErrorKind::UnexpectedRule { rule },
                    location: None,
                })
            }
        };

        let value_pair = value_pair.ok_or(ParseError::Structure {
            kind: StructureErrorKind::MissingToken {
                expected: "numeric value",
                context: "numeric operator",
            },
            location: None,
        })?;

        let value = Self::parse_numeric_value(value_pair)?;
        Ok((op, value))
    }

    fn parse_numeric_value(pair: Pair<'_, Rule>) -> Result<u64, ParseError> {
        match pair.as_rule() {
            Rule::numeric_value => {
                let inner = pair.into_inner().next().ok_or(ParseError::Structure {
                    kind: StructureErrorKind::MissingToken {
                        expected: "numeric content",
                        context: "numeric_value",
                    },
                    location: None,
                })?;
                Self::parse_numeric_value(inner)
            }
            Rule::size_value => crate::parser::parse_size_value_as_bytes(pair),
            Rule::bare_number => pair.as_str().parse().map_err(|_| ParseError::Structure {
                kind: StructureErrorKind::InvalidToken {
                    expected: "numeric value",
                    found: pair.as_str().to_string(),
                },
                location: None,
            }),
            rule => Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: None,
            }),
        }
    }

    fn parse_temporal_predicate(pair: Pair<'_, Rule>) -> Result<Self, ParseError> {
        let mut inner = pair.into_inner();

        // Parse selector
        let selector_pair = inner.next().ok_or(ParseError::Structure {
            kind: StructureErrorKind::MissingToken {
                expected: "temporal selector",
                context: "temporal_predicate",
            },
            location: None,
        })?;

        let selector = match selector_pair.as_rule() {
            Rule::temporal_selector => {
                // Unwrap the temporal_selector wrapper
                let inner = selector_pair
                    .into_inner()
                    .next()
                    .ok_or(ParseError::Structure {
                        kind: StructureErrorKind::MissingToken {
                            expected: "temporal selector type",
                            context: "temporal_selector",
                        },
                        location: None,
                    })?;
                match inner.as_rule() {
                    Rule::modified => TemporalSelectorType::Modified,
                    Rule::created => TemporalSelectorType::Created,
                    Rule::accessed => TemporalSelectorType::Accessed,
                    rule => {
                        return Err(ParseError::Structure {
                            kind: StructureErrorKind::UnexpectedRule { rule },
                            location: None,
                        })
                    }
                }
            }
            Rule::modified => TemporalSelectorType::Modified,
            Rule::created => TemporalSelectorType::Created,
            Rule::accessed => TemporalSelectorType::Accessed,
            rule => {
                return Err(ParseError::Structure {
                    kind: StructureErrorKind::UnexpectedRule { rule },
                    location: None,
                })
            }
        };

        // Parse operator and value
        let op_value_pair = inner.next().ok_or(ParseError::Structure {
            kind: StructureErrorKind::MissingToken {
                expected: "operator and value",
                context: "temporal_predicate",
            },
            location: None,
        })?;

        let (op, value) = Self::parse_temporal_op_value(op_value_pair)?;
        Ok(TypedPredicate::Temporal {
            selector,
            op,
            value,
        })
    }

    fn parse_temporal_op_value(pair: Pair<'_, Rule>) -> Result<(TemporalOp, String), ParseError> {
        let rule = pair.as_rule();
        let mut inner = pair.into_inner();

        let (op, value_pair) = match rule {
            Rule::temporal_eq => {
                let _ = inner.next();
                (TemporalOp::Equals, inner.next())
            }
            Rule::temporal_ne => {
                let _ = inner.next();
                (TemporalOp::NotEquals, inner.next())
            }
            Rule::temporal_comparison => {
                let op_pair = inner.next().ok_or(ParseError::Structure {
                    kind: StructureErrorKind::MissingToken {
                        expected: "comparison operator",
                        context: "temporal_comparison",
                    },
                    location: None,
                })?;
                let op = match op_pair.as_rule() {
                    Rule::gt | Rule::gteq => TemporalOp::After,
                    Rule::lt | Rule::lteq => TemporalOp::Before,
                    rule => {
                        return Err(ParseError::Structure {
                            kind: StructureErrorKind::UnexpectedRule { rule },
                            location: None,
                        })
                    }
                };
                (op, inner.next())
            }
            rule => {
                return Err(ParseError::Structure {
                    kind: StructureErrorKind::UnexpectedRule { rule },
                    location: None,
                })
            }
        };

        let value_pair = value_pair.ok_or(ParseError::Structure {
            kind: StructureErrorKind::MissingToken {
                expected: "temporal value",
                context: "temporal operator",
            },
            location: None,
        })?;

        let value = Self::parse_temporal_value(value_pair)?;
        Ok((op, value))
    }

    fn parse_temporal_value(pair: Pair<'_, Rule>) -> Result<String, ParseError> {
        match pair.as_rule() {
            Rule::temporal_value => {
                let inner = pair.into_inner().next().ok_or(ParseError::Structure {
                    kind: StructureErrorKind::MissingToken {
                        expected: "temporal content",
                        context: "temporal_value",
                    },
                    location: None,
                })?;
                Self::parse_temporal_value(inner)
            }
            Rule::time_value => Ok(pair.as_str().to_string()),
            Rule::quoted_string => Self::extract_string_value(pair),
            Rule::time_keyword => Ok(pair.as_str().to_string()),
            rule => Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: None,
            }),
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
                        let pattern = match &string_matcher {
                            StringMatcher::Regex(r) => r.as_str().to_string(),
                            StringMatcher::Equals(s) => format!("^{}$", regex::escape(s)),
                            StringMatcher::Contains(s) => regex::escape(s),
                            _ => {
                                return Err(ParseError::Structure {
                                    kind: StructureErrorKind::InvalidToken {
                                        expected: "regex, equals, or contains for contents",
                                        found: format!("{:?}", string_matcher),
                                    },
                                    location: None,
                                })
                            }
                        };
                        let content_pred = StreamingCompiledContentPredicate::new(pattern)
                            .map_err(|_| ParseError::Structure {
                                kind: StructureErrorKind::InvalidToken {
                                    expected: "valid regex pattern for DFA",
                                    found: value,
                                },
                                location: None,
                            })?;
                        DomainPredicate::contents(content_pred)
                    }
                };
                Ok(Expr::Predicate(predicate))
            }
            TypedPredicate::Set { selector, items } => {
                let string_matcher = StringMatcher::In(items);
                let predicate = match selector {
                    StringSelectorType::PathFull => {
                        DomainPredicate::name(NamePredicate::FullPath(string_matcher))
                    }
                    StringSelectorType::PathParent => {
                        DomainPredicate::name(NamePredicate::DirPath(string_matcher))
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
                        return Err(ParseError::Structure {
                            kind: StructureErrorKind::InvalidToken {
                                expected: "path or type selector for 'in' operator",
                                found: format!("{:?}", selector),
                            },
                            location: None,
                        })
                    }
                };
                Ok(Expr::Predicate(predicate))
            }
            TypedPredicate::Numeric { selector, op, value } => {
                let number_matcher = match op {
                    NumericOp::Equals => NumberMatcher::Equals(value),
                    NumericOp::NotEquals => NumberMatcher::NotEquals(value),
                    NumericOp::Greater => {
                        NumberMatcher::In(Bound::Left(RangeFrom { start: value + 1 }))
                    }
                    NumericOp::GreaterOrEqual => {
                        NumberMatcher::In(Bound::Left(RangeFrom { start: value }))
                    }
                    NumericOp::Less => {
                        NumberMatcher::In(Bound::Right(RangeTo { end: value }))
                    }
                    NumericOp::LessOrEqual => {
                        NumberMatcher::In(Bound::Right(RangeTo { end: value + 1 }))
                    }
                };
                let predicate = match selector {
                    NumericSelectorType::Size => DomainPredicate::meta(MetadataPredicate::Filesize(number_matcher)),
                    NumericSelectorType::Depth => DomainPredicate::meta(MetadataPredicate::Depth(number_matcher)),
                };
                Ok(Expr::Predicate(predicate))
            }
            TypedPredicate::Temporal {
                selector,
                op,
                value,
            } => {
                let parsed_time = crate::predicate::parse_time_value(&value).map_err(|_| {
                    ParseError::Structure {
                        kind: StructureErrorKind::InvalidToken {
                            expected: "valid time value",
                            found: value,
                        },
                        location: None,
                    }
                })?;

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
