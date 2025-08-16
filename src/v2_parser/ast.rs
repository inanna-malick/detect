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
    String(&'a str),   // Content with quotes stripped but escapes preserved
    Set(Vec<&'a str>), // Set of strings with quotes stripped but escapes preserved
}

#[derive(Debug, Clone, PartialEq)]
pub enum RawExpr<'a> {
    Not(Box<RawExpr<'a>>),
    And(Box<RawExpr<'a>>, Box<RawExpr<'a>>),
    Or(Box<RawExpr<'a>>, Box<RawExpr<'a>>),
    Predicate(RawPredicate<'a>),
    Glob(pest::Span<'a>),
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
            RawExpr::Glob(span) => test_utils::RawTestExpr::Glob(span.as_str()),
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
    /// Get the string value for single string values
    pub fn as_string(&self) -> Option<&'a str> {
        match self {
            RawValue::String(s) => Some(s),
            RawValue::Set(_) => None,
        }
    }

    /// Get the set values for set values
    pub fn as_set(&self) -> Option<&[&'a str]> {
        match self {
            RawValue::String(_) => None,
            RawValue::Set(items) => Some(items),
        }
    }

    /// Convert to test-friendly value without spans
    pub fn to_test_value(&self) -> test_utils::RawTestValue<'a> {
        match self {
            RawValue::String(s) => test_utils::RawTestValue::String(s),
            RawValue::Set(items) => test_utils::RawTestValue::Set(items.clone()),
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
        String(&'a str),   // Content with quotes stripped but escapes preserved
        Set(Vec<&'a str>), // Set of strings with quotes stripped but escapes preserved
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum RawTestExpr<'a> {
        Not(Box<RawTestExpr<'a>>),
        And(Box<RawTestExpr<'a>>, Box<RawTestExpr<'a>>),
        Or(Box<RawTestExpr<'a>>, Box<RawTestExpr<'a>>),
        Predicate(RawTestPredicate<'a>),
        Glob(&'a str),
    }

    impl<'a> RawTestExpr<'a> {
        /// Helper constructor for creating a string predicate
        pub fn string_predicate(selector: &'a str, operator: &'a str, value: &'a str) -> Self {
            RawTestExpr::Predicate(RawTestPredicate {
                selector,
                operator,
                value: RawTestValue::String(value),
            })
        }

        /// Helper constructor for creating a set predicate
        pub fn set_predicate(selector: &'a str, operator: &'a str, values: Vec<&'a str>) -> Self {
            RawTestExpr::Predicate(RawTestPredicate {
                selector,
                operator,
                value: RawTestValue::Set(values),
            })
        }

        /// Helper constructor for creating a glob expression
        pub fn glob(pattern: &'a str) -> Self {
            RawTestExpr::Glob(pattern)
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
        pub fn not(expr: RawTestExpr<'a>) -> Self {
            RawTestExpr::Not(Box::new(expr))
        }
    }
}
