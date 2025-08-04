//! Simple value types with pest-ast derives
use pest_ast::*;

use super::parse_helpers::span_to_string;
use crate::parser::pratt_parser::Rule;

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
