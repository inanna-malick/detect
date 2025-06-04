//! New progressive query AST

use crate::expr::Expr;
use crate::predicate::{
    MetadataPredicate, NamePredicate, Predicate, StreamingCompiledContentPredicate,
};

/// Progressive query types that map to the new grammar
#[derive(Debug, Clone, PartialEq)]
pub enum Query {
    /// Simple implicit search
    Implicit(Pattern),

    /// Search with filters
    Filtered {
        base: FilterBase,
        filters: Vec<Filter>,
    },

    /// Full boolean expression
    Expression(Box<Expression>),
}

/// Base for filtered searches
#[derive(Debug, Clone, PartialEq)]
pub enum FilterBase {
    Type(FileType),
    Pattern(Pattern),
    TypeWithPattern(FileType, Pattern),
}

/// Simple patterns without operators
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Quoted(String),        // "exact match"
    Regex(String, String), // /pattern/flags
    Glob(String),          // *.rs, **/*.js
    Bare(String),          // TODO, main
}

/// Filters that can be applied
#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    Size(SizeOp, f64, SizeUnit),
    Time(TimeSelector, TimeExpr),
    Path(String),
    Property(Property),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SizeOp {
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Equal,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SizeUnit {
    Bytes,
    Kilobytes,
    Megabytes,
    Gigabytes,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimeSelector {
    Modified,
    Created,
    Accessed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimeExpr {
    Relative(f64, TimeUnit),
    Keyword(TimeKeyword),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimeUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
    Weeks,
    Months,
    Years,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimeKeyword {
    Today,
    Yesterday,
    Now,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    Executable,
    Hidden,
    Empty,
    Binary,
    Symlink,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    Cpp,
    C,
    Image,
    Video,
    Audio,
    Text,
    Binary,
}

/// Full boolean expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Not(Box<Expression>),
    Atom(Atom),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Atom {
    Query(Query),
    Predicate(PredicateExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PredicateExpr {
    Comparison(Selector, CompOp, Value),
    Property(Selector),
    Contains(Pattern),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    Name,
    Path,
    Ext,
    Size,
    Type,
    Lines,
    Binary,
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompOp {
    Equal,
    NotEqual,
    Matches,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64, Option<SizeUnit>),
}

// ============================================================================
// Conversion to existing expression types
// ============================================================================

impl Query {
    pub fn to_expr(
        self,
    ) -> Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>> {
        self.to_expr_ref()
    }
    
    fn to_expr_ref(
        &self,
    ) -> Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>> {
        match self {
            Query::Implicit(pattern) => pattern.to_expr(),
            Query::Filtered { base, filters } => {
                let mut expr = base.to_expr();
                for filter in filters {
                    expr = Expr::and(expr, filter.to_expr());
                }
                expr
            }
            Query::Expression(expr) => expr.to_expr(),
        }
    }
}

impl FilterBase {
    fn to_expr(
        &self,
    ) -> Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>> {
        match self {
            FilterBase::Type(file_type) => file_type.to_expr(),
            FilterBase::Pattern(pattern) => pattern.to_expr(),
            FilterBase::TypeWithPattern(file_type, pattern) => {
                // Combine type and pattern with AND
                Expr::and(file_type.to_expr(), pattern.to_expr())
            }
        }
    }
}

impl Pattern {
    fn to_expr(
        &self,
    ) -> Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>> {
        match self {
            Pattern::Quoted(s) => {
                // Try as filename, then content
                Expr::or(
                    Expr::name_predicate(NamePredicate::Equals(s.clone())),
                    Expr::content_predicate(
                        StreamingCompiledContentPredicate::new(regex::escape(s)).unwrap(),
                    ),
                )
            }
            Pattern::Regex(pattern, flags) => {
                let regex_str = if flags.is_empty() {
                    pattern.clone()
                } else {
                    format!("(?{}){}", flags, pattern)
                };
                Expr::content_predicate(StreamingCompiledContentPredicate::new(regex_str).unwrap())
            }
            Pattern::Glob(glob) => {
                // If it's a path pattern, match against full path
                if glob.contains('/') {
                    let regex = glob_to_regex(glob);
                    Expr::name_predicate(NamePredicate::Path(
                        crate::predicate::StringMatcher::regex(&regex).unwrap(),
                    ))
                } else {
                    // Otherwise match against filename only
                    let regex = glob_to_regex(glob);
                    Expr::name_predicate(NamePredicate::Regex(regex))
                }
            }
            Pattern::Bare(word) => {
                // Content search
                Expr::content_predicate(
                    StreamingCompiledContentPredicate::new(regex::escape(word)).unwrap(),
                )
            }
        }
    }
}

impl FileType {
    fn to_expr(
        &self,
    ) -> Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>> {
        let extensions = match self {
            FileType::Rust => vec!["rs"],
            FileType::Python => vec!["py", "pyw"],
            FileType::JavaScript => vec!["js", "jsx", "mjs"],
            FileType::TypeScript => vec!["ts", "tsx"],
            FileType::Go => vec!["go"],
            FileType::Java => vec!["java"],
            FileType::Cpp => vec!["cpp", "cc", "cxx", "hpp"],
            FileType::C => vec!["c", "h"],
            FileType::Image => vec!["jpg", "jpeg", "png", "gif", "svg", "webp"],
            FileType::Video => vec!["mp4", "avi", "mov", "mkv", "webm"],
            FileType::Audio => vec!["mp3", "wav", "flac", "ogg", "m4a"],
            FileType::Text => vec!["txt"],
            FileType::Binary => return Expr::Literal(true), // TODO: implement binary detection
        };

        extensions
            .into_iter()
            .map(|ext| Expr::name_predicate(NamePredicate::Regex(format!(r"\.{}$", ext))))
            .reduce(Expr::or)
            .unwrap_or(Expr::Literal(false))
    }
}

impl Filter {
    fn to_expr(
        &self,
    ) -> Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>> {
        match self {
            Filter::Size(op, value, unit) => {
                let bytes = match unit {
                    SizeUnit::Bytes => *value as u64,
                    SizeUnit::Kilobytes => (*value * 1024.0) as u64,
                    SizeUnit::Megabytes => (*value * 1024.0 * 1024.0) as u64,
                    SizeUnit::Gigabytes => (*value * 1024.0 * 1024.0 * 1024.0) as u64,
                };

                match op {
                    SizeOp::Greater => Expr::meta_predicate(MetadataPredicate::SizeGreater(bytes)),
                    SizeOp::GreaterEqual => {
                        if bytes == 0 {
                            Expr::Literal(true)
                        } else {
                            Expr::meta_predicate(MetadataPredicate::SizeGreater(bytes - 1))
                        }
                    }
                    SizeOp::Less => Expr::meta_predicate(MetadataPredicate::SizeLess(bytes)),
                    SizeOp::LessEqual => {
                        Expr::meta_predicate(MetadataPredicate::SizeLess(bytes + 1))
                    }
                    SizeOp::Equal => Expr::meta_predicate(MetadataPredicate::SizeEquals(bytes)),
                }
            }
            Filter::Time(_, _) => {
                // TODO: implement time filters
                Expr::Literal(true)
            }
            Filter::Path(path) => {
                Expr::name_predicate(NamePredicate::Regex(format!("^{}", regex::escape(path))))
            }
            Filter::Property(prop) => match prop {
                Property::Executable => Expr::meta_predicate(MetadataPredicate::IsExecutable),
                Property::Hidden => Expr::name_predicate(NamePredicate::Regex(r"^\.".to_string())),
                Property::Empty => Expr::meta_predicate(MetadataPredicate::SizeEquals(0)),
                Property::Binary => Expr::Literal(true), // TODO: implement
                Property::Symlink => Expr::meta_predicate(MetadataPredicate::IsSymlink),
            },
        }
    }
}

impl Expression {
    fn to_expr(
        &self,
    ) -> Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>> {
        match self {
            Expression::And(a, b) => Expr::and(a.to_expr(), b.to_expr()),
            Expression::Or(a, b) => Expr::or(a.to_expr(), b.to_expr()),
            Expression::Not(a) => Expr::negate(a.to_expr()),
            Expression::Atom(atom) => atom.to_expr(),
        }
    }
}

impl Atom {
    fn to_expr(
        &self,
    ) -> Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>> {
        match self {
            Atom::Query(q) => q.to_expr_ref(),
            Atom::Predicate(p) => p.to_expr(),
        }
    }
}

impl PredicateExpr {
    fn to_expr(
        &self,
    ) -> Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>> {
        match self {
            PredicateExpr::Comparison(selector, op, value) => {
                match (selector, op, value) {
                    (Selector::Name, CompOp::Equal, Value::String(s)) => {
                        Expr::name_predicate(NamePredicate::Equals(s.clone()))
                    }
                    (Selector::Name, CompOp::Matches, Value::String(s)) => {
                        Expr::name_predicate(NamePredicate::Regex(s.clone()))
                    }
                    (Selector::Size, CompOp::Greater, Value::Number(n, _)) => {
                        Expr::meta_predicate(MetadataPredicate::SizeGreater(*n as u64))
                    }
                    (Selector::Size, CompOp::Less, Value::Number(n, _)) => {
                        Expr::meta_predicate(MetadataPredicate::SizeLess(*n as u64))
                    }
                    // TODO: implement other combinations
                    _ => Expr::Literal(true),
                }
            }
            PredicateExpr::Property(selector) => {
                match selector {
                    Selector::Empty => Expr::meta_predicate(MetadataPredicate::SizeEquals(0)),
                    Selector::Binary => Expr::Literal(true), // TODO
                    _ => Expr::Literal(true),
                }
            }
            PredicateExpr::Contains(pattern) => pattern.to_expr(),
        }
    }
}

/// Convert glob pattern to regex
fn glob_to_regex(glob: &str) -> String {
    let mut regex = String::new();
    let mut chars = glob.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    regex.push_str(".*");
                } else {
                    regex.push_str("[^/]*");
                }
            }
            '?' => regex.push('.'),
            '[' => {
                regex.push('[');
                for ch in chars.by_ref() {
                    regex.push(ch);
                    if ch == ']' {
                        break;
                    }
                }
            }
            '.' | '^' | '$' | '(' | ')' | '+' | '|' | '\\' | '{' | '}' => {
                regex.push('\\');
                regex.push(ch);
            }
            _ => regex.push(ch),
        }
    }

    regex
}
