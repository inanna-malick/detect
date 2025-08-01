use pest::{
    iterators::Pairs,
    pratt_parser::{Assoc::*, Op, PrattParser},
    Parser, Span,
};

use crate::{
    expr::Expr,
    parse_error::{ParseError, StructureErrorKind},
    predicate::{
        self, Bound, NumberMatcher, Predicate, StreamingCompiledContentPredicate, StringMatcher,
        TimeMatcher,
    },
    MetadataPredicate, NamePredicate,
};
use regex::Regex;
use std::ops::{RangeFrom, RangeTo};

pub mod pratt_parser {
    use pest_derive::Parser;

    #[derive(Parser)]
    #[grammar = "expr/expr.pest"]
    pub struct Parser;
}

use pratt_parser::Rule;

fn get_location(span: &Span) -> Option<(usize, usize)> {
    let (line, col) = span.start_pos().line_col();
    Some((line, col))
}

fn parse_size_value_as_bytes(pair: pest::iterators::Pair<Rule>) -> Result<u64, ParseError> {
    let span = pair.as_span();
    let text = pair.as_str();

    // The grammar ensures this has a number followed by a unit
    // Find where the unit starts (first non-digit, non-dot character after initial digits)
    let mut unit_start = 0;
    let chars: Vec<char> = text.chars().collect();

    // Skip the number part (digits and optional decimal point)
    while unit_start < chars.len()
        && (chars[unit_start].is_ascii_digit() || chars[unit_start] == '.')
    {
        unit_start += 1;
    }

    if unit_start == 0 || unit_start == chars.len() {
        return Err(ParseError::Structure {
            kind: StructureErrorKind::InvalidToken {
                expected: "size value with unit",
                found: text.to_string(),
            },
            location: get_location(&span),
        });
    }

    let number_part = &text[..unit_start];
    let unit_part = &text[unit_start..];

    let value = number_part
        .parse::<f64>()
        .map_err(|_| ParseError::Structure {
            kind: StructureErrorKind::InvalidToken {
                expected: "numeric value",
                found: number_part.to_string(),
            },
            location: get_location(&span),
        })?;

    let multiplier = match unit_part.to_lowercase().as_str() {
        "b" => 1.0,
        "k" | "kb" => 1024.0,
        "m" | "mb" => 1024.0 * 1024.0,
        "g" | "gb" => 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => {
            return Err(ParseError::Structure {
                kind: StructureErrorKind::InvalidToken {
                    expected: "size unit (B, KB, MB, GB, TB)",
                    found: unit_part.to_string(),
                },
                location: get_location(&span),
            })
        }
    };

    Ok((value * multiplier) as u64)
}

fn extract_string_value(pair: pest::iterators::Pair<Rule>) -> Result<String, ParseError> {
    let span = pair.as_span();
    match pair.as_rule() {
        Rule::string_value => {
            // Unwrap the string_value to get the inner quoted_string or bare_string
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| ParseError::Structure {
                    kind: StructureErrorKind::MissingToken {
                        expected: "string content",
                        context: "string_value",
                    },
                    location: get_location(&span),
                })?;
            extract_string_value(inner)
        }
        Rule::quoted_string => {
            let quoted = pair
                .into_inner()
                .next()
                .ok_or_else(|| ParseError::Structure {
                    kind: StructureErrorKind::MissingToken {
                        expected: "quoted content",
                        context: "quoted string",
                    },
                    location: get_location(&span),
                })?;
            let inner_str = quoted
                .into_inner()
                .next()
                .ok_or_else(|| ParseError::Structure {
                    kind: StructureErrorKind::MissingToken {
                        expected: "string content",
                        context: "quoted string",
                    },
                    location: get_location(&span),
                })?;
            Ok(inner_str.as_str().to_string())
        }
        Rule::bare_string => Ok(pair.as_str().to_string()),
        rule => Err(ParseError::Structure {
            kind: StructureErrorKind::UnexpectedRule { rule },
            location: get_location(&span),
        }),
    }
}

fn parse_string_predicate(
    pair: pest::iterators::Pair<Rule>,
) -> Result<
    Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
    ParseError,
> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    // Parse string selector
    let selector_pair = inner.next().ok_or_else(|| ParseError::Structure {
        kind: StructureErrorKind::MissingToken {
            expected: "string selector",
            context: "string_predicate",
        },
        location: get_location(&span),
    })?;

    // For path_selector, we might have a nested structure or a direct selector
    let selector = if selector_pair.as_rule() == Rule::path_selector {
        // Path selector might be just "path" or "path.component"
        let mut inner = selector_pair.clone().into_inner();
        if let Some(first) = inner.next() {
            first
        } else {
            // No inner content means it's the atomic @{ } rule result
            selector_pair
        }
    } else {
        // For other selectors (contents, type), get the inner content
        selector_pair
            .into_inner()
            .next()
            .ok_or_else(|| ParseError::Structure {
                kind: StructureErrorKind::MissingToken {
                    expected: "selector name",
                    context: "string_selector",
                },
                location: get_location(&span),
            })?
    };

    // Parse operator and value
    let op_value_pair = inner.next().ok_or_else(|| ParseError::Structure {
        kind: StructureErrorKind::MissingToken {
            expected: "operator and value",
            context: "string_predicate",
        },
        location: get_location(&span),
    })?;

    let op_value_rule = op_value_pair.as_rule();
    let mut op_value_inner = op_value_pair.into_inner();
    let _op_pair = op_value_inner.next().ok_or_else(|| ParseError::Structure {
        kind: StructureErrorKind::MissingToken {
            expected: "operator",
            context: "string operator",
        },
        location: get_location(&span),
    })?;

    let value_pair = op_value_inner.next().ok_or_else(|| ParseError::Structure {
        kind: StructureErrorKind::MissingToken {
            expected: "value",
            context: "string operator",
        },
        location: get_location(&span),
    })?;

    // Create the appropriate string matcher based on operator
    let string_matcher = match op_value_rule {
        Rule::string_eq => {
            let value = extract_string_value(value_pair)?;
            StringMatcher::Equals(value)
        }
        Rule::string_ne => {
            let value = extract_string_value(value_pair)?;
            StringMatcher::NotEquals(value)
        }
        Rule::string_contains => {
            let value = extract_string_value(value_pair)?;
            StringMatcher::Contains(value)
        }
        Rule::string_regex => {
            let value = extract_string_value(value_pair)?;
            // Special case for '*' which users commonly expect to work
            let pattern = if value == "*" { ".*" } else { &value };
            StringMatcher::Regex(Regex::new(pattern).map_err(|_e| ParseError::Structure {
                kind: StructureErrorKind::InvalidToken {
                    expected: "valid regex pattern",
                    found: value.clone(),
                },
                location: get_location(&span),
            })?)
        }
        Rule::string_in => {
            // Parse set literal
            let mut items = Vec::new();
            if let Some(set_items) = value_pair.into_inner().next() {
                for item in set_items.into_inner() {
                    if item.as_rule() == Rule::set_item {
                        let inner_item =
                            item.into_inner()
                                .next()
                                .ok_or_else(|| ParseError::Structure {
                                    kind: StructureErrorKind::MissingToken {
                                        expected: "set item value",
                                        context: "set item",
                                    },
                                    location: get_location(&span),
                                })?;
                        let value = match inner_item.as_rule() {
                            Rule::quoted_string => extract_string_value(inner_item)?,
                            Rule::set_token => inner_item.as_str().to_string(),
                            rule => {
                                return Err(ParseError::Structure {
                                    kind: StructureErrorKind::UnexpectedRule { rule },
                                    location: get_location(&span),
                                })
                            }
                        };
                        items.push(value);
                    }
                }
            }
            StringMatcher::In(items)
        }
        rule => {
            return Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: get_location(&span),
            })
        }
    };

    // Create the appropriate predicate based on selector
    match selector.as_rule() {
        Rule::path_selector => {
            // Handle hierarchical path selectors
            let mut path_parts = selector.into_inner();
            let first = path_parts.next().ok_or_else(|| ParseError::Structure {
                kind: StructureErrorKind::MissingToken {
                    expected: "path selector",
                    context: "path_selector",
                },
                location: get_location(&span),
            })?;

            match first.as_rule() {
                Rule::path_alias => {
                    // Simple 'path' selector - maps to full path (alias for path.full)
                    Ok(Expr::Predicate(Predicate::name(NamePredicate::FullPath(
                        string_matcher,
                    ))))
                }
                Rule::path_with_component => {
                    // Handle path.full, path.parent, path.name, etc.
                    let inner = first.into_inner();
                    // Find the component token (should be last)
                    let component = inner
                        .into_iter()
                        .find(|t| {
                            matches!(
                                t.as_rule(),
                                Rule::path_full
                                    | Rule::path_parent
                                    | Rule::path_name
                                    | Rule::path_stem
                                    | Rule::path_suffix
                            )
                        })
                        .ok_or_else(|| ParseError::Structure {
                            kind: StructureErrorKind::MissingToken {
                                expected: "path component",
                                context: "path_with_component",
                            },
                            location: get_location(&span),
                        })?;

                    match component.as_rule() {
                        Rule::path_full => Ok(Expr::Predicate(Predicate::name(
                            NamePredicate::FullPath(string_matcher),
                        ))),
                        Rule::path_parent => Ok(Expr::Predicate(Predicate::name(
                            NamePredicate::DirPath(string_matcher),
                        ))),
                        Rule::path_name => Ok(Expr::Predicate(Predicate::name(
                            NamePredicate::FileName(string_matcher),
                        ))),
                        Rule::path_stem => Ok(Expr::Predicate(Predicate::name(
                            NamePredicate::BaseName(string_matcher),
                        ))),
                        Rule::path_suffix => Ok(Expr::Predicate(Predicate::name(
                            NamePredicate::Extension(string_matcher),
                        ))),
                        rule => Err(ParseError::Structure {
                            kind: StructureErrorKind::UnexpectedRule { rule },
                            location: get_location(&span),
                        }),
                    }
                }
                rule => Err(ParseError::Structure {
                    kind: StructureErrorKind::UnexpectedRule { rule },
                    location: get_location(&span),
                }),
            }
        }
        Rule::r#type => Ok(Expr::Predicate(Predicate::meta(MetadataPredicate::Type(
            string_matcher,
        )))),
        Rule::contents => {
            // For contents, we need to create a StreamingCompiledContentPredicate
            match string_matcher {
                StringMatcher::Regex(regex) => {
                    let content_pred =
                        StreamingCompiledContentPredicate::new(regex.as_str().to_string())
                            .map_err(|_| ParseError::Structure {
                                kind: StructureErrorKind::InvalidToken {
                                    expected: "valid regex pattern for DFA",
                                    found: regex.as_str().to_string(),
                                },
                                location: get_location(&span),
                            })?;
                    Ok(Expr::Predicate(Predicate::contents(content_pred)))
                }
                StringMatcher::Equals(s) => {
                    let regex = format!("^{}$", regex::escape(&s));
                    let content_pred =
                        StreamingCompiledContentPredicate::new(regex).map_err(|_| {
                            ParseError::Structure {
                                kind: StructureErrorKind::InvalidToken {
                                    expected: "valid regex pattern for DFA",
                                    found: s.clone(),
                                },
                                location: get_location(&span),
                            }
                        })?;
                    Ok(Expr::Predicate(Predicate::contents(content_pred)))
                }
                StringMatcher::Contains(s) => {
                    let regex = regex::escape(&s);
                    let content_pred =
                        StreamingCompiledContentPredicate::new(regex).map_err(|_| {
                            ParseError::Structure {
                                kind: StructureErrorKind::InvalidToken {
                                    expected: "valid regex pattern for DFA",
                                    found: s.clone(),
                                },
                                location: get_location(&span),
                            }
                        })?;
                    Ok(Expr::Predicate(Predicate::contents(content_pred)))
                }
                _ => Err(ParseError::Structure {
                    kind: StructureErrorKind::InvalidToken {
                        expected: "regex, equals, or contains operator for contents",
                        found: format!("{:?}", string_matcher),
                    },
                    location: get_location(&span),
                }),
            }
        }
        rule => Err(ParseError::Structure {
            kind: StructureErrorKind::UnexpectedRule { rule },
            location: get_location(&span),
        }),
    }
}

fn parse_numeric_predicate(
    pair: pest::iterators::Pair<Rule>,
) -> Result<
    Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
    ParseError,
> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    // Parse size selector (only numeric selector for now)
    let _size_selector = inner.next().ok_or_else(|| ParseError::Structure {
        kind: StructureErrorKind::MissingToken {
            expected: "size selector",
            context: "numeric_predicate",
        },
        location: get_location(&span),
    })?;

    // Parse operator and value
    let op_value_pair = inner.next().ok_or_else(|| ParseError::Structure {
        kind: StructureErrorKind::MissingToken {
            expected: "operator and value",
            context: "numeric_predicate",
        },
        location: get_location(&span),
    })?;

    let op_value_rule = op_value_pair.as_rule();
    let mut op_value_inner = op_value_pair.into_inner();
    let _op_pair = op_value_inner.next().ok_or_else(|| ParseError::Structure {
        kind: StructureErrorKind::MissingToken {
            expected: "operator",
            context: "numeric operator",
        },
        location: get_location(&span),
    })?;

    let value_pair = op_value_inner.next().ok_or_else(|| ParseError::Structure {
        kind: StructureErrorKind::MissingToken {
            expected: "value",
            context: "numeric operator",
        },
        location: get_location(&span),
    })?;

    // Parse the numeric value directly as bytes
    let bytes = match value_pair.as_rule() {
        Rule::numeric_value => {
            // Unwrap the numeric_value to get the inner size_value or bare_number
            let inner = value_pair
                .into_inner()
                .next()
                .ok_or_else(|| ParseError::Structure {
                    kind: StructureErrorKind::MissingToken {
                        expected: "numeric content",
                        context: "numeric_value",
                    },
                    location: get_location(&span),
                })?;
            match inner.as_rule() {
                Rule::size_value => parse_size_value_as_bytes(inner)?,
                Rule::bare_number => {
                    let num_str = inner.as_str();
                    num_str.parse::<u64>().map_err(|_| ParseError::Structure {
                        kind: StructureErrorKind::InvalidToken {
                            expected: "numeric value",
                            found: num_str.to_string(),
                        },
                        location: get_location(&span),
                    })?
                }
                rule => {
                    return Err(ParseError::Structure {
                        kind: StructureErrorKind::UnexpectedRule { rule },
                        location: get_location(&span),
                    })
                }
            }
        }
        Rule::size_value => parse_size_value_as_bytes(value_pair)?,
        Rule::bare_number => {
            let num_str = value_pair.as_str();
            num_str.parse::<u64>().map_err(|_| ParseError::Structure {
                kind: StructureErrorKind::InvalidToken {
                    expected: "numeric value",
                    found: num_str.to_string(),
                },
                location: get_location(&span),
            })?
        }
        rule => {
            return Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: get_location(&span),
            })
        }
    };

    // Create the numeric matcher based on operator
    let numeric_matcher = match op_value_rule {
        Rule::numeric_eq => NumberMatcher::Equals(bytes),
        Rule::numeric_ne => NumberMatcher::NotEquals(bytes),
        Rule::numeric_comparison => {
            let op_rule = _op_pair.as_rule();
            match op_rule {
                Rule::gt => NumberMatcher::In(Bound::Left(RangeFrom { start: bytes })),
                Rule::gteq => NumberMatcher::In(Bound::Left(RangeFrom {
                    start: bytes.saturating_sub(1),
                })),
                Rule::lt => NumberMatcher::In(Bound::Right(RangeTo {
                    end: bytes.saturating_add(1),
                })),
                Rule::lteq => NumberMatcher::In(Bound::Right(RangeTo { end: bytes })),
                rule => {
                    return Err(ParseError::Structure {
                        kind: StructureErrorKind::UnexpectedRule { rule },
                        location: get_location(&span),
                    })
                }
            }
        }
        rule => {
            return Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: get_location(&span),
            })
        }
    };

    Ok(Expr::Predicate(Predicate::meta(
        MetadataPredicate::Filesize(numeric_matcher),
    )))
}

fn parse_temporal_predicate(
    pair: pest::iterators::Pair<Rule>,
) -> Result<
    Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
    ParseError,
> {
    let span = pair.as_span();
    let mut inner = pair.into_inner();

    // Parse temporal selector
    let selector_pair = inner.next().ok_or_else(|| ParseError::Structure {
        kind: StructureErrorKind::MissingToken {
            expected: "temporal selector",
            context: "temporal_predicate",
        },
        location: get_location(&span),
    })?;

    let selector = selector_pair
        .into_inner()
        .next()
        .ok_or_else(|| ParseError::Structure {
            kind: StructureErrorKind::MissingToken {
                expected: "selector name",
                context: "temporal_selector",
            },
            location: get_location(&span),
        })?;

    // Parse operator and value
    let op_value_pair = inner.next().ok_or_else(|| ParseError::Structure {
        kind: StructureErrorKind::MissingToken {
            expected: "operator and value",
            context: "temporal_predicate",
        },
        location: get_location(&span),
    })?;

    let op_value_rule = op_value_pair.as_rule();
    let mut op_value_inner = op_value_pair.into_inner();
    let op_pair = op_value_inner.next().ok_or_else(|| ParseError::Structure {
        kind: StructureErrorKind::MissingToken {
            expected: "operator",
            context: "temporal operator",
        },
        location: get_location(&span),
    })?;

    let value_pair = op_value_inner.next().ok_or_else(|| ParseError::Structure {
        kind: StructureErrorKind::MissingToken {
            expected: "value",
            context: "temporal operator",
        },
        location: get_location(&span),
    })?;

    // Parse the temporal value
    let time_str = match value_pair.as_rule() {
        Rule::temporal_value => {
            // Unwrap the temporal_value to get the inner value
            let inner = value_pair
                .into_inner()
                .next()
                .ok_or_else(|| ParseError::Structure {
                    kind: StructureErrorKind::MissingToken {
                        expected: "temporal content",
                        context: "temporal_value",
                    },
                    location: get_location(&span),
                })?;
            match inner.as_rule() {
                Rule::time_value => inner.as_str().to_string(),
                Rule::quoted_string => extract_string_value(inner)?,
                Rule::time_keyword => inner.as_str().to_string(),
                rule => {
                    return Err(ParseError::Structure {
                        kind: StructureErrorKind::UnexpectedRule { rule },
                        location: get_location(&span),
                    })
                }
            }
        }
        Rule::time_value => value_pair.as_str().to_string(),
        Rule::quoted_string => extract_string_value(value_pair)?,
        Rule::time_keyword => value_pair.as_str().to_string(),
        rule => {
            return Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: get_location(&span),
            })
        }
    };

    // Parse the time value using the existing parse_time_value function
    let parsed_time =
        predicate::parse_time_value(&time_str).map_err(|_| ParseError::Structure {
            kind: StructureErrorKind::InvalidToken {
                expected: "valid time value",
                found: time_str.clone(),
            },
            location: get_location(&span),
        })?;

    // Create the time matcher based on operator
    let time_matcher = match op_value_rule {
        Rule::temporal_eq => TimeMatcher::Equals(parsed_time),
        Rule::temporal_ne => TimeMatcher::NotEquals(parsed_time),
        Rule::temporal_comparison => {
            let op_rule = op_pair.as_rule();
            match op_rule {
                Rule::gt | Rule::gteq => TimeMatcher::After(parsed_time),
                Rule::lt | Rule::lteq => TimeMatcher::Before(parsed_time),
                rule => {
                    return Err(ParseError::Structure {
                        kind: StructureErrorKind::UnexpectedRule { rule },
                        location: get_location(&span),
                    })
                }
            }
        }
        rule => {
            return Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: get_location(&span),
            })
        }
    };

    // Create the appropriate predicate based on selector
    match selector.as_rule() {
        Rule::modified => Ok(Expr::Predicate(Predicate::meta(
            MetadataPredicate::Modified(time_matcher),
        ))),
        Rule::created => Ok(Expr::Predicate(Predicate::meta(
            MetadataPredicate::Created(time_matcher),
        ))),
        Rule::accessed => Ok(Expr::Predicate(Predicate::meta(
            MetadataPredicate::Accessed(time_matcher),
        ))),
        rule => Err(ParseError::Structure {
            kind: StructureErrorKind::UnexpectedRule { rule },
            location: get_location(&span),
        }),
    }
}

fn parse_typed_predicates(
    pairs: Pairs<Rule>,
    pratt: &PrattParser<Rule>,
) -> Result<
    Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
    ParseError,
> {
    pratt
        .map_primary(|primary| match primary.as_rule() {
            Rule::typed_predicate => {
                let span = primary.as_span();
                let inner = primary
                    .into_inner()
                    .next()
                    .ok_or_else(|| ParseError::Structure {
                        kind: StructureErrorKind::MissingToken {
                            expected: "predicate type",
                            context: "typed_predicate",
                        },
                        location: get_location(&span),
                    })?;

                match inner.as_rule() {
                    Rule::string_predicate => parse_string_predicate(inner),
                    Rule::numeric_predicate => parse_numeric_predicate(inner),
                    Rule::temporal_predicate => parse_temporal_predicate(inner),
                    rule => Err(ParseError::Structure {
                        kind: StructureErrorKind::UnexpectedRule { rule },
                        location: get_location(&span),
                    }),
                }
            }
            Rule::expr => parse_typed_predicates(primary.into_inner(), pratt),
            rule => Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: None,
            }),
        })
        .map_prefix(|op, rhs| match op.as_rule() {
            Rule::neg => Ok(Expr::Not(Box::new(rhs?))),
            rule => Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: None,
            }),
        })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            Rule::and => Ok(Expr::And(Box::new(lhs?), Box::new(rhs?))),
            Rule::or => Ok(Expr::Or(Box::new(lhs?), Box::new(rhs?))),
            rule => Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: None,
            }),
        })
        .parse(pairs)
}

pub fn parse_expr(
    input: &str,
) -> Result<
    Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
    ParseError,
> {
    let pratt = PrattParser::new()
        .op(Op::infix(Rule::or, Left))
        .op(Op::infix(Rule::and, Left))
        .op(Op::prefix(Rule::neg));

    let mut parse_tree = pratt_parser::Parser::parse(Rule::program, input)
        .map_err(|e| ParseError::Syntax(Box::new(e)))?;

    let program = parse_tree.next().ok_or(ParseError::Structure {
        kind: StructureErrorKind::MissingToken {
            expected: "program",
            context: "root",
        },
        location: None,
    })?;

    let expr_pair = program.into_inner().next().ok_or(ParseError::Structure {
        kind: StructureErrorKind::MissingToken {
            expected: "expression",
            context: "program",
        },
        location: None,
    })?;

    let pairs = expr_pair.into_inner();

    // Use the new typed predicate parser
    parse_typed_predicates(pairs, &pratt)
}
