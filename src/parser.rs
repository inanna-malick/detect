use anyhow::Context as _;
use pest::{
    iterators::Pairs,
    pratt_parser::{Assoc::*, Op, PrattParser},
    Parser,
};
use pratt_parser::Rule;

use crate::{
    expr::Expr,
    predicate::{self, Predicate, RawPredicate, StreamingCompiledContentPredicate},
    MetadataPredicate, NamePredicate,
};

mod pratt_parser {
    use pest_derive::Parser;

    #[derive(Parser)]
    #[grammar = "expr/expr.pest"]
    pub struct Parser;
}

fn parse(pairs: Pairs<Rule>, pratt: &PrattParser<Rule>) -> anyhow::Result<Expr<RawPredicate>> {
    pratt
        .map_primary(|primary| match primary.as_rule() {
            Rule::predicate => {
                let mut inner = primary.into_inner();
                // TODO: catchall 'invalid selector' to support better error msgs
                let lhs = match inner.next().unwrap().into_inner().next().unwrap().as_rule() {
                    Rule::name => predicate::Selector::FileName,
                    Rule::path => predicate::Selector::FilePath,
                    Rule::ext => predicate::Selector::Extension,
                    Rule::size => predicate::Selector::Size,
                    Rule::r#type => predicate::Selector::EntityType,
                    Rule::contents => predicate::Selector::Contents,
                    Rule::modified => predicate::Selector::Modified,
                    Rule::created => predicate::Selector::Created,
                    Rule::accessed => predicate::Selector::Accessed,
                    x => panic!("{:?}", x),
                };

                let op = match inner.next().unwrap().as_rule() {
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
                    Rule::glob => predicate::Op::Glob,
                    _ => unreachable!(),
                };
                let rhs_pair = inner.next().unwrap();
                let rhs = match rhs_pair.as_rule() {
                    Rule::rhs => {
                        let inner_rhs = rhs_pair.into_inner().next().unwrap();
                        match inner_rhs.as_rule() {
                            Rule::quoted_string => {
                                // Extract the inner string without quotes
                                let quoted = inner_rhs.into_inner().next().unwrap();
                                let inner_str = quoted.into_inner().next().unwrap();
                                inner_str.as_str().to_string()
                            }
                            Rule::bare_token => inner_rhs.as_str().to_string(),
                            Rule::set_literal => {
                                // For set literals, we'll encode them as a special format
                                // that parse_string can recognize
                                let set_items = inner_rhs.into_inner().next().unwrap();
                                let mut items = Vec::new();
                                for item in set_items.into_inner() {
                                    if item.as_rule() == Rule::set_item {
                                        let inner_item = item.into_inner().next().unwrap();
                                        let value = match inner_item.as_rule() {
                                            Rule::quoted_string => {
                                                let quoted = inner_item.into_inner().next().unwrap();
                                                let inner_str = quoted.into_inner().next().unwrap();
                                                inner_str.as_str().to_string()
                                            }
                                            Rule::bare_token => inner_item.as_str().to_string(),
                                            _ => unreachable!(),
                                        };
                                        items.push(value);
                                    }
                                }
                                // Encode as JSON array string for now
                                serde_json::to_string(&items).unwrap()
                            }
                            _ => unreachable!(),
                        }
                    }
                    _ => unreachable!(),
                };

                Ok(Expr::Predicate(RawPredicate { lhs, op, rhs }))
            }
            Rule::expr => parse(primary.into_inner(), pratt),
            _ => unreachable!(),
        })
        .map_prefix(|op, rhs| match op.as_rule() {
            Rule::neg => Ok(Expr::Not(Box::new(rhs?))),
            _ => unreachable!(),
        })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            Rule::and => Ok(Expr::And(Box::new(lhs?), Box::new(rhs?))),
            Rule::or => Ok(Expr::Or(Box::new(lhs?), Box::new(rhs?))),
            _ => unreachable!(),
        })
        .parse(pairs)
}

pub fn parse_expr(
    input: &str,
) -> anyhow::Result<
    Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
> {
    let pratt = PrattParser::new()
        .op(Op::infix(Rule::or, Left))
        .op(Op::infix(Rule::and, Left))
        .op(Op::prefix(Rule::neg));

    let mut parse_tree =
        pratt_parser::Parser::parse(Rule::program, input).context("failed to parse input")?;

    let pairs = parse_tree
        .next()
        .context("inner of program")?
        .into_inner()
        .next()
        .context("inner of expr")?
        .into_inner();

    let expr = parse(pairs, &pratt)?;

    expr.map_predicate_err(|r| r.parse())
}
