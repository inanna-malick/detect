use pest::{
    iterators::Pair,
    pratt_parser::{Assoc::*, Op, PrattParser},
    Parser,
};
use pest_derive::Parser;

use super::{
    ast::{RawExpr, RawPredicate, RawValue},
    error::{RawParseError, SpanExt},
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

        Self::parse_expr(expr_pair).map_err(|e| e.with_source(input.to_string()))
    }

    /// Parse set contents from a string like "rs, js, ts" or "foo, \"bar, baz\", qux"
    /// Used by typechecker for 'in' operator
    ///
    /// Properly handles:
    /// - Quoted items with commas: `"foo, bar", baz`
    /// - Bare items: `rs, js, ts`
    /// - Mixed: `foo, "bar baz", qux`
    /// - Trailing commas: `rs, js,`
    /// - Empty sets: ``
    pub fn parse_set_contents(input: &str) -> Result<Vec<String>, RawParseError> {
        let pairs = Self::parse(Rule::set_contents, input)
            .map_err(|e| RawParseError::from_pest(Box::new(e), input.to_string()))?;

        let items: Vec<String> = pairs
            .flat_map(|pair| pair.into_inner()) // set_contents -> set_items or EOI
            .filter(|pair| pair.as_rule() == Rule::set_items)
            .flat_map(|pair| pair.into_inner()) // set_items -> set_item*
            .filter_map(|item_pair| {
                // set_item -> quoted_string | bare_set_item
                item_pair.into_inner().next()
            })
            .map(|inner| {
                match inner.as_rule() {
                    Rule::quoted_string => {
                        // quoted_string -> inner_double | inner_single (quotes stripped)
                        // Preserve all whitespace inside quotes
                        inner
                            .into_inner()
                            .next()
                            .map(|s| s.as_str().to_string())
                            .unwrap_or_default()
                    }
                    Rule::bare_set_item => {
                        // Trim whitespace from bare items
                        inner.as_str().trim().to_string()
                    }
                    _ => String::new(), // Should never happen
                }
            })
            .filter(|s| !s.is_empty())
            .collect();

        Ok(items)
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
                // value = { value_content ~ trailing_quote? }
                // Check if there's a trailing quote error
                let mut inner = pair.into_inner();
                let value_content = inner.next().ok_or_else(|| {
                    RawParseError::internal("Grammar guarantees value has content")
                })?;

                // Check for trailing quote
                if let Some(trailing) = inner.next() {
                    if trailing.as_rule() == Rule::trailing_quote {
                        let span = trailing.as_span();
                        let quote = span.as_str().chars().next().unwrap_or('"');
                        return Err(RawParseError::StrayQuote {
                            span: span.to_source_span(),
                            quote,
                            src: String::new(), // Will be filled by with_source()
                        });
                    }
                }

                // No trailing quote, parse the value content
                Self::parse_value(value_content)
            }
            Rule::quoted_string => {
                // Grammar already parsed inner content without quotes
                let inner = pair.into_inner().next().ok_or_else(|| {
                    RawParseError::internal("Grammar guarantees quoted_string has inner content")
                })?;
                Ok(RawValue::Quoted(inner.as_str()))
            }
            Rule::unterminated_string => {
                // Matched an unterminated string literal - return error with proper span
                let span = pair.as_span();
                let text = span.as_str();
                let quote = text.chars().next().unwrap_or('"');

                // Point span at just the opening quote and a few chars (not extending to EOI)
                let start = span.start();
                let length = text.len().min(10); // Show first 10 chars max
                let error_span = (start, length).into();

                Err(RawParseError::UnterminatedString {
                    span: error_span,
                    quote,
                    src: String::new(), // Will be filled by with_source()
                })
            }
            Rule::raw_token => {
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
