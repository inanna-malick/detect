use pest::{
    iterators::Pair,
    pratt_parser::{Assoc::*, Op, PrattParser},
    Parser,
};
use pest_derive::Parser;

use super::{
    ast::{RawExpr, RawPredicate, RawValue},
    error::RawParseError,
};

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
pub struct RawParser;

impl RawParser {
    /// Parse an expression from input string into a Raw AST
    pub fn parse_raw_expr(input: &str) -> Result<RawExpr<'_>, RawParseError> {
        let mut pairs = Self::parse(Rule::program, input)
            .map_err(|e| RawParseError::from_pest(Box::new(e), input.to_string()))?;

        let program_pair = pairs
            .next()
            .ok_or_else(|| RawParseError::internal("Grammar guarantees program exists"))?;

        let expr_pair = program_pair
            .into_inner()
            .next()
            .ok_or_else(|| RawParseError::internal("Grammar guarantees program contains expr"))?;

        Self::parse_expr(expr_pair)
    }

    fn parse_expr(pair: Pair<'_, Rule>) -> Result<RawExpr<'_>, RawParseError> {
        let pratt = PrattParser::new()
            .op(Op::infix(Rule::or, Left))
            .op(Op::infix(Rule::and, Left))
            .op(Op::prefix(Rule::neg));

        pratt
            .map_primary(Self::parse_primary)
            .map_infix(Self::parse_infix)
            .map_prefix(Self::parse_prefix)
            .parse(pair.into_inner())
    }

    fn parse_primary(pair: Pair<'_, Rule>) -> Result<RawExpr<'_>, RawParseError> {
        match pair.as_rule() {
            Rule::predicate => Self::parse_predicate(pair),
            Rule::glob_pattern => Ok(RawExpr::Glob(pair.as_span())),
            Rule::expr => Self::parse_expr(pair),
            rule => Err(RawParseError::internal(format!(
                "Unexpected primary rule: {:?}",
                rule
            ))),
        }
    }

    fn parse_infix<'a>(
        lhs: Result<RawExpr<'a>, RawParseError>,
        _pair: Pair<'a, Rule>,
        rhs: Result<RawExpr<'a>, RawParseError>,
    ) -> Result<RawExpr<'a>, RawParseError> {
        match _pair.as_rule() {
            Rule::and => Ok(RawExpr::And(Box::new(lhs?), Box::new(rhs?))),
            Rule::or => Ok(RawExpr::Or(Box::new(lhs?), Box::new(rhs?))),
            rule => Err(RawParseError::internal(format!(
                "Unexpected infix rule: {:?}",
                rule
            ))),
        }
    }

    fn parse_prefix<'a>(
        _pair: Pair<'a, Rule>,
        rhs: Result<RawExpr<'a>, RawParseError>,
    ) -> Result<RawExpr<'a>, RawParseError> {
        match _pair.as_rule() {
            Rule::neg => Ok(RawExpr::Not(Box::new(rhs?))),
            rule => Err(RawParseError::internal(format!(
                "Unexpected prefix rule: {:?}",
                rule
            ))),
        }
    }

    fn parse_predicate(pair: Pair<'_, Rule>) -> Result<RawExpr<'_>, RawParseError> {
        let span = pair.as_span();
        let mut inner = pair.into_inner();

        let selector_pair = inner
            .next()
            .ok_or_else(|| RawParseError::internal("Grammar guarantees predicate has selector"))?;
        let selector = selector_pair.as_str();
        let selector_span = selector_pair.as_span();

        let operator_pair = inner
            .next()
            .ok_or_else(|| RawParseError::internal("Grammar guarantees predicate has operator"))?;
        let operator = operator_pair.as_str();
        let operator_span = operator_pair.as_span();

        let value_pair = inner
            .next()
            .ok_or_else(|| RawParseError::internal("Grammar guarantees predicate has value"))?;
        let value_span = value_pair.as_span();
        let value = Self::parse_value(value_pair)?;

        Ok(RawExpr::Predicate(RawPredicate {
            selector,
            operator,
            value,
            span,
            selector_span,
            operator_span,
            value_span,
        }))
    }

    fn parse_value(pair: Pair<'_, Rule>) -> Result<RawValue<'_>, RawParseError> {
        match pair.as_rule() {
            Rule::value => {
                // value wraps the actual value type, so parse the inner content
                let inner = pair.into_inner().next().ok_or_else(|| {
                    RawParseError::internal("Grammar guarantees value has content")
                })?;
                Self::parse_value(inner)
            }
            Rule::quoted_string => {
                // Grammar already parsed inner content without quotes
                let inner = pair.into_inner().next().ok_or_else(|| {
                    RawParseError::internal("Grammar guarantees quoted_string has inner content")
                })?;
                Ok(RawValue::Quoted(inner.as_str()))
            }
            Rule::raw_token | Rule::bracketed | Rule::parenthesized |
            Rule::curly_braced | Rule::bare_token => {
                // All raw tokens stored as-is, typechecker decides meaning based on operator
                Ok(RawValue::Raw(pair.as_str()))
            }
            rule => Err(RawParseError::internal(format!(
                "Unexpected value rule: {:?}",
                rule
            ))),
        }
    }
}
