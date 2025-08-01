use pest::{
    iterators::Pairs,
    pratt_parser::{Assoc::*, Op, PrattParser},
    Parser, Span,
};

use crate::{
    ast::TypedPredicate,
    expr::Expr,
    parse_error::{ParseError, StructureErrorKind},
    predicate::{Predicate, StreamingCompiledContentPredicate},
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

pub fn parse_size_value_as_bytes(pair: pest::iterators::Pair<Rule>) -> Result<u64, ParseError> {
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
                // Use the new TypedPredicate from ast.rs
                let typed_pred = TypedPredicate::from_pair(primary)?;
                typed_pred.into_expr()
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
