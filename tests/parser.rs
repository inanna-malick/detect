use detect::expr::Expr;
use detect::parser::parse_expr;
use detect::predicate::{
    Bound, MetadataPredicate, NamePredicate, NumberMatcher, Predicate,
    StreamingCompiledContentPredicate, StringMatcher,
};

type TestExpr =
    Expr<Predicate<NamePredicate, MetadataPredicate, StreamingCompiledContentPredicate>>;

fn test_cases(cases: &[(&str, TestExpr)]) {
    for (input, expected) in cases {
        let parsed =
            parse_expr(input).unwrap_or_else(|e| panic!("Failed to parse '{}': {:?}", input, e));
        assert_eq!(parsed, *expected, "Mismatch for '{}'", input);
    }
}

fn test_parse_ok(cases: &[&str]) {
    for input in cases {
        parse_expr(input).unwrap_or_else(|e| panic!("Failed to parse '{}': {:?}", input, e));
    }
}

fn test_parse_err(cases: &[&str]) {
    for input in cases {
        assert!(
            parse_expr(input).is_err(),
            "Expected parse error for '{}', but it succeeded",
            input
        );
    }
}

#[test]
fn test_path_shorthands() {
    test_cases(&[
        ("name == foo", name_eq("foo")),
        ("filename == foo", filename_eq("foo")),
        ("path.name == foo", name_eq("foo")),
        ("stem == README", stem_eq("README")),
        ("path.stem == README", stem_eq("README")),
        ("extension == rs", ext_eq("rs")),
        ("ext == rs", ext_eq("rs")),
        ("path.extension == rs", ext_eq("rs")),
        ("path.ext == rs", ext_eq("rs")),
        ("parent == /usr/bin", parent_eq("/usr/bin")),
        ("path.parent == /usr/bin", parent_eq("/usr/bin")),
        ("full == /usr/bin/foo", full_path_eq("/usr/bin/foo")),
        ("path.full == /usr/bin/foo", full_path_eq("/usr/bin/foo")),
        ("path == /usr/bin/foo", full_path_eq("/usr/bin/foo")),
    ]);
}

#[test]
fn test_content_selectors() {
    let expected = content_contains("TODO");
    test_cases(&[
        (r#"content.text contains "TODO""#, expected),
        (r#"text contains "TODO""#, content_contains("TODO")),
        (r#"contents contains "TODO""#, content_contains("TODO")),
        (r#"content contains "TODO""#, content_contains("TODO")),
    ]);
}

#[test]
fn test_name_operations() {
    test_cases(&[
        ("path.name == foo", name_eq("foo")),
        ("path.name != bar", name_ne("bar")),
        ("path.name ~= test.*", name_regex("test.*").unwrap()),
        (r#"path.name contains "test""#, name_contains("test")),
    ]);
}

#[test]
fn test_regex_patterns() {
    test_parse_ok(&[
        r"path.name ~= ^[0-9]{10,13}.*\.ts$",
        r#"path.name ~= "(foo|bar)""#,
        r"path.name ~= test\?.*",
        r"contents ~= TODO.*\{.*\}",
        r#"contents ~= "@(Injectable|Component)""#,
        r"contents ~= class\s+\w+Service",
    ]);
}

#[test]
fn test_set_operations() {
    test_cases(&[
        ("path.name in [foo,bar,baz]", name_in(["foo", "bar", "baz"])),
        (
            r#"path.name in ["foo","bar","baz"]"#,
            name_in(["foo", "bar", "baz"]),
        ),
        (
            "path.name in [foo, bar, baz]",
            name_in(["foo", "bar", "baz"]),
        ),
        (
            r#"path.name in ["foo", "bar", "baz"]"#,
            name_in(["foo", "bar", "baz"]),
        ),
        ("extension in [js, ts, jsx]", ext_in(["js", "ts", "jsx"])),
        ("path.ext in [rs, toml]", ext_in(["rs", "toml"])),
    ]);
}

#[test]
fn test_size_operations() {
    test_cases(&[
        ("size > 1000", size_gt(1000)),
        ("filesize > 1000", size_gt(1000)),
        ("bytes > 1000", size_gt(1000)),
        ("size < 1000", size_lt(1000)),
        ("size >= 1000", size_gte(1000)),
        ("size <= 1000", size_lte(1000)),
        ("size == 1000", size_eq(1000)),
        ("size != 1000", size_ne(1000)),
    ]);
}

#[test]
fn test_size_units() {
    test_cases(&[
        ("size > 1kb", size_gt(1024)),
        ("size > 1KB", size_gt(1024)),
        ("size > 1k", size_gt(1024)),
        ("size > 1K", size_gt(1024)),
        ("bytes > 1mb", size_gt(1048576)),
        ("size > 1mb", size_gt(1048576)),
        ("size > 1MB", size_gt(1048576)),
        ("size > 1m", size_gt(1048576)),
        ("size > 1M", size_gt(1048576)),
        ("size > 1gb", size_gt(1073741824)),
        ("size > 1GB", size_gt(1073741824)),
        ("size > 1g", size_gt(1073741824)),
        ("size > 1G", size_gt(1073741824)),
        ("size > 1tb", size_gt(1099511627776)),
        ("size > 1TB", size_gt(1099511627776)),
        ("size > 1t", size_gt(1099511627776)),
        ("size > 1T", size_gt(1099511627776)),
        ("size > 1.5mb", size_gt(1572864)),
        ("size > 2.5gb", size_gt(2684354560)),
    ]);
}

#[test]
fn test_temporal_operations() {
    test_parse_ok(&[
        "modified > -7days",
        "modified > -7.days",
        "modified > -7d",
        "created < -30minutes",
        "created < -30min",
        "created < -30m",
        "accessed == now",
        "accessed == today",
        "mtime > yesterday",
        "ctime > 2024-01-01",
        "mdate > -1d",
        "cdate < now",
        "adate > yesterday",
        "time.modified > -1hours",
        "time.created < -1h",
    ]);
}

#[test]
fn test_boolean_operations() {
    test_cases(&[
        (
            "size > 100 && name == foo",
            Expr::and(size_gt(100), name_eq("foo")),
        ),
        (
            "size > 100 || name == foo",
            Expr::or(size_gt(100), name_eq("foo")),
        ),
        ("!name == foo", Expr::negate(name_eq("foo"))),
        ("!(size > 100)", Expr::negate(size_gt(100))),
        (
            "size > 100 && (name == foo || name == bar)",
            Expr::and(size_gt(100), Expr::or(name_eq("foo"), name_eq("bar"))),
        ),
    ]);
}

#[test]
fn test_case_insensitive_keywords() {
    test_cases(&[
        (
            "size > 100 and name == foo",
            Expr::and(size_gt(100), name_eq("foo")),
        ),
        (
            "size > 100 OR name == foo",
            Expr::or(size_gt(100), name_eq("foo")),
        ),
        ("NOT name == foo", Expr::negate(name_eq("foo"))),
    ]);
}

#[test]
fn test_glob_patterns() {
    // Test that glob patterns parse successfully
    test_parse_ok(&[
        "*.rs",
        "*test*",
        "src/*.rs",
        "**/*.rs",
        "*.{rs,toml}",
        "test_*.txt",
        "??.rs",
        "[ab]*.txt",
    ]);

    // Test glob patterns in boolean expressions
    test_parse_ok(&[
        "*.rs && size > 1kb",
        "*test* || contents contains TODO",
        "NOT *.md",
        "(*.rs || *.toml) && modified > -7days",
    ]);
}

#[test]
fn test_size_with_spaces() {
    // Test that spaces between number and unit work
    test_parse_ok(&[
        "size > 10 kb",
        "size < 1 mb",
        "size >= 100 gb",
        "size == 5 tb",
        "size > 10 kb && modified > -7days",
    ]);
}

#[test]
fn test_type_selectors() {
    test_cases(&[
        (r#"type == "file""#, type_eq("file")),
        (r#"filetype == "file""#, type_eq("file")),
        (r#"meta.type == "file""#, type_eq("file")),
        (r#"type == "directory""#, type_eq("directory")),
        (r#"type == "dir""#, type_eq("dir")),
    ]);
}

#[test]
fn test_quoted_strings() {
    test_cases(&[
        (r#"name == "foo bar""#, name_eq("foo bar")),
        (
            r#"contents contains "TODO: fix this""#,
            content_contains("TODO: fix this"),
        ),
        (
            r#"path == "/path with spaces/file.txt""#,
            full_path_eq("/path with spaces/file.txt"),
        ),
        (r#"name == 'foo bar'"#, name_eq("foo bar")),
        (
            r#"parent contains 'my folder'"#,
            parent_contains("my folder"),
        ),
    ]);
}

#[test]
fn test_operator_precedence() {
    test_cases(&[
        // Expr::and binds tighter than OR
        (
            "name == a || name == b && size > 100",
            Expr::or(name_eq("a"), Expr::and(name_eq("b"), size_gt(100))),
        ),
        // NOT binds tightest
        (
            "!name == foo && size > 100",
            Expr::and(Expr::negate(name_eq("foo")), size_gt(100)),
        ),
        // Parentheses override precedence
        (
            "(name == a || name == b) && size > 100",
            Expr::and(Expr::or(name_eq("a"), name_eq("b")), size_gt(100)),
        ),
    ]);
}

#[test]
fn test_complex_expressions() {
    test_cases(&[
        (
            r#"extension in [rs, toml] && size > 1kb && !path.parent contains "target""#,
            Expr::and(
                Expr::and(ext_in(["rs", "toml"]), size_gt(1024)),
                Expr::negate(parent_contains("target")),
            ),
        ),
        (
            r#"type == "file" && size < 10mb && (name ~= ".*\.rs$" || name ~= ".*\.toml$")"#,
            Expr::and(
                Expr::and(type_eq("file"), size_lt(10485760)),
                Expr::or(
                    name_regex(r".*\.rs$").unwrap(),
                    name_regex(r".*\.toml$").unwrap(),
                ),
            ),
        ),
    ]);
}

#[test]
fn test_content_operations() {
    test_cases(&[
        (r#"contents contains "TODO""#, content_contains("TODO")),
        (r#"contents ~= "TODO|FIXME""#, content_regex("TODO|FIXME")),
        (
            r#"contents ~= "(BEGIN|END).*(PRIVATE|KEY)""#,
            content_regex("(BEGIN|END).*(PRIVATE|KEY)"),
        ),
    ]);
}

#[test]
fn test_error_cases() {
    test_parse_err(&[
        "size >",
        "name ==",
        "modified > -7",
        "size > 1zb",
        "unknown_selector == foo",
        "path.unknown == foo",
        "((name == foo)",
        "name == foo))",
        "&& name == foo",
        "name == foo ||",
        r#"name ~= "[unclosed""#,
    ]);
}

#[test]
fn test_alternate_operators() {
    test_cases(&[
        ("name ~ foo.*", name_regex("foo.*").unwrap()),
        ("name = foo", name_eq("foo")),
        ("name =~ foo.*", name_regex("foo.*").unwrap()),
    ]);
}

#[test]
fn test_bare_vs_quoted_values() {
    test_cases(&[
        ("name == foo", name_eq("foo")),
        ("name == foo.txt", name_eq("foo.txt")),
        ("name == foo-bar_baz", name_eq("foo-bar_baz")),
        (r#"name == "foo bar""#, name_eq("foo bar")),
        (r#"name == "foo(bar)""#, name_eq("foo(bar)")),
        (r#"name == "foo&bar""#, name_eq("foo&bar")),
    ]);
}

#[test]
fn test_path_with_dots() {
    test_cases(&[
        ("path == ./foo/bar.txt", full_path_eq("./foo/bar.txt")),
        ("path == ../foo/bar.txt", full_path_eq("../foo/bar.txt")),
        ("parent == ./foo", parent_eq("./foo")),
    ]);
}

#[test]
fn test_multiline_content() {
    test_parse_ok(&[
        r#"contents ~= "struct \{[\s\S]*?field""#,
        r#"contents ~= "TODO[\s\S]*?DONE""#,
    ]);
}

// Boilerplate (helper functions)

// Name predicate helpers - most common patterns
pub fn name_eq(s: &str) -> Expr {
    Expr::name_predicate(NamePredicate::stem_eq(s))
}

pub fn name_ne(s: &str) -> Expr {
    Expr::name_predicate(NamePredicate::BaseName(StringMatcher::ne(s)))
}

pub fn name_contains(s: &str) -> Expr {
    Expr::name_predicate(NamePredicate::BaseName(StringMatcher::contains(s)))
}

pub fn name_regex(s: &str) -> Result<Expr, regex::Error> {
    Ok(Expr::name_predicate(NamePredicate::BaseName(
        StringMatcher::regex(s)?,
    )))
}

pub fn name_in<I: IntoIterator<Item = S>, S: AsRef<str>>(items: I) -> Expr {
    Expr::name_predicate(NamePredicate::BaseName(StringMatcher::in_set(items)))
}

pub fn filename_eq(s: &str) -> Expr {
    Expr::name_predicate(NamePredicate::file_eq(s))
}

// Stem (BaseName) helpers
pub fn stem_eq(s: &str) -> Expr {
    Expr::name_predicate(NamePredicate::stem_eq(s))
}

pub fn stem_contains(s: &str) -> Expr {
    Expr::name_predicate(NamePredicate::BaseName(StringMatcher::contains(s)))
}

// Extension helpers
pub fn ext_eq(s: &str) -> Expr {
    Expr::name_predicate(NamePredicate::ext_eq(s))
}

pub fn ext_contains(s: &str) -> Expr {
    Expr::name_predicate(NamePredicate::Extension(StringMatcher::contains(s)))
}

pub fn ext_in<I: IntoIterator<Item = S>, S: AsRef<str>>(items: I) -> Expr {
    Expr::name_predicate(NamePredicate::ext_in(items))
}

// Parent directory (DirPath) helpers
pub fn parent_eq(s: &str) -> Expr {
    Expr::name_predicate(NamePredicate::DirPath(StringMatcher::eq(s)))
}

pub fn parent_contains(s: &str) -> Expr {
    Expr::name_predicate(NamePredicate::DirPath(StringMatcher::contains(s)))
}

// Full path helpers
pub fn full_path_eq(s: &str) -> Expr {
    Expr::name_predicate(NamePredicate::path_eq(s))
}

pub fn full_path_contains(s: &str) -> Expr {
    Expr::name_predicate(NamePredicate::FullPath(StringMatcher::contains(s)))
}

// Metadata helpers
pub fn type_eq(s: &str) -> Expr {
    Expr::meta_predicate(MetadataPredicate::Type(StringMatcher::eq(s)))
}

pub fn size_eq(bytes: u64) -> Expr {
    Expr::meta_predicate(MetadataPredicate::Filesize(NumberMatcher::Equals(bytes)))
}

pub fn size_ne(bytes: u64) -> Expr {
    Expr::meta_predicate(MetadataPredicate::Filesize(NumberMatcher::NotEquals(bytes)))
}

pub fn size_gt(bytes: u64) -> Expr {
    Expr::meta_predicate(MetadataPredicate::Filesize(NumberMatcher::In(Bound::Left(
        (bytes + 1)..,
    ))))
}

pub fn size_gte(bytes: u64) -> Expr {
    Expr::meta_predicate(MetadataPredicate::Filesize(NumberMatcher::In(Bound::Left(
        bytes..,
    ))))
}

pub fn size_lt(bytes: u64) -> Expr {
    Expr::meta_predicate(MetadataPredicate::Filesize(NumberMatcher::In(
        Bound::Right(..bytes),
    )))
}

pub fn size_lte(bytes: u64) -> Expr {
    Expr::meta_predicate(MetadataPredicate::Filesize(NumberMatcher::In(
        Bound::Right(..(bytes + 1)),
    )))
}

// Content helpers
pub fn content_contains(pattern: &str) -> Expr {
    Expr::content_predicate(StreamingCompiledContentPredicate::new(regex::escape(pattern)).unwrap())
}

pub fn content_regex(pattern: &str) -> Expr {
    Expr::content_predicate(StreamingCompiledContentPredicate::new(pattern.to_string()).unwrap())
}
