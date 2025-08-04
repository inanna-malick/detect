//! Parsing helper utilities for AST construction

use pest::iterators::{Pair, Pairs};
use pest::Span;

use crate::parse_error::ParseError;
use crate::parser::pratt_parser::Rule;

/// Extension trait for better error handling with iterators
pub(super) trait ParseIterExt<'i> {
    fn expect_next(&mut self, context: &'static str) -> Result<Pair<'i, Rule>, ParseError>;
}

impl<'i> ParseIterExt<'i> for Pairs<'i, Rule> {
    fn expect_next(&mut self, context: &'static str) -> Result<Pair<'i, Rule>, ParseError> {
        self.next()
            .ok_or(ParseError::Internal(context))
    }
}

/// Helper for extracting string from span
pub(super) fn span_to_string(span: Span) -> String {
    span.as_str().to_string()
}
