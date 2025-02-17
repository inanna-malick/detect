use anyhow::Context as _;
use pest::{
    iterators::Pairs,
    pratt_parser::{Assoc::*, Op, PrattParser},
    Parser,
};
use pratt_parser::Rule;

use crate::{
    expr::Expr,
    predicate::{self, CompiledContentPredicate, Predicate, RawPredicate},
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
                let lhs = match inner.next().unwrap().into_inner().next().unwrap().as_rule() {
                    Rule::name => predicate::Selector::FileName,
                    Rule::path => predicate::Selector::FilePath,
                    Rule::ext => predicate::Selector::Extension,
                    Rule::size => predicate::Selector::Size,
                    Rule::r#type => predicate::Selector::EntityType,
                    Rule::contents => predicate::Selector::Contents,
                    x => panic!("{:?}", x),
                };

                let op = match inner.next().unwrap().as_rule() {
                    Rule::eq => predicate::Op::Equality,
                    Rule::like => predicate::Op::Matches,
                    Rule::gt => predicate::Op::NumericComparison(predicate::NumericalOp::Greater),
                    Rule::gteq => {
                        predicate::Op::NumericComparison(predicate::NumericalOp::GreaterOrEqual)
                    }
                    Rule::lt => predicate::Op::NumericComparison(predicate::NumericalOp::Less),
                    Rule::lteq => {
                        predicate::Op::NumericComparison(predicate::NumericalOp::LessOrEqual)
                    }
                    _ => unreachable!(),
                };
                let rhs = inner.next().unwrap().as_str().to_string();

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

pub(crate) fn parse_expr(
    input: &str,
) -> anyhow::Result<Expr<Predicate<NamePredicate, MetadataPredicate, CompiledContentPredicate>>> {
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
