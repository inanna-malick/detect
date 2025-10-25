#[derive(Debug, Clone, PartialEq)]
pub struct RawPredicate<'a> {
    pub selector: &'a str,
    pub operator: &'a str,
    pub value: RawValue<'a>,
    pub span: pest::Span<'a>,
    // Subcomponent spans for precise error reporting
    pub selector_span: pest::Span<'a>,
    pub operator_span: pest::Span<'a>,
    pub value_span: pest::Span<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RawValue<'a> {
    Quoted(&'a str), // Explicitly quoted by user (quotes stripped, escapes preserved)
    Raw(&'a str),    // Raw token - could be bare word, [set], (group), {curly}, etc.
                     // Typechecker interprets based on operator context
}

#[derive(Debug, Clone, PartialEq)]
pub enum RawExpr<'a> {
    Not(Box<RawExpr<'a>>),
    And(Box<RawExpr<'a>>, Box<RawExpr<'a>>),
    Or(Box<RawExpr<'a>>, Box<RawExpr<'a>>),
    Predicate(RawPredicate<'a>),
    SingleWord(pest::Span<'a>),
}

impl<'a> RawExpr<'a> {
    /// Convert to test-friendly expression without spans
    pub fn to_test_expr(&self) -> test_utils::RawTestExpr<'a> {
        match self {
            RawExpr::Not(expr) => test_utils::RawTestExpr::Not(Box::new(expr.to_test_expr())),
            RawExpr::And(left, right) => test_utils::RawTestExpr::And(
                Box::new(left.to_test_expr()),
                Box::new(right.to_test_expr()),
            ),
            RawExpr::Or(left, right) => test_utils::RawTestExpr::Or(
                Box::new(left.to_test_expr()),
                Box::new(right.to_test_expr()),
            ),
            RawExpr::Predicate(pred) => {
                test_utils::RawTestExpr::Predicate(pred.to_test_predicate())
            }
            RawExpr::SingleWord(span) => test_utils::RawTestExpr::SingleWord(span.as_str()),
        }
    }
}

impl<'a> RawPredicate<'a> {
    /// Convert to test-friendly predicate without spans
    pub fn to_test_predicate(&self) -> test_utils::RawTestPredicate<'a> {
        test_utils::RawTestPredicate {
            selector: self.selector,
            operator: self.operator,
            value: self.value.to_test_value(),
        }
    }
}

impl<'a> RawValue<'a> {
    /// Get the string value (works for both Quoted and Raw)
    pub fn as_string(&self) -> &'a str {
        match self {
            RawValue::Quoted(s) | RawValue::Raw(s) => s,
        }
    }

    /// Check if this is a quoted value (user explicitly quoted it)
    pub fn is_quoted(&self) -> bool {
        matches!(self, RawValue::Quoted(_))
    }

    /// Convert to test-friendly value without spans
    pub fn to_test_value(&self) -> test_utils::RawTestValue<'a> {
        match self {
            RawValue::Quoted(s) => test_utils::RawTestValue::Quoted(s),
            RawValue::Raw(s) => test_utils::RawTestValue::Raw(s),
        }
    }
}

pub mod test_utils {
    #[derive(Debug, Clone, PartialEq)]
    pub struct RawTestPredicate<'a> {
        pub selector: &'a str,
        pub operator: &'a str,
        pub value: RawTestValue<'a>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum RawTestValue<'a> {
        Quoted(&'a str), // Explicitly quoted by user
        Raw(&'a str),    // Raw token (bare word, [brackets], (parens), {curlies})
    }

    impl<'a> std::fmt::Display for RawTestValue<'a> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                RawTestValue::Quoted(s) | RawTestValue::Raw(s) => write!(f, "{}", s),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum RawTestExpr<'a> {
        Not(Box<RawTestExpr<'a>>),
        And(Box<RawTestExpr<'a>>, Box<RawTestExpr<'a>>),
        Or(Box<RawTestExpr<'a>>, Box<RawTestExpr<'a>>),
        Predicate(RawTestPredicate<'a>),
        SingleWord(&'a str),
    }

    impl<'a> RawTestExpr<'a> {
        /// Helper constructor for creating a raw token predicate
        pub fn string_predicate(selector: &'a str, operator: &'a str, value: &'a str) -> Self {
            RawTestExpr::Predicate(RawTestPredicate {
                selector,
                operator,
                value: RawTestValue::Raw(value),
            })
        }

        /// Helper constructor for creating a quoted value predicate
        pub fn quoted_predicate(selector: &'a str, operator: &'a str, value: &'a str) -> Self {
            RawTestExpr::Predicate(RawTestPredicate {
                selector,
                operator,
                value: RawTestValue::Quoted(value),
            })
        }

        /// Helper constructor for creating a set predicate (now just raw token with brackets)
        pub fn set_predicate(selector: &'a str, operator: &'a str, values: Vec<&'a str>) -> Self {
            // Format as bracketed list
            let value = format!("[{}]", values.join(","));
            RawTestExpr::Predicate(RawTestPredicate {
                selector,
                operator,
                value: RawTestValue::Raw(Box::leak(value.into_boxed_str())),
            })
        }

        /// Helper constructor for creating a single-word expression
        pub fn single_word(word: &'a str) -> Self {
            RawTestExpr::SingleWord(word)
        }

        /// Helper constructor for creating an AND expression
        pub fn and(left: RawTestExpr<'a>, right: RawTestExpr<'a>) -> Self {
            RawTestExpr::And(Box::new(left), Box::new(right))
        }

        /// Helper constructor for creating an OR expression
        pub fn or(left: RawTestExpr<'a>, right: RawTestExpr<'a>) -> Self {
            RawTestExpr::Or(Box::new(left), Box::new(right))
        }

        /// Helper constructor for creating a NOT expression
        #[allow(clippy::should_implement_trait)]
        pub fn not(expr: RawTestExpr<'a>) -> Self {
            RawTestExpr::Not(Box::new(expr))
        }
    }
}
