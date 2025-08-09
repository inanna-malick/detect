use pest::{
    iterators::Pairs,
    pratt_parser::{Assoc::*, Op, PrattParser},
    Parser, Span,
};

use crate::{
    expr::Expr,
    parse_error::{ParseError, PredicateParseError, StructureErrorKind},
    predicate::{
        parse_time_value, Bound, MetadataPredicate, NamePredicate, NumberMatcher, Predicate,
        StreamingCompiledContentPredicate, StringMatcher, TimeMatcher,
    },
};

pub mod pratt_parser {
    use pest_derive::Parser;

    #[derive(Parser)]
    #[grammar = "expr/expr.pest"]
    pub struct Parser;
}

use pratt_parser::Rule;

// Size units and their multipliers
const BYTE: f64 = 1.0;
const KB: f64 = 1024.0;
const MB: f64 = KB * 1024.0;
const GB: f64 = MB * 1024.0;
const TB: f64 = GB * 1024.0;

const SIZE_UNITS: &[(&str, f64)] = &[
    ("b", BYTE),
    ("k", KB),
    ("kb", KB),
    ("m", MB),
    ("mb", MB),
    ("g", GB),
    ("gb", GB),
    ("t", TB),
    ("tb", TB),
];

// Extension trait for cleaner span location extraction
trait SpanExt {
    fn to_location(&self) -> (usize, usize);
}

impl SpanExt for Span<'_> {
    #[inline]
    fn to_location(&self) -> (usize, usize) {
        self.start_pos().line_col()
    }
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
            location: Some(span.to_location()),
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
            location: Some(span.to_location()),
        })?;

    let unit_lower = unit_part.to_lowercase();
    let multiplier = SIZE_UNITS
        .iter()
        .find(|(unit, _)| *unit == unit_lower)
        .map(|(_, mult)| *mult)
        .ok_or_else(|| ParseError::Structure {
            kind: StructureErrorKind::InvalidToken {
                expected: "size unit (B, KB, MB, GB, TB)",
                found: unit_part.to_string(),
            },
            location: Some(span.to_location()),
        })?;

    Ok((value * multiplier) as u64)
}

// Helper to extract string value from various forms
fn extract_string_value(pair: pest::iterators::Pair<Rule>) -> Result<String, ParseError> {
    match pair.as_rule() {
        Rule::string_value => {
            let inner = pair.into_inner().next().ok_or(ParseError::Internal(
                "grammar guarantees string_value has content",
            ))?;
            extract_string_value(inner)
        }
        Rule::quoted_string => {
            let quoted = pair.into_inner().next().ok_or(ParseError::Internal(
                "grammar guarantees quoted_string has content",
            ))?;
            let inner_str = quoted.into_inner().next().ok_or(ParseError::Internal(
                "grammar guarantees quoted content has string",
            ))?;
            Ok(inner_str.as_str().to_string())
        }
        Rule::bare_string => Ok(pair.as_str().to_string()),
        _ => Err(ParseError::unexpected_rule(pair.as_rule(), None)),
    }
}

// Parse predicate directly from pest pair to domain predicate
fn parse_predicate_direct(
    pair: pest::iterators::Pair<Rule>,
) -> Result<
    Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
    ParseError,
> {
    let mut inner = pair.into_inner();
    let pred_type = inner.next().ok_or(ParseError::Internal(
        "grammar guarantees typed_predicate has content",
    ))?;

    match pred_type.as_rule() {
        Rule::string_predicate => parse_string_predicate(pred_type),
        Rule::numeric_predicate => parse_numeric_predicate(pred_type),
        Rule::temporal_predicate => parse_temporal_predicate(pred_type),
        rule => Err(ParseError::unexpected_rule(rule, None)),
    }
}

// Parse string predicates (path, type, contents)
fn parse_string_predicate(
    pair: pest::iterators::Pair<Rule>,
) -> Result<
    Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
    ParseError,
> {
    let mut inner = pair.into_inner();

    // Get selector
    let selector_pair = inner.next().ok_or(ParseError::Internal(
        "grammar guarantees string_predicate has selector",
    ))?;

    // Get operator and value
    let op_value_pair = inner.next().ok_or(ParseError::Internal(
        "grammar guarantees string_predicate has op_value",
    ))?;

    // Parse selector type
    let selector_type = parse_string_selector(selector_pair)?;

    // Parse operator and value
    let (op, value, set_items) = parse_string_op_value(op_value_pair)?;

    // Create string matcher
    let string_matcher = if let Some(items) = set_items {
        StringMatcher::In(items.into_iter().collect())
    } else {
        match op {
            StringOp::Equals => StringMatcher::Equals(value),
            StringOp::NotEquals => StringMatcher::NotEquals(value),
            StringOp::Contains => StringMatcher::Contains(value),
            StringOp::Regex => {
                let pattern = if value == "*" { ".*" } else { &value };
                StringMatcher::regex(pattern).map_err(|e| ParseError::Predicate {
                    selector: selector_type.canonical(),
                    operator: crate::predicate::Op::Matches,
                    value: crate::predicate::RhsValue::String(value.clone()),
                    source: PredicateParseError::Regex(e),
                })?
            }
        }
    };

    // Create domain predicate based on selector
    let predicate = match selector_type {
        StringSelectorType::PathFull => Predicate::name(NamePredicate::FullPath(string_matcher)),
        StringSelectorType::PathParent => Predicate::name(NamePredicate::DirPath(string_matcher)),
        StringSelectorType::PathParentDir => {
            Predicate::name(NamePredicate::ParentDir(string_matcher))
        }
        StringSelectorType::PathName => Predicate::name(NamePredicate::FileName(string_matcher)),
        StringSelectorType::PathStem => Predicate::name(NamePredicate::BaseName(string_matcher)),
        StringSelectorType::PathSuffix => Predicate::name(NamePredicate::Extension(string_matcher)),
        StringSelectorType::Type => Predicate::meta(MetadataPredicate::Type(string_matcher)),
        StringSelectorType::Contents => {
            let pattern = match &string_matcher {
                StringMatcher::Regex(r) => r.as_str().to_string(),
                StringMatcher::Equals(s) => format!("^{}$", regex::escape(s)),
                StringMatcher::Contains(s) => regex::escape(s),
                _ => {
                    return Err(ParseError::invalid_token(
                        "regex, equals, contains, or not-equals for contents",
                        format!("{:?}", string_matcher),
                    ))
                }
            };
            let content_pred = StreamingCompiledContentPredicate::new(pattern)
                .map_err(|_| ParseError::Internal("Failed to compile content predicate"))?;
            Predicate::contents(content_pred)
        }
    };

    Ok(Expr::Predicate(predicate))
}

// Parse numeric predicates (size, depth)
fn parse_numeric_predicate(
    pair: pest::iterators::Pair<Rule>,
) -> Result<
    Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
    ParseError,
> {
    let mut inner = pair.into_inner();

    // Get selector
    let selector_pair = inner.next().ok_or(ParseError::Internal(
        "grammar guarantees numeric_predicate has selector",
    ))?;

    // Parse selector type
    let selector_type = match selector_pair.as_rule() {
        Rule::numeric_selector => {
            let inner = selector_pair
                .into_inner()
                .next()
                .ok_or(ParseError::Internal(
                    "grammar guarantees numeric_selector has type",
                ))?;
            if matches!(
                inner.as_rule(),
                Rule::meta_size | Rule::bare_size | Rule::size
            ) {
                NumericSelectorType::Size
            } else {
                NumericSelectorType::Depth
            }
        }
        Rule::size => NumericSelectorType::Size,
        Rule::depth => NumericSelectorType::Depth,
        _ => return Err(ParseError::unexpected_rule(selector_pair.as_rule(), None)),
    };

    // Get operator and value
    let op_value_pair = inner.next().ok_or(ParseError::Internal(
        "grammar guarantees numeric_predicate has op_value",
    ))?;

    // Parse operator and value
    let (op, value) = parse_numeric_op_value(op_value_pair, &selector_type)?;

    // Create number matcher
    let number_matcher = match op {
        NumericOp::Equals => NumberMatcher::Equals(value),
        NumericOp::NotEquals => NumberMatcher::NotEquals(value),
        NumericOp::Greater => NumberMatcher::In(Bound::Left((value + 1)..)),
        NumericOp::GreaterOrEqual => NumberMatcher::In(Bound::Left(value..)),
        NumericOp::Less => NumberMatcher::In(Bound::Right(..value)),
        NumericOp::LessOrEqual => NumberMatcher::In(Bound::Right(..(value + 1))),
    };

    // Create domain predicate
    let predicate = match selector_type {
        NumericSelectorType::Size => Predicate::meta(MetadataPredicate::Filesize(number_matcher)),
        NumericSelectorType::Depth => Predicate::meta(MetadataPredicate::Depth(number_matcher)),
    };

    Ok(Expr::Predicate(predicate))
}

// Parse temporal predicates (modified, created, accessed)
fn parse_temporal_predicate(
    pair: pest::iterators::Pair<Rule>,
) -> Result<
    Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>,
    ParseError,
> {
    let mut inner = pair.into_inner();

    // Get selector
    let selector_pair = inner.next().ok_or(ParseError::Internal(
        "grammar guarantees temporal_predicate has selector",
    ))?;

    // Parse selector type
    let selector_type = parse_temporal_selector(selector_pair)?;

    // Get operator and value
    let op_value_pair = inner.next().ok_or(ParseError::Internal(
        "grammar guarantees temporal_predicate has op_value",
    ))?;

    // Parse operator and value
    let (op, value_str) = parse_temporal_op_value(op_value_pair)?;

    // Parse time value
    let time_value = parse_time_value(&value_str).map_err(|e| ParseError::Predicate {
        selector: selector_type.canonical(),
        operator: crate::predicate::Op::Equality,
        value: crate::predicate::RhsValue::String(value_str.clone()),
        source: e,
    })?;

    // Create time matcher
    let time_matcher = match op {
        TemporalOp::Equals => TimeMatcher::Equals(time_value),
        TemporalOp::NotEquals => TimeMatcher::NotEquals(time_value),
        TemporalOp::Before => TimeMatcher::Before(time_value),
        TemporalOp::After => TimeMatcher::After(time_value),
    };

    // Create domain predicate
    let predicate = match selector_type {
        TemporalSelectorType::Modified => {
            Predicate::meta(MetadataPredicate::Modified(time_matcher))
        }
        TemporalSelectorType::Created => Predicate::meta(MetadataPredicate::Created(time_matcher)),
        TemporalSelectorType::Accessed => {
            Predicate::meta(MetadataPredicate::Accessed(time_matcher))
        }
    };

    Ok(Expr::Predicate(predicate))
}

// Selector types
#[derive(Debug, Clone)]
enum StringSelectorType {
    PathFull,
    PathParent,
    PathParentDir,
    PathName,
    PathStem,
    PathSuffix,
    Type,
    Contents,
}

impl StringSelectorType {
    fn canonical(&self) -> &'static str {
        match self {
            StringSelectorType::PathFull => "path.full",
            StringSelectorType::PathParent => "path.parent",
            StringSelectorType::PathParentDir => "path.parent_dir",
            StringSelectorType::PathName => "path.name",
            StringSelectorType::PathStem => "path.stem",
            StringSelectorType::PathSuffix => "path.extension",
            StringSelectorType::Type => "type",
            StringSelectorType::Contents => "contents",
        }
    }
}

#[derive(Debug, Clone)]
enum TemporalSelectorType {
    Modified,
    Created,
    Accessed,
}

impl TemporalSelectorType {
    fn canonical(&self) -> &'static str {
        match self {
            TemporalSelectorType::Modified => "modified",
            TemporalSelectorType::Created => "created",
            TemporalSelectorType::Accessed => "accessed",
        }
    }
}

#[derive(Debug, Clone)]
enum NumericSelectorType {
    Size,
    Depth,
}

impl NumericSelectorType {
    fn canonical(&self) -> &'static str {
        match self {
            NumericSelectorType::Size => "size",
            NumericSelectorType::Depth => "depth",
        }
    }
}

// Operator types
#[derive(Debug, Clone)]
enum StringOp {
    Equals,
    NotEquals,
    Contains,
    Regex,
}

#[derive(Debug, Clone)]
enum NumericOp {
    Equals,
    NotEquals,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
}

#[derive(Debug, Clone)]
enum TemporalOp {
    Equals,
    NotEquals,
    Before,
    After,
}

// Helper parsers
fn parse_string_selector(
    pair: pest::iterators::Pair<Rule>,
) -> Result<StringSelectorType, ParseError> {
    match pair.as_rule() {
        Rule::string_selector => {
            let inner = pair.into_inner().next().ok_or(ParseError::Internal(
                "grammar guarantees string_selector has content",
            ))?;
            parse_string_selector(inner)
        }
        Rule::path_selector => {
            let inner = pair.into_inner().next().ok_or(ParseError::Internal(
                "grammar guarantees path_selector has content",
            ))?;
            match inner.as_rule() {
                Rule::path_alias => Ok(StringSelectorType::PathFull),
                Rule::path_with_component => {
                    // path_with_component contains the component directly after dot
                    // The grammar: "path" ~ "." ~ (path_full | path_parent | ...)
                    let component = inner
                        .into_inner()
                        .next_back()
                        .ok_or(ParseError::Internal("grammar guarantees path component"))?;
                    match component.as_rule() {
                        Rule::path_full => Ok(StringSelectorType::PathFull),
                        Rule::path_parent => Ok(StringSelectorType::PathParent),
                        Rule::path_parent_dir => Ok(StringSelectorType::PathParentDir),
                        Rule::path_name => Ok(StringSelectorType::PathName),
                        Rule::path_stem => Ok(StringSelectorType::PathStem),
                        Rule::path_extension => Ok(StringSelectorType::PathSuffix),
                        rule => Err(ParseError::unexpected_rule(rule, None)),
                    }
                }
                rule => Err(ParseError::unexpected_rule(rule, None)),
            }
        }
        Rule::bare_path_shorthand => {
            let inner = pair.into_inner().next().ok_or(ParseError::Internal(
                "grammar guarantees bare_path_shorthand has content",
            ))?;
            match inner.as_rule() {
                Rule::bare_name => Ok(StringSelectorType::PathName),
                Rule::bare_stem => Ok(StringSelectorType::PathStem),
                Rule::bare_extension => Ok(StringSelectorType::PathSuffix),
                Rule::bare_parent => Ok(StringSelectorType::PathParent),
                Rule::bare_full => Ok(StringSelectorType::PathFull),
                rule => Err(ParseError::unexpected_rule(rule, None)),
            }
        }
        Rule::bare_name => Ok(StringSelectorType::PathName),
        Rule::bare_stem => Ok(StringSelectorType::PathStem),
        Rule::bare_extension => Ok(StringSelectorType::PathSuffix),
        Rule::bare_parent => Ok(StringSelectorType::PathParent),
        Rule::bare_full => Ok(StringSelectorType::PathFull),
        Rule::content_selector => Ok(StringSelectorType::Contents),
        Rule::bare_content => Ok(StringSelectorType::Contents),
        Rule::content_with_domain => Ok(StringSelectorType::Contents),
        Rule::type_selector | Rule::bare_type | Rule::meta_type => Ok(StringSelectorType::Type),
        rule => Err(ParseError::unexpected_rule(rule, None)),
    }
}

fn parse_temporal_selector(
    pair: pest::iterators::Pair<Rule>,
) -> Result<TemporalSelectorType, ParseError> {
    match pair.as_rule() {
        Rule::temporal_selector => {
            let inner = pair.into_inner().next().ok_or(ParseError::Internal(
                "grammar guarantees temporal_selector has content",
            ))?;
            parse_temporal_selector(inner)
        }
        Rule::time_with_domain => {
            // time_with_domain contains the component directly after dot
            // The grammar: "time" ~ "." ~ (modified | created | accessed)
            let component = pair
                .into_inner()
                .next_back()
                .ok_or(ParseError::Internal("grammar guarantees time component"))?;
            match component.as_rule() {
                Rule::modified => Ok(TemporalSelectorType::Modified),
                Rule::created => Ok(TemporalSelectorType::Created),
                Rule::accessed => Ok(TemporalSelectorType::Accessed),
                rule => Err(ParseError::unexpected_rule(rule, None)),
            }
        }
        Rule::bare_time => {
            // bare_time contains one of: modified, created, accessed
            let inner = pair.into_inner().next().ok_or(ParseError::Internal(
                "grammar guarantees bare_time has content",
            ))?;
            parse_temporal_selector(inner)
        }
        Rule::modified => Ok(TemporalSelectorType::Modified),
        Rule::created => Ok(TemporalSelectorType::Created),
        Rule::accessed => Ok(TemporalSelectorType::Accessed),
        rule => Err(ParseError::unexpected_rule(rule, None)),
    }
}

fn parse_string_op_value(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(StringOp, String, Option<Vec<String>>), ParseError> {
    let rule = pair.as_rule();
    let mut inner = pair.into_inner();

    match rule {
        Rule::string_eq => {
            inner.next(); // skip operator
            let value = extract_string_value(inner.next().ok_or(ParseError::Internal(
                "grammar guarantees string_eq has value",
            ))?)?;
            Ok((StringOp::Equals, value, None))
        }
        Rule::string_ne => {
            inner.next(); // skip operator
            let value = extract_string_value(inner.next().ok_or(ParseError::Internal(
                "grammar guarantees string_ne has value",
            ))?)?;
            Ok((StringOp::NotEquals, value, None))
        }
        Rule::string_contains => {
            inner.next(); // skip operator
            let value = extract_string_value(inner.next().ok_or(ParseError::Internal(
                "grammar guarantees string_contains has value",
            ))?)?;
            Ok((StringOp::Contains, value, None))
        }
        Rule::string_regex => {
            inner.next(); // skip operator
            let value = extract_string_value(inner.next().ok_or(ParseError::Internal(
                "grammar guarantees string_regex has value",
            ))?)?;
            Ok((StringOp::Regex, value, None))
        }
        Rule::string_in => {
            inner.next(); // skip operator
            let set_literal = inner
                .next()
                .ok_or(ParseError::Internal("grammar guarantees string_in has set"))?;
            let items = parse_set_items(set_literal)?;
            Ok((StringOp::Equals, String::new(), Some(items)))
        }
        rule => Err(ParseError::unexpected_rule(rule, None)),
    }
}

fn parse_set_items(pair: pest::iterators::Pair<Rule>) -> Result<Vec<String>, ParseError> {
    pair.into_inner()
        .filter(|item| item.as_rule() == Rule::set_items)
        .flat_map(|item| item.into_inner())
        .filter(|set_item| set_item.as_rule() == Rule::set_item)
        .map(|set_item| {
            let inner = set_item.into_inner().next().ok_or(ParseError::Internal(
                "grammar guarantees set_item has content",
            ))?;
            match inner.as_rule() {
                Rule::set_token => Ok(inner.as_str().to_string()),
                Rule::quoted_string => extract_string_value(inner),
                _ => Err(ParseError::Internal("unexpected rule in set_item")),
            }
        })
        .collect()
}

fn parse_numeric_op_value(
    pair: pest::iterators::Pair<Rule>,
    selector_type: &NumericSelectorType,
) -> Result<(NumericOp, u64), ParseError> {
    let rule = pair.as_rule();
    let mut inner = pair.into_inner();

    let (op, value_pair) = match rule {
        Rule::numeric_eq => {
            inner.next(); // skip operator
            (NumericOp::Equals, inner.next())
        }
        Rule::numeric_ne => {
            inner.next(); // skip operator
            (NumericOp::NotEquals, inner.next())
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
            (op, inner.next())
        }
        rule => return Err(ParseError::unexpected_rule(rule, None)),
    };

    let value_pair = value_pair.ok_or(ParseError::Internal(
        "grammar guarantees numeric operator has value",
    ))?;

    let value = parse_numeric_value(value_pair, selector_type)?;
    Ok((op, value))
}

fn parse_numeric_value(
    pair: pest::iterators::Pair<Rule>,
    selector_type: &NumericSelectorType,
) -> Result<u64, ParseError> {
    match pair.as_rule() {
        Rule::numeric_value => {
            let inner = pair.into_inner().next().ok_or(ParseError::Internal(
                "grammar guarantees numeric_value has content",
            ))?;
            parse_numeric_value(inner, selector_type)
        }
        Rule::size_value => parse_size_value_as_bytes(pair),
        Rule::bare_number => pair.as_str().parse().map_err(|e| ParseError::Predicate {
            selector: selector_type.canonical(),
            operator: crate::predicate::Op::Equality,
            value: crate::predicate::RhsValue::Number(0),
            source: PredicateParseError::Numeric(e),
        }),
        rule => Err(ParseError::unexpected_rule(rule, None)),
    }
}

fn parse_temporal_op_value(
    pair: pest::iterators::Pair<Rule>,
) -> Result<(TemporalOp, String), ParseError> {
    let rule = pair.as_rule();
    let mut inner = pair.into_inner();

    let (op, value_pair) = match rule {
        Rule::temporal_eq => {
            inner.next(); // skip operator
            (TemporalOp::Equals, inner.next())
        }
        Rule::temporal_ne => {
            inner.next(); // skip operator
            (TemporalOp::NotEquals, inner.next())
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
            (op, inner.next())
        }
        rule => return Err(ParseError::unexpected_rule(rule, None)),
    };

    let value_pair = value_pair.ok_or(ParseError::Internal(
        "grammar guarantees temporal operator has value",
    ))?;

    let value = parse_temporal_value(value_pair)?;
    Ok((op, value))
}

fn parse_temporal_value(pair: pest::iterators::Pair<Rule>) -> Result<String, ParseError> {
    match pair.as_rule() {
        Rule::temporal_value => {
            let inner = pair.into_inner().next().ok_or(ParseError::Internal(
                "grammar guarantees temporal_value has content",
            ))?;
            parse_temporal_value(inner)
        }
        Rule::absolute_date | Rule::relaxed_time_value | Rule::time_value | Rule::time_keyword => {
            Ok(pair.as_str().to_string())
        }
        Rule::quoted_string => extract_string_value(pair),
        rule => Err(ParseError::unexpected_rule(rule, None)),
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
            Rule::typed_predicate => parse_predicate_direct(primary),
            Rule::expr => parse_typed_predicates(primary.into_inner(), pratt),
            rule => Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: None,
            }),
        })
        .map_prefix(|op, rhs| match op.as_rule() {
            Rule::neg => Ok(Expr::negate(rhs?)),
            rule => Err(ParseError::Structure {
                kind: StructureErrorKind::UnexpectedRule { rule },
                location: None,
            }),
        })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            Rule::and => Ok(Expr::and(lhs?, rhs?)),
            Rule::or => Ok(Expr::or(lhs?, rhs?)),
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
        .map_err(|e| ParseError::Syntax(crate::parse_error::PestError(Box::new(e))))?;

    let program = parse_tree.next().ok_or(ParseError::Internal(
        "grammar guarantees program exists at root",
    ))?;

    let expr_pair = program.into_inner().next().ok_or(ParseError::Internal(
        "grammar guarantees program contains expression",
    ))?;

    let pairs = expr_pair.into_inner();

    // Use the new typed predicate parser
    parse_typed_predicates(pairs, &pratt)
}
