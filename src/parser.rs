use pest::{
    iterators::Pairs,
    pratt_parser::{Assoc::*, Op, PrattParser},
    Parser, Span,
};

use crate::{
    expr::Expr,
    parse_error::{ParseError, StructureErrorKind},
    predicate::{self, Predicate, RawPredicate, RhsValue, StreamingCompiledContentPredicate},
    MetadataPredicate, NamePredicate,
};

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

fn parse_size_value(pair: pest::iterators::Pair<Rule>) -> Result<RhsValue, ParseError> {
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

    Ok(RhsValue::Size((value * multiplier) as u64))
}

fn parse(pairs: Pairs<Rule>, pratt: &PrattParser<Rule>) -> Result<Expr<RawPredicate>, ParseError> {
    pratt
        .map_primary(|primary| match primary.as_rule() {
            Rule::predicate => {
                let span = primary.as_span();
                let mut inner = primary.into_inner();

                // Parse selector
                let selector_pair = inner.next().ok_or_else(|| ParseError::Structure {
                    kind: StructureErrorKind::MissingToken {
                        expected: "selector",
                        context: "predicate",
                    },
                    location: get_location(&span),
                })?;

                let selector_inner =
                    selector_pair
                        .into_inner()
                        .next()
                        .ok_or_else(|| ParseError::Structure {
                            kind: StructureErrorKind::MissingToken {
                                expected: "selector name",
                                context: "selector",
                            },
                            location: get_location(&span),
                        })?;

                let lhs = match selector_inner.as_rule() {
                    Rule::name => predicate::Selector::FileName,
                    Rule::path => predicate::Selector::FilePath,
                    Rule::ext => predicate::Selector::Extension,
                    Rule::size => predicate::Selector::Size,
                    Rule::r#type => predicate::Selector::EntityType,
                    Rule::contents => predicate::Selector::Contents,
                    Rule::modified => predicate::Selector::Modified,
                    Rule::created => predicate::Selector::Created,
                    Rule::accessed => predicate::Selector::Accessed,
                    rule => {
                        return Err(ParseError::Structure {
                            kind: StructureErrorKind::InvalidSelector {
                                found: format!("{:?}", rule),
                            },
                            location: get_location(&span),
                        })
                    }
                };

                // Parse operator
                let op_pair = inner.next().ok_or_else(|| ParseError::Structure {
                    kind: StructureErrorKind::MissingToken {
                        expected: "operator",
                        context: "predicate",
                    },
                    location: get_location(&span),
                })?;

                let op = match op_pair.as_rule() {
                    Rule::eq => predicate::Op::Equality,
                    Rule::ne => predicate::Op::NotEqual,
                    Rule::like | Rule::match_ => predicate::Op::Matches,
                    Rule::gt => predicate::Op::NumericComparison(predicate::NumericalOp::Greater),
                    Rule::gteq => {
                        predicate::Op::NumericComparison(predicate::NumericalOp::GreaterOrEqual)
                    }
                    Rule::lt => predicate::Op::NumericComparison(predicate::NumericalOp::Less),
                    Rule::lteq => {
                        predicate::Op::NumericComparison(predicate::NumericalOp::LessOrEqual)
                    }
                    Rule::in_ => predicate::Op::In,
                    Rule::contains => predicate::Op::Contains,
                    rule => {
                        return Err(ParseError::Structure {
                            kind: StructureErrorKind::UnexpectedRule { rule },
                            location: get_location(&span),
                        })
                    }
                };

                // Parse RHS
                let rhs_pair = inner.next().ok_or_else(|| ParseError::Structure {
                    kind: StructureErrorKind::MissingToken {
                        expected: "value",
                        context: "predicate",
                    },
                    location: get_location(&span),
                })?;

                let rhs = match rhs_pair.as_rule() {
                    Rule::rhs => {
                        let inner_rhs =
                            rhs_pair
                                .into_inner()
                                .next()
                                .ok_or_else(|| ParseError::Structure {
                                    kind: StructureErrorKind::MissingToken {
                                        expected: "value expression",
                                        context: "rhs",
                                    },
                                    location: get_location(&span),
                                })?;

                        match inner_rhs.as_rule() {
                            Rule::quoted_string => {
                                // Extract the inner string without quotes
                                let quoted = inner_rhs.into_inner().next().ok_or_else(|| {
                                    ParseError::Structure {
                                        kind: StructureErrorKind::MissingToken {
                                            expected: "quoted content",
                                            context: "quoted string",
                                        },
                                        location: get_location(&span),
                                    }
                                })?;
                                let inner_str = quoted.into_inner().next().ok_or_else(|| {
                                    ParseError::Structure {
                                        kind: StructureErrorKind::MissingToken {
                                            expected: "string content",
                                            context: "quoted string",
                                        },
                                        location: get_location(&span),
                                    }
                                })?;
                                RhsValue::String(inner_str.as_str().to_string())
                            }
                            Rule::bare_token => {
                                let token = inner_rhs.as_str();
                                // Try to parse as number first
                                if let Ok(num) = token.parse::<u64>() {
                                    RhsValue::Number(num)
                                } else {
                                    // Otherwise treat as string
                                    RhsValue::String(token.to_string())
                                }
                            }
                            Rule::size_value => {
                                // Parse size value with unit
                                parse_size_value(inner_rhs)?
                            }
                            Rule::set_literal => {
                                // Handle set literals, including empty sets
                                let mut items = Vec::new();

                                // set_items is now optional for empty sets
                                if let Some(set_items) = inner_rhs.into_inner().next() {
                                    for item in set_items.into_inner() {
                                        if item.as_rule() == Rule::set_item {
                                            let inner_item =
                                                item.into_inner().next().ok_or_else(|| {
                                                    ParseError::Structure {
                                                        kind: StructureErrorKind::MissingToken {
                                                            expected: "set item value",
                                                            context: "set item",
                                                        },
                                                        location: get_location(&span),
                                                    }
                                                })?;
                                            let value = match inner_item.as_rule() {
                                                Rule::quoted_string => {
                                                    let quoted = inner_item
                                                        .into_inner()
                                                        .next()
                                                        .ok_or_else(|| ParseError::Structure {
                                                            kind:
                                                                StructureErrorKind::MissingToken {
                                                                    expected: "quoted content",
                                                                    context:
                                                                        "set item quoted string",
                                                                },
                                                            location: get_location(&span),
                                                        })?;
                                                    let inner_str = quoted
                                                        .into_inner()
                                                        .next()
                                                        .ok_or_else(|| ParseError::Structure {
                                                            kind:
                                                                StructureErrorKind::MissingToken {
                                                                    expected: "string content",
                                                                    context:
                                                                        "set item quoted string",
                                                                },
                                                            location: get_location(&span),
                                                        })?;
                                                    inner_str.as_str().to_string()
                                                }
                                                Rule::bare_token => inner_item.as_str().to_string(),
                                                Rule::set_token => inner_item.as_str().to_string(),
                                                rule => {
                                                    return Err(ParseError::Structure {
                                                        kind: StructureErrorKind::UnexpectedRule {
                                                            rule,
                                                        },
                                                        location: get_location(&span),
                                                    })
                                                }
                                            };
                                            items.push(value);
                                        }
                                    }
                                }
                                RhsValue::Set(items)
                            }
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

                Ok(Expr::Predicate(RawPredicate { lhs, op, rhs }))
            }
            Rule::expr => parse(primary.into_inner(), pratt),
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

    let mut parse_tree =
        pratt_parser::Parser::parse(Rule::program, input).map_err(ParseError::Syntax)?;

    let program = parse_tree.next().ok_or_else(|| ParseError::Structure {
        kind: StructureErrorKind::MissingToken {
            expected: "program",
            context: "root",
        },
        location: None,
    })?;

    let expr_pair = program
        .into_inner()
        .next()
        .ok_or_else(|| ParseError::Structure {
            kind: StructureErrorKind::MissingToken {
                expected: "expression",
                context: "program",
            },
            location: None,
        })?;

    let pairs = expr_pair.into_inner();

    let expr = parse(pairs, &pratt)?;

    expr.map_predicate_err(|r| {
        let selector = r.lhs.clone();
        let operator = r.op.clone();
        let value = r.rhs.clone();
        r.parse().map_err(|e| ParseError::Predicate {
            selector,
            operator,
            value,
            source: e,
        })
    })
}
