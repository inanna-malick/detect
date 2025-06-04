use pest::{
    iterators::Pair,
    pratt_parser::{Assoc::*, Op, PrattParser},
    Parser,
};

use crate::{
    expr::Expr,
    predicate::{Predicate, StreamingCompiledContentPredicate},
    query::*,
    MetadataPredicate, NamePredicate,
};

mod pratt_parser {
    use pest_derive::Parser;

    #[derive(Parser)]
    #[grammar = "expr/expr.pest"]
    pub struct ExprParser;
}

use pratt_parser::Rule;

/// Parse input into a Query AST
pub fn parse_query(input: &str) -> anyhow::Result<Query> {
    let mut pairs = pratt_parser::ExprParser::parse(Rule::program, input)?;
    let query_pair = pairs
        .next()
        .and_then(|p| p.into_inner().next())
        .ok_or_else(|| anyhow::anyhow!("expected query"))?;

    parse_query_pair(query_pair)
}

fn parse_query_pair(pair: Pair<Rule>) -> anyhow::Result<Query> {
    match pair.as_rule() {
        Rule::query => {
            let inner = pair.into_inner().next().unwrap();
            parse_query_pair(inner)
        }
        Rule::implicit_search => Ok(Query::Implicit(parse_pattern(pair)?)),
        Rule::filtered_search => parse_filtered_search(pair),
        Rule::expression => Ok(Query::Expression(Box::new(parse_expression(pair)?))),
        Rule::type_with_pattern => {
            // Type + pattern at top level
            let mut parts = pair.into_inner();
            let file_type = parse_file_type_str(parts.next().unwrap().as_str())?;
            let pattern = parse_pattern(parts.next().unwrap())?;
            Ok(Query::Filtered {
                base: FilterBase::TypeWithPattern(file_type, pattern),
                filters: vec![],
            })
        }
        Rule::file_type => {
            // Standalone file type becomes a filtered query with just the type
            let file_type = parse_file_type_str(pair.as_str())?;
            Ok(Query::Filtered {
                base: FilterBase::Type(file_type),
                filters: vec![],
            })
        }
        Rule::predicate => {
            // Standalone predicate becomes an expression
            let predicate = parse_predicate(pair)?;
            Ok(Query::Expression(Box::new(Expression::Atom(
                Atom::Predicate(predicate),
            ))))
        }
        _ => anyhow::bail!("unexpected rule: {:?}", pair.as_rule()),
    }
}

fn parse_pattern(pair: Pair<Rule>) -> anyhow::Result<Pattern> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::quoted_string => {
            let s = inner.into_inner().next().unwrap().as_str();
            Ok(Pattern::Quoted(s.to_string()))
        }
        Rule::regex_pattern => {
            let mut parts = inner.into_inner();
            let pattern = parts.next().unwrap().as_str().to_string();
            let flags = parts
                .next()
                .map(|f| f.as_str().to_string())
                .unwrap_or_default();
            Ok(Pattern::Regex(pattern, flags))
        }
        Rule::glob_pattern => Ok(Pattern::Glob(inner.as_str().to_string())),
        Rule::bare_word => Ok(Pattern::Bare(inner.as_str().to_string())),
        _ => anyhow::bail!("unexpected pattern rule: {:?}", inner.as_rule()),
    }
}

fn parse_filtered_search(pair: Pair<Rule>) -> anyhow::Result<Query> {
    let mut base = None;
    let mut filters = Vec::new();

    for part in pair.into_inner() {
        match part.as_rule() {
            Rule::type_with_pattern => {
                // Handle "rust TODO" case
                let mut parts = part.into_inner();
                let file_type = parse_file_type_str(parts.next().unwrap().as_str())?;
                let pattern = parse_pattern(parts.next().unwrap())?;
                base = Some(FilterBase::TypeWithPattern(file_type, pattern));
            }
            Rule::search_base => {
                base = Some(parse_search_base(part)?);
            }
            Rule::size_filter => filters.push(parse_size_filter(part)?),
            Rule::time_filter => filters.push(parse_time_filter(part)?),
            Rule::path_filter => filters.push(parse_path_filter(part)?),
            Rule::property_filter => filters.push(parse_property_filter(part)?),
            _ => {}
        }
    }

    Ok(Query::Filtered {
        base: base.ok_or_else(|| anyhow::anyhow!("filtered search needs base"))?,
        filters,
    })
}

fn parse_search_base(pair: Pair<Rule>) -> anyhow::Result<FilterBase> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::file_type => {
            let file_type = parse_file_type_str(inner.as_str())?;
            Ok(FilterBase::Type(file_type))
        }
        Rule::implicit_search => {
            let pattern = parse_pattern(inner)?;
            Ok(FilterBase::Pattern(pattern))
        }
        _ => anyhow::bail!("unexpected search base rule: {:?}", inner.as_rule()),
    }
}

fn parse_file_type_str(s: &str) -> anyhow::Result<FileType> {
    match s {
        "rust" | "rs" => Ok(FileType::Rust),
        "python" | "py" => Ok(FileType::Python),
        "javascript" | "js" => Ok(FileType::JavaScript),
        "typescript" | "ts" => Ok(FileType::TypeScript),
        "go" => Ok(FileType::Go),
        "java" => Ok(FileType::Java),
        "cpp" | "c++" => Ok(FileType::Cpp),
        "c" => Ok(FileType::C),
        "image" => Ok(FileType::Image),
        "video" => Ok(FileType::Video),
        "audio" => Ok(FileType::Audio),
        "text" => Ok(FileType::Text),
        "binary" => Ok(FileType::Binary),
        _ => anyhow::bail!("unknown file type: {}", s),
    }
}


fn parse_size_filter(pair: Pair<Rule>) -> anyhow::Result<Filter> {
    let mut inner = pair.into_inner();
    let op_str = inner.next().unwrap().as_str();
    let size_value_pair = inner.next().unwrap();

    let op = match op_str {
        ">" => SizeOp::Greater,
        ">=" => SizeOp::GreaterEqual,
        "<" => SizeOp::Less,
        "<=" => SizeOp::LessEqual,
        "=" => SizeOp::Equal,
        _ => anyhow::bail!("unknown size op: {}", op_str),
    };

    let (value, unit) = parse_size_value(size_value_pair.as_str())?;
    Ok(Filter::Size(op, value, unit))
}

fn parse_size_value(s: &str) -> anyhow::Result<(f64, SizeUnit)> {
    let mut chars = s.chars().peekable();
    let mut num_str = String::new();

    while let Some(&ch) = chars.peek() {
        if ch.is_numeric() || ch == '.' {
            num_str.push(ch);
            chars.next();
        } else {
            break;
        }
    }

    let value: f64 = num_str.parse()?;
    let unit_str: String = chars.collect();

    let unit = match unit_str.to_uppercase().as_str() {
        "" | "B" => SizeUnit::Bytes,
        "K" | "KB" => SizeUnit::Kilobytes,
        "M" | "MB" => SizeUnit::Megabytes,
        "G" | "GB" => SizeUnit::Gigabytes,
        _ => anyhow::bail!("unknown size unit: {}", unit_str),
    };

    Ok((value, unit))
}

fn parse_time_filter(pair: Pair<Rule>) -> anyhow::Result<Filter> {
    let mut inner = pair.into_inner();
    let selector_str = inner.next().unwrap().as_str();
    let time_expr_pair = inner.next().unwrap();

    let selector = match selector_str {
        "modified" | "m" => TimeSelector::Modified,
        "created" | "c" => TimeSelector::Created,
        "accessed" | "a" => TimeSelector::Accessed,
        _ => anyhow::bail!("unknown time selector: {}", selector_str),
    };

    let time_expr = parse_time_expr(time_expr_pair)?;
    Ok(Filter::Time(selector, time_expr))
}

fn parse_time_expr(pair: Pair<Rule>) -> anyhow::Result<TimeExpr> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::relative_time => {
            let s = inner.as_str();
            let (value, unit) = parse_relative_time(s)?;
            Ok(TimeExpr::Relative(value, unit))
        }
        Rule::time_keyword => match inner.as_str() {
            "today" => Ok(TimeExpr::Keyword(TimeKeyword::Today)),
            "yesterday" => Ok(TimeExpr::Keyword(TimeKeyword::Yesterday)),
            "now" => Ok(TimeExpr::Keyword(TimeKeyword::Now)),
            _ => anyhow::bail!("unknown time keyword: {}", inner.as_str()),
        },
        _ => anyhow::bail!("unexpected time expr rule: {:?}", inner.as_rule()),
    }
}

fn parse_relative_time(s: &str) -> anyhow::Result<(f64, TimeUnit)> {
    let mut chars = s.chars().peekable();
    let mut num_str = String::new();

    while let Some(&ch) = chars.peek() {
        if ch.is_numeric() || ch == '.' {
            num_str.push(ch);
            chars.next();
        } else {
            break;
        }
    }

    let value: f64 = num_str.parse()?;
    let unit_str: String = chars.collect();

    let unit = match unit_str.as_str() {
        "s" => TimeUnit::Seconds,
        "m" => TimeUnit::Minutes,
        "h" => TimeUnit::Hours,
        "d" => TimeUnit::Days,
        "w" => TimeUnit::Weeks,
        "mo" => TimeUnit::Months,
        "y" => TimeUnit::Years,
        _ => anyhow::bail!("unknown time unit: {}", unit_str),
    };

    Ok((value, unit))
}

fn parse_path_filter(pair: Pair<Rule>) -> anyhow::Result<Filter> {
    let mut inner = pair.into_inner();
    let _prefix = inner.next(); // Skip prefix
    let path_value = inner.next().unwrap();

    let path = match path_value.as_rule() {
        Rule::quoted_string => path_value.into_inner().next().unwrap().as_str().to_string(),
        Rule::path_value => path_value.as_str().to_string(),
        _ => anyhow::bail!("unexpected path value rule: {:?}", path_value.as_rule()),
    };

    Ok(Filter::Path(path))
}

fn parse_property_filter(pair: Pair<Rule>) -> anyhow::Result<Filter> {
    let prop = match pair.as_str() {
        "executable" => Property::Executable,
        "hidden" => Property::Hidden,
        "empty" => Property::Empty,
        "binary" => Property::Binary,
        "symlink" => Property::Symlink,
        _ => anyhow::bail!("unknown property: {}", pair.as_str()),
    };

    Ok(Filter::Property(prop))
}

fn parse_expression(pair: Pair<Rule>) -> anyhow::Result<Expression> {
    let pratt = PrattParser::new()
        .op(Op::infix(Rule::or_op, Left))
        .op(Op::infix(Rule::and_op, Left))
        .op(Op::prefix(Rule::not_op));

    parse_expr_pratt(pair.into_inner(), &pratt)
}

fn parse_expr_pratt(
    pairs: pest::iterators::Pairs<Rule>,
    pratt: &PrattParser<Rule>,
) -> anyhow::Result<Expression> {
    pratt
        .map_primary(|primary| -> anyhow::Result<Expression> {
            match primary.as_rule() {
                Rule::atom => {
                    let inner = primary.into_inner().next().unwrap();
                    match inner.as_rule() {
                        Rule::expression => parse_expression(inner),
                        Rule::predicate => {
                            Ok(Expression::Atom(Atom::Predicate(parse_predicate(inner)?)))
                        }
                        Rule::filtered_search => {
                            Ok(Expression::Atom(Atom::Query(parse_filtered_search(inner)?)))
                        }
                        Rule::implicit_search => Ok(Expression::Atom(Atom::Query(
                            Query::Implicit(parse_pattern(inner)?),
                        ))),
                        _ => anyhow::bail!("unexpected atom rule: {:?}", inner.as_rule()),
                    }
                }
                Rule::or_expr | Rule::and_expr | Rule::not_expr => {
                    parse_expr_pratt(primary.into_inner(), pratt)
                }
                _ => anyhow::bail!("unexpected expr rule: {:?}", primary.as_rule()),
            }
        })
        .map_prefix(|op, rhs| match op.as_rule() {
            Rule::not_op => Ok(Expression::Not(Box::new(rhs?))),
            _ => unreachable!(),
        })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            Rule::and_op => Ok(Expression::And(Box::new(lhs?), Box::new(rhs?))),
            Rule::or_op => Ok(Expression::Or(Box::new(lhs?), Box::new(rhs?))),
            _ => unreachable!(),
        })
        .parse(pairs)
}

fn parse_predicate(pair: Pair<Rule>) -> anyhow::Result<PredicateExpr> {
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();

    match first.as_rule() {
        Rule::selector => {
            let selector = parse_selector(first)?;
            if let Some(op_pair) = inner.next() {
                let op = parse_comp_op(op_pair)?;
                let value = parse_value(inner.next().unwrap())?;
                Ok(PredicateExpr::Comparison(selector, op, value))
            } else {
                Ok(PredicateExpr::Property(selector))
            }
        }
        Rule::contains_expr => {
            let mut inner = first.into_inner();
            // Skip "contains" and "("
            let pattern_pair = inner
                .find(|p| matches!(p.as_rule(), Rule::pattern))
                .ok_or_else(|| anyhow::anyhow!("expected pattern in contains expression"))?;
            let pattern = parse_contains_pattern(pattern_pair)?;
            Ok(PredicateExpr::Contains(pattern))
        }
        _ => anyhow::bail!("unexpected predicate rule: {:?}", first.as_rule()),
    }
}

fn parse_selector(pair: Pair<Rule>) -> anyhow::Result<Selector> {
    match pair.as_str() {
        "name" => Ok(Selector::Name),
        "path" => Ok(Selector::Path),
        "ext" => Ok(Selector::Ext),
        "size" => Ok(Selector::Size),
        "type" => Ok(Selector::Type),
        "lines" => Ok(Selector::Lines),
        "binary" => Ok(Selector::Binary),
        "empty" => Ok(Selector::Empty),
        _ => anyhow::bail!("unknown selector: {}", pair.as_str()),
    }
}

fn parse_comp_op(pair: Pair<Rule>) -> anyhow::Result<CompOp> {
    match pair.as_str() {
        "=" => Ok(CompOp::Equal),
        "!=" => Ok(CompOp::NotEqual),
        "~" => Ok(CompOp::Matches),
        ">" => Ok(CompOp::Greater),
        ">=" => Ok(CompOp::GreaterEqual),
        "<" => Ok(CompOp::Less),
        "<=" => Ok(CompOp::LessEqual),
        _ => anyhow::bail!("unknown comparison op: {}", pair.as_str()),
    }
}

fn parse_value(pair: Pair<Rule>) -> anyhow::Result<Value> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::quoted_string => {
            let s = inner.into_inner().next().unwrap().as_str();
            Ok(Value::String(s.to_string()))
        }
        Rule::number_value => {
            let (value, unit) = parse_number_value(inner.as_str())?;
            Ok(Value::Number(value, unit))
        }
        Rule::bare_value => Ok(Value::String(inner.as_str().to_string())),
        _ => anyhow::bail!("unexpected value rule: {:?}", inner.as_rule()),
    }
}

fn parse_number_value(s: &str) -> anyhow::Result<(f64, Option<SizeUnit>)> {
    let mut chars = s.chars().peekable();
    let mut num_str = String::new();

    while let Some(&ch) = chars.peek() {
        if ch.is_numeric() || ch == '.' {
            num_str.push(ch);
            chars.next();
        } else {
            break;
        }
    }

    let value: f64 = num_str.parse()?;
    let unit_str: String = chars.collect();

    if unit_str.is_empty() {
        Ok((value, None))
    } else {
        let unit = match unit_str.to_uppercase().as_str() {
            "B" => SizeUnit::Bytes,
            "K" | "KB" => SizeUnit::Kilobytes,
            "M" | "MB" => SizeUnit::Megabytes,
            "G" | "GB" => SizeUnit::Gigabytes,
            _ => return Ok((value, None)), // Not a size unit
        };
        Ok((value, Some(unit)))
    }
}

fn parse_contains_pattern(pair: Pair<Rule>) -> anyhow::Result<Pattern> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::regex_pattern => {
            let mut parts = inner.into_inner();
            let pattern = parts.next().unwrap().as_str().to_string();
            let flags = parts
                .next()
                .map(|f| f.as_str().to_string())
                .unwrap_or_default();
            Ok(Pattern::Regex(pattern, flags))
        }
        Rule::quoted_string => {
            let s = inner.into_inner().next().unwrap().as_str();
            Ok(Pattern::Quoted(s.to_string()))
        }
        _ => anyhow::bail!("unexpected contains pattern rule: {:?}", inner.as_rule()),
    }
}

/// Parse and convert to existing expression type
pub fn parse_expr(
    input: &str,
) -> anyhow::Result<
    Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
> {
    let query = parse_query(input)?;
    Ok(query.to_expr())
}
