use std::{env::set_current_dir, fs::create_dir_all};

use slog::{o, Discard, Logger};
use tempfile::TempDir;

// Test helper functions
fn test_logger() -> Logger {
    Logger::root(Discard, o!())
}

fn f<'a>(path: &'a str, content: &'a str) -> TestFile<'a> {
    let (path, name) = if path.contains('/') {
        path.rsplit_once('/').unwrap()
    } else {
        ("", path)
    };

    TestFile {
        path,
        name,
        content,
    }
}

#[derive(Clone)]
struct TestFile<'a> {
    path: &'a str,
    name: &'a str,
    content: &'a str,
}

struct Case<'a> {
    expr: &'static str,
    expected: &'static [&'static str],
    files: Vec<TestFile<'a>>,
}

impl<'a> Case<'a> {
    fn build(&self) -> TempDir {
        let t = tempfile::Builder::new()
            .prefix("fileset-expr")
            .tempdir()
            .unwrap();
        let tmp_path = t.path().to_str().unwrap();
        for file in self.files.iter() {
            create_dir_all(format!("{}/{}", tmp_path, file.path)).unwrap();
            std::fs::write(
                format!("{}/{}/{}", tmp_path, file.path, file.name),
                file.content,
            )
            .unwrap();
        }

        t
    }

    async fn run(&self) {
        let tmp_dir = self.build();
        let mut out = Vec::new();
        set_current_dir(tmp_dir.path()).unwrap();
        detect::parse_and_run_fs(
            test_logger(),
            tmp_dir.path(),
            false,
            self.expr.to_owned(),
            detect::RuntimeConfig::default(),
            |p| {
                let s = p
                    .strip_prefix(format!("{}/", tmp_dir.path().to_str().unwrap()))
                    .unwrap()
                    .as_os_str()
                    .to_string_lossy()
                    .into_owned();
                out.push(s)
            },
        )
        .await
        .unwrap();

        out.sort();
        let mut expected = self.expected.to_owned();
        expected.sort();

        // Sort both sides for ordering-agnostic comparison
        assert_eq!(expected, out, "Failed for expression: {}", self.expr)
    }
}

// Shared test runner helper
async fn run_test_cases<'a>(
    cases: Vec<(&'static str, &'static [&'static str], Vec<TestFile<'a>>)>,
) {
    for (expr, expected, files) in cases {
        Case {
            expr,
            expected,
            files,
        }
        .run()
        .await;
    }
}

#[tokio::test]
async fn test_path_operations() {
    let basic_files = vec![
        f("foo", "foo"),
        f("bar/foo", "baz"),
        f("bar/baz", "foo"),
        f("z/foo/bar", ""),
    ];

    let cases = vec![
        // Test name only
        (
            "basename == foo",
            &["foo", "z/foo", "bar/foo"][..],
            basic_files.clone(),
        ),
        // Test path only
        (
            "path ~= bar",
            &["bar", "bar/baz", "bar/foo"][..],
            vec![f("foo", "foo"), f("bar/foo", "baz"), f("bar/baz", "foo")],
        ),
        // Test not name
        (
            "!basename == foo",
            &["bar", "bar/baz"][..],
            vec![f("foo", "foo"), f("bar/foo", "baz"), f("bar/baz", "foo")],
        ),
        // Test name and content
        (
            "basename == foo && content ~= foo",
            &["foo"][..],
            basic_files.clone(),
        ),
        // Test parent directory
        (
            "dir == bar",
            &["bar/foo", "bar/baz"][..],
            basic_files.clone(),
        ),
        // Test stem
        (
            "basename == README",
            &["README.md", "README"][..],
            vec![
                f("README.md", "# Hello"),
                f("readme.md", "# hello"),
                f("README", "text"),
            ],
        ),
        // Test full path
        ("path == bar/foo", &["bar/foo"][..], basic_files.clone()),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_extension_operations() {
    let code_files = vec![
        f("main.rs", "fn main() {}"),
        f("lib.rs", "// lib"),
        f("test.rs", "// test"),
        f("style.css", "body {}"),
        f("app.js", "console.log()"),
        f("component.jsx", "React"),
        f("README.md", "# Doc"),
        f("Makefile", "build:"),
    ];

    let cases = vec![
        (
            "ext == rs",
            &["main.rs", "lib.rs", "test.rs"][..],
            code_files.clone(),
        ),
        (
            "ext != rs",
            &[
                "style.css",
                "app.js",
                "component.jsx",
                "README.md",
                "Makefile",
            ][..],
            code_files.clone(),
        ),
        ("ext == \"\"", &["Makefile"][..], code_files.clone()),
        (
            "ext in [js, jsx, ts, tsx]",
            &["app.js", "component.jsx"][..],
            code_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_size_operations() {
    let kb_size = "x".repeat(1025);
    let size_files = vec![
        f("large.txt", &kb_size),
        f("small.txt", "tiny"),
        f("exact.txt", "12345"),
        f("empty.txt", ""),
    ];

    let cases = vec![
        ("size > 1000", &["large.txt"][..], size_files.clone()),
        (
            "size < 10",
            &["small.txt", "exact.txt", "empty.txt"][..],
            size_files.clone(),
        ),
        ("size == 5", &["exact.txt"][..], size_files.clone()),
        (
            "size >= 5",
            &["large.txt", "exact.txt"][..],
            size_files.clone(),
        ),
        (
            "size <= 5",
            &["small.txt", "exact.txt", "empty.txt"][..],
            size_files.clone(),
        ),
        (
            "basename == small && size < 5", // name without extension
            &["small.txt"][..],
            size_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_content_operations() {
    let content_files = vec![
        f("main.rs", "// TODO: implement feature"),
        f("lib.rs", "/* TODO: add tests */"),
        f("done.rs", "// All tasks completed"),
        f("readme.md", "# Project TODO list"),
        f("config.json", "{\"todo\": false}"),
    ];

    let cases = vec![
        (
            r#"content contains "TODO""#,
            &["main.rs", "lib.rs", "readme.md"][..],
            content_files.clone(),
        ),
        (
            r#"content ~= "TODO|FIXME""#,
            &["main.rs", "lib.rs", "readme.md"][..],
            content_files.clone(),
        ),
        (
            r#"content ~= "^//.*TODO""#,
            &["main.rs"][..],
            content_files.clone(),
        ),
        (
            r#"ext == rs && content contains "TODO""#,
            &["main.rs", "lib.rs"][..],
            content_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_boolean_operations() {
    let bool_files = vec![
        f("test.rs", "// test"),
        f("test_utils.rs", "// utils"),
        f("main.rs", "fn main()"),
        f("lib.rs", "// lib"),
        f("doc.txt", "docs"),
    ];

    let cases = vec![
        // AND operation
        (
            r#"ext == "rs" && basename contains "test""#,
            &["test.rs", "test_utils.rs"][..],
            bool_files.clone(),
        ),
        // OR operation
        (
            r#"basename == "main" || basename == "lib""#, // name without extension
            &["main.rs", "lib.rs"][..],
            bool_files.clone(),
        ),
        // NOT operation
        (
            r#"!(basename contains "test")"#,
            &["main.rs", "lib.rs", "doc.txt"][..],
            bool_files.clone(),
        ),
        // Complex: (A || B) && !C
        (
            r#"(basename contains "main" || basename contains "lib") && !basename contains "test""#,
            &["main.rs", "lib.rs"][..],
            bool_files.clone(),
        ),
        // Nested parentheses
        (
            r#"ext == "rs" && !(basename == "test" || basename == "main")"#, // name without extension
            &["test_utils.rs", "lib.rs"][..],
            bool_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_regex_patterns() {
    let regex_files = vec![
        f("test_utils.rs", ""),
        f("test_integration.rs", ""),
        f("main_test.rs", ""),
        f("tests.rs", ""),
        f("src/test/utils.rs", ""),
        f("lib/test/helpers.rs", ""),
        f("test/main.rs", ""),
    ];

    let cases = vec![
        (
            r#"basename ~= "test_.*""#, // name without extension
            &["test_utils.rs", "test_integration.rs"][..],
            regex_files.clone(),
        ),
        (
            r#"path ~= "(^|.*/)?test/.*\.rs$""#,
            &["src/test/utils.rs", "lib/test/helpers.rs", "test/main.rs"][..],
            regex_files.clone(),
        ),
        (
            r#"basename ~= "^test""#,
            &[
                "test_utils.rs",
                "test_integration.rs",
                "tests.rs",
                "test",
                "src/test",
                "lib/test",
            ][..],
            regex_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_set_operations() {
    let set_files = vec![
        f("main.rs", ""),
        f("lib.rs", ""),
        f("app.js", ""),
        f("style.css", ""),
        f("index.html", ""),
        f("README.md", ""),
    ];

    let cases = vec![
        (
            "ext in [rs, js, ts]",
            &["main.rs", "lib.rs", "app.js"][..],
            set_files.clone(),
        ),
        (
            "basename in [main, app, index]", // name without extension
            &["main.rs", "app.js", "index.html"][..],
            set_files.clone(),
        ),
        (
            r#"basename in ["main", "lib", "app"]"#,
            &["main.rs", "lib.rs", "app.js"][..],
            set_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_type_selectors() {
    // Type tests need special handling for directories
    let tmp_dir = tempfile::Builder::new()
        .prefix("detect-type")
        .tempdir()
        .unwrap();

    // Create files and directories
    std::fs::write(tmp_dir.path().join("file1.txt"), "content").unwrap();
    std::fs::write(tmp_dir.path().join("file2.rs"), "code").unwrap();
    std::fs::create_dir(tmp_dir.path().join("mydir")).unwrap();
    std::fs::create_dir(tmp_dir.path().join("another_dir")).unwrap();
    std::fs::write(tmp_dir.path().join("mydir").join("nested.txt"), "nested").unwrap();

    // Test file type
    let mut files = Vec::new();
    detect::parse_and_run_fs(
        test_logger(),
        tmp_dir.path(),
        false,
        r#"type == "file""#.to_owned(),
        detect::RuntimeConfig::default(),
        |p| files.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    assert!(files.contains(&"file1.txt".to_string()));
    assert!(files.contains(&"file2.rs".to_string()));
    assert!(files.contains(&"nested.txt".to_string()));
    assert!(!files.contains(&"mydir".to_string()));

    // Test directory type
    let mut dirs = Vec::new();
    detect::parse_and_run_fs(
        test_logger(),
        tmp_dir.path(),
        false,
        r#"type == "dir""#.to_owned(),
        detect::RuntimeConfig::default(),
        |p| dirs.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    assert!(dirs.contains(&"mydir".to_string()));
    assert!(dirs.contains(&"another_dir".to_string()));
    assert!(!dirs.contains(&"file1.txt".to_string()));
}

#[tokio::test]
async fn test_quoted_strings() {
    let quoted_files = vec![
        f("my file.txt", "content"),
        f("myfile.txt", "other"),
        f("test file 1.txt", "content"),
        f("test file 2.doc", "other"),
        f("config.json", "{}"),
        f("config.yaml", "test: true"),
    ];

    let cases = vec![
        (
            r#"name == "my file.txt""#, // Use filename for exact match
            &["my file.txt"][..],
            quoted_files.clone(),
        ),
        (
            r#"basename ~= "test file""#, // name matches without extension
            &["test file 1.txt", "test file 2.doc"][..],
            quoted_files.clone(),
        ),
        (
            r#"name == 'config.json'"#, // use filename for exact match
            &["config.json"][..],
            quoted_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_special_cases() {
    // Empty values and edge cases
    let empty_cases = vec![
        // Empty extension
        (
            r#"ext == """#,
            &["README", "Makefile", "noext"][..],
            vec![
                f("README", ""),
                f("Makefile", ""),
                f("noext", ""),
                f("file.txt", ""),
                f("script.rs", ""),
            ],
        ),
        // Empty parent (root files)
        (
            r#"dir == "" && type == "file""#,
            &["rootfile.txt"][..],
            vec![
                f("rootfile.txt", ""),
                f("dir/file.txt", ""),
                f("dir/subdir/file.txt", ""),
            ],
        ),
        // Empty content
        (
            r#"content == """#,
            &["empty.txt", "also_empty.rs"][..],
            vec![
                f("empty.txt", ""),
                f("also_empty.rs", ""),
                f("has_content.txt", "text"),
            ],
        ),
    ];

    run_test_cases(empty_cases).await;
}

#[tokio::test]
async fn test_complex_queries() {
    let complex_files = vec![
        f("src/main.rs", "fn main() { todo!() }"),
        f("src/lib.rs", "pub mod tests { }"),
        f("tests/integration.rs", "// TODO: write tests"),
        f("tests/unit.rs", "// Done"),
        f("docs/README.md", "# TODO: Documentation"),
        f("Cargo.toml", "[package]"),
        f(".gitignore", "target/"),
    ];

    let cases = vec![
        // Complex path and content
        (
            r#"path contains "src" && content contains "todo""#,
            &["src/main.rs"][..],
            complex_files.clone(),
        ),
        // Multiple conditions with negation
        (
            r#"ext == "rs" && !dir contains "test" && content ~= "fn|pub""#,
            &["src/main.rs", "src/lib.rs"][..],
            complex_files.clone(),
        ),
        // Set membership with other conditions
        (
            r#"ext in [rs, md] && (content contains "TODO" || content contains "todo")"#,
            &["src/main.rs", "tests/integration.rs", "docs/README.md"][..],
            complex_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_symlinks() {
    let tmp_dir = tempfile::Builder::new()
        .prefix("detect-symlinks")
        .tempdir()
        .unwrap();

    // Create target files
    let target = tmp_dir.path().join("target.txt");
    std::fs::write(&target, "target content").unwrap();

    // Create symlinks
    #[cfg(unix)]
    {
        let link1 = tmp_dir.path().join("link_to_target.txt");
        let link2 = tmp_dir.path().join("short");
        std::os::unix::fs::symlink(&target, &link1).unwrap();
        std::os::unix::fs::symlink(&target, &link2).unwrap();
    }

    let mut found = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        r#"basename ~= link"#.to_owned(),
        detect::RuntimeConfig::default(),
        |p| found.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    assert!(found.contains(&"link_to_target.txt".to_string()));
    assert!(!found.contains(&"short".to_string()));
    assert!(!found.contains(&"target.txt".to_string()));
}

#[tokio::test]
async fn test_alias_smoke_test() {
    // Simple smoke test to verify selector aliases work with real filesystem
    let cases = vec![
        // Test filename alias
        (
            "filename == test.rs",
            &["test.rs"][..],
            vec![f("test.rs", "fn main() {}"), f("lib.rs", "")],
        ),
        // Test extension alias
        (
            "extension == md",
            &["README.md"][..],
            vec![f("README.md", "# Test"), f("main.rs", "")],
        ),
        // Test filesize alias
        (
            "filesize > 10",
            &["large.txt"][..],
            vec![f("small.txt", "x"), f("large.txt", "xxxxxxxxxxxxxxxxxxxx")],
        ),
        // Test filetype alias
        (
            "filetype == file",
            &["a.txt", "b.txt"][..],
            vec![f("a.txt", ""), f("b.txt", "")],
        ),
        // Test mtime alias (all files are recently created)
        (
            "mtime > -1h",
            &["recent.txt"][..],
            vec![f("recent.txt", "new")],
        ),
        // Test contents alias
        (
            "contents contains TODO",
            &["todo.txt"][..],
            vec![f("todo.txt", "TODO: fix this"), f("done.txt", "all done")],
        ),
        // Test text alias
        (
            "text ~= FIXME",
            &["broken.rs"][..],
            vec![
                f("broken.rs", "// FIXME: bug here"),
                f("working.rs", "// works"),
            ],
        ),
        // Test stem alias
        (
            "stem == config",
            &["config.json", "config.toml"][..],
            vec![
                f("config.json", "{}"),
                f("config.toml", ""),
                f("other.txt", ""),
            ],
        ),
        // Test directory alias
        (
            "directory contains src",
            &["src/main.rs"][..],
            vec![f("src/main.rs", ""), f("test.rs", "")],
        ),
        // Test combined aliases in complex query
        (
            "filename ~= test AND extension == rs AND filesize > 0",
            &["test_one.rs", "test_two.rs"][..],
            vec![
                f("test_one.rs", "x"),
                f("test_two.rs", "x"),
                f("test.txt", "x"),
                f("main.rs", "x"),
            ],
        ),
    ];

    run_test_cases(cases).await;
}
// REGEX ENGINE TESTS - Isolate broken regex features
// These tests verify end-to-end regex compilation and matching
// Tests currently failing indicate issues in the regex engine layer

#[tokio::test]
async fn test_regex_escaped_parentheses() {
    // CRITICAL: Escaped parentheses should match literal parens in content
    // Current status: BROKEN - "missing closing parenthesis" error
    let test_files = vec![
        f("func.rs", "fn main() {"),
        f("call.rs", "foo(bar)"),
        f("generic.rs", "Vec<T>(item)"),
        f("no_parens.txt", "no parentheses here"),
    ];

    let cases = vec![
        // Basic escaped open paren
        (
            r"content ~= \(",
            &["func.rs", "call.rs", "generic.rs"][..],
            test_files.clone(),
        ),
        // Basic escaped close paren
        (
            r"content ~= \)",
            &["func.rs", "call.rs", "generic.rs"][..],
            test_files.clone(),
        ),
        // Both parens
        (r"content ~= \(\)", &["func.rs"][..], test_files.clone()),
        // Function pattern: fn followed by parens
        (
            r"content ~= fn\s+\w+\(",
            &["func.rs"][..],
            test_files.clone(),
        ),
        // Workaround with character class (should also work)
        (
            r"content ~= [(]",
            &["func.rs", "call.rs", "generic.rs"][..],
            test_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_regex_word_boundaries() {
    // CRITICAL: Word boundary anchors should isolate whole words
    // Current status: BROKEN - returns 0 matches
    let test_files = vec![
        f("use.rs", "use std::fs;"),
        f("reuse.rs", "reuse this code"),
        f("unused.rs", "unused variable"),
        f("user.txt", "username field"),
    ];

    let cases = vec![
        // Exact word "use" with boundaries
        (r"content ~= \buse\b", &["use.rs"][..], test_files.clone()),
        // Word boundary + pattern
        (
            r"content ~= \b\w+\b",
            &["use.rs", "reuse.rs", "unused.rs", "user.txt"][..],
            test_files.clone(),
        ),
        // Negated word boundary
        (
            r"content ~= \Buse",
            &["reuse.rs", "unused.rs"][..],
            test_files.clone(),
        ),
        // Function name pattern
        (
            r"content ~= \bfn\b",
            &["func.rs"][..],
            vec![
                f("func.rs", "fn main() {}"),
                f("confn.rs", "confusing name"),
            ],
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_regex_shorthand_class_quantifiers() {
    // CRITICAL: Curly brace quantifiers on \d, \w, \s should work
    // Current status: BROKEN - returns 0 matches
    // Note: [a-z]{3} works, but \w{3} doesn't
    let test_files = vec![
        f("port.txt", "localhost:8080"),
        f("var.rs", "let foo_bar = 42;"),
        f("spaces.txt", "a    b"),
        f("short.txt", "ab"),
    ];

    let cases = vec![
        // Exact digit count
        (r"content ~= \d{4}", &["port.txt"][..], test_files.clone()),
        // Digit range quantifier
        (
            r"content ~= \d{2,4}",
            &["port.txt", "var.rs"][..],
            test_files.clone(),
        ),
        // Word character quantifier
        (
            r"content ~= \w{3}",
            &["port.txt", "var.rs"][..],
            test_files.clone(),
        ),
        // Open-ended quantifier
        (
            r"content ~= \w{3,}",
            &["port.txt", "var.rs"][..], // spaces.txt has only single letters
            test_files.clone(),
        ),
        // Whitespace quantifier
        (
            r"content ~= \s{2,}",
            &["spaces.txt"][..],
            test_files.clone(),
        ),
        // Complex IP-like pattern
        (
            r"content ~= \d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}",
            &["ip.txt"][..],
            vec![f("ip.txt", "192.168.1.1"), f("port.txt", "localhost:8080")],
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_regex_unicode_properties() {
    // CRITICAL: Unicode property escapes should match character classes
    // Current status: BROKEN - returns 0 matches
    let test_files = vec![
        f("ascii.txt", "Hello123"),
        f("unicode.txt", "Привет мир"),
        f("mixed.txt", "Hello мир 123"),
        f("digits.txt", "12345"),
    ];

    let cases = vec![
        // Unicode letters
        (
            r"content ~= \p{L}+",
            &["ascii.txt", "unicode.txt", "mixed.txt"][..],
            test_files.clone(),
        ),
        // Unicode digits
        (
            r"content ~= \p{Nd}+",
            &["ascii.txt", "mixed.txt", "digits.txt"][..],
            test_files.clone(),
        ),
        // General category
        (
            r"content ~= \p{Letter}+",
            &["ascii.txt", "unicode.txt", "mixed.txt"][..],
            test_files.clone(),
        ),
        // Negated property
        (
            r"content ~= \P{Nd}+",
            &["ascii.txt", "unicode.txt", "mixed.txt"][..],
            test_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_regex_hex_octal_escapes() {
    // CRITICAL: Hex and octal character codes should match
    // Current status: BROKEN - returns 0 matches
    let test_files = vec![
        f("letter_a.txt", "ABC"),
        f("space.txt", "a b"),
        f("newline.txt", "line1\nline2"),
        f("other.txt", "xyz"),
    ];

    let cases = vec![
        // Hex escape (2 digits)
        (
            r"content ~= \x41", // Matches 'A'
            &["letter_a.txt"][..],
            test_files.clone(),
        ),
        // Hex escape (braces)
        (
            r"content ~= \x{41}", // Matches 'A'
            &["letter_a.txt"][..],
            test_files.clone(),
        ),
        // Hex space
        (
            r"content ~= \x20", // Matches space
            &["space.txt"][..],
            test_files.clone(),
        ),
        // Unicode escape (4 digits)
        (
            r"content ~= \u0041", // Matches 'A'
            &["letter_a.txt"][..],
            test_files.clone(),
        ),
        // Octal escape
        (
            r"content ~= \101", // Matches 'A'
            &["letter_a.txt"][..],
            test_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_regex_special_escapes() {
    // CRITICAL: Special regex escapes should function correctly
    // Current status: BROKEN - returns 0 matches
    let test_files = vec![
        f("start.txt", "use std::fs;"),
        f("middle.txt", "// use this"),
        f("literal.txt", "a.b.c"),
        f("regex.txt", "a*b+c?"),
    ];

    let cases = vec![
        // Absolute string start
        (r"content ~= \Ause", &["start.txt"][..], test_files.clone()),
        // Absolute string end
        (r"content ~= ;\z", &["start.txt"][..], test_files.clone()),
        // Literal quoting
        (
            r"content ~= \Qa.b\E",
            &["literal.txt"][..],
            test_files.clone(),
        ),
        // Literal quoting with regex metacharacters
        (
            r"content ~= \Qa*b+c?\E",
            &["regex.txt"][..],
            test_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_depth_predicate() {
    // Search root is depth 0, files in root are depth 1, etc.
    let depth_files = vec![
        f("level1.txt", "level 1"),
        f("dir1/level2.txt", "level 2"),
        f("dir1/sub/level3.txt", "level 3"),
        f("dir1/sub/deep/level4.txt", "level 4"),
        f("dir2/another2.txt", "depth 2"),
        f("dir2/sub/another3.txt", "depth 3"),
    ];

    let cases = vec![
        (
            "depth == 1 AND ext == txt",
            &["level1.txt"][..],
            depth_files.clone(),
        ),
        (
            "depth == 2 AND ext == txt",
            &["dir1/level2.txt", "dir2/another2.txt"][..],
            depth_files.clone(),
        ),
        (
            "depth == 3 AND ext == txt",
            &["dir1/sub/level3.txt", "dir2/sub/another3.txt"][..],
            depth_files.clone(),
        ),
        (
            "depth <= 2 AND ext == txt",
            &["level1.txt", "dir1/level2.txt", "dir2/another2.txt"][..],
            depth_files.clone(),
        ),
        (
            "depth > 1 AND ext == txt",
            &[
                "dir1/level2.txt",
                "dir1/sub/level3.txt",
                "dir1/sub/deep/level4.txt",
                "dir2/another2.txt",
                "dir2/sub/another3.txt",
            ][..],
            depth_files.clone(),
        ),
        (
            "depth >= 3 AND ext == txt",
            &[
                "dir1/sub/level3.txt",
                "dir1/sub/deep/level4.txt",
                "dir2/sub/another3.txt",
            ][..],
            depth_files.clone(),
        ),
        (
            "depth > 3 AND ext == txt",
            &["dir1/sub/deep/level4.txt"][..],
            depth_files.clone(),
        ),
        (
            "depth == 3 AND content contains level",
            &["dir1/sub/level3.txt"][..],
            depth_files.clone(),
        ),
        (
            "NOT depth > 3 AND ext == txt",
            &[
                "level1.txt",
                "dir1/level2.txt",
                "dir1/sub/level3.txt",
                "dir2/another2.txt",
                "dir2/sub/another3.txt",
            ][..],
            depth_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_multi_dot_extensions() {
    let multi_dot_files = vec![
        f("archive.tar.gz", "compressed"),
        f("backup.2024.tar", "backup data"),
        f("config.local.json", "{}"),
        f(".gitignore", "node_modules/"),
        f(".env.production", "API_KEY=secret"),
        f("file.backup.old.txt", "old backup"),
        f("simple.txt", "simple file"),
        f("nodot", "no extension"),
        f(".hidden", "hidden file"),
    ];

    let cases = vec![
        (
            "ext == gz",
            &["archive.tar.gz"][..],
            multi_dot_files.clone(),
        ),
        (
            "ext == tar",
            &["backup.2024.tar"][..],
            multi_dot_files.clone(),
        ),
        (
            "ext == json",
            &["config.local.json"][..],
            multi_dot_files.clone(),
        ),
        (
            "ext == txt",
            &["file.backup.old.txt", "simple.txt"][..],
            multi_dot_files.clone(),
        ),
        (
            r#"ext == """#,
            &[".gitignore", ".hidden", "nodot"][..],
            multi_dot_files.clone(),
        ),
        (
            "ext == production",
            &[".env.production"][..],
            multi_dot_files.clone(),
        ),
        (
            "name == archive.tar.gz",
            &["archive.tar.gz"][..],
            multi_dot_files.clone(),
        ),
        (
            "name == .gitignore",
            &[".gitignore"][..],
            multi_dot_files.clone(),
        ),
        (
            "basename == archive.tar",
            &["archive.tar.gz"][..],
            multi_dot_files.clone(),
        ),
        (
            "basename == file.backup.old",
            &["file.backup.old.txt"][..],
            multi_dot_files.clone(),
        ),
        (
            "basename == .gitignore",
            &[".gitignore"][..],
            multi_dot_files.clone(),
        ),
        (
            "name contains .tar",
            &["archive.tar.gz", "backup.2024.tar"][..],
            multi_dot_files.clone(),
        ),
        (
            r#"name ~= "\.tar\.(gz|bz2|xz)$""#,
            &["archive.tar.gz"][..],
            multi_dot_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_binary_and_non_utf8_content() {
    let tmp_dir = tempfile::Builder::new()
        .prefix("detect-binary")
        .tempdir()
        .unwrap();

    let text_file = tmp_dir.path().join("text.txt");
    std::fs::write(&text_file, "Hello world").unwrap();

    let binary_file = tmp_dir.path().join("binary.dat");
    std::fs::write(
        &binary_file,
        [0x00, 0x01, 0x02, 0xFF, 0xFE, b'A', b'B', 0x00],
    )
    .unwrap();

    let invalid_utf8 = tmp_dir.path().join("invalid.txt");
    std::fs::write(&invalid_utf8, [0xFF, 0xFE, 0xFD]).unwrap();

    let mixed_file = tmp_dir.path().join("mixed.dat");
    let mut mixed_data = b"Start ".to_vec();
    mixed_data.extend_from_slice(&[0x00, 0xFF, 0x00]);
    mixed_data.extend_from_slice(b" End");
    std::fs::write(&mixed_file, &mixed_data).unwrap();

    // Content search should not crash on binary files, only finds text
    let mut found = Vec::new();
    detect::parse_and_run_fs(
        test_logger(),
        tmp_dir.path(),
        false,
        "content contains Hello".to_owned(),
        detect::RuntimeConfig::default(),
        |p| found.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    found.sort();
    assert_eq!(found, vec!["text.txt"]);

    // Regex search should not crash on binary files, only finds text
    let mut found = Vec::new();
    detect::parse_and_run_fs(
        test_logger(),
        tmp_dir.path(),
        false,
        r#"content ~= "Hello.*world""#.to_owned(),
        detect::RuntimeConfig::default(),
        |p| found.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    found.sort();
    assert_eq!(found, vec!["text.txt"]);

    // Size-based queries work fine on all file types
    let mut found = Vec::new();
    detect::parse_and_run_fs(
        test_logger(),
        tmp_dir.path(),
        false,
        "size > 0 AND type == file".to_owned(),
        detect::RuntimeConfig::default(),
        |p| found.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    found.sort();
    assert_eq!(
        found,
        vec!["binary.dat", "invalid.txt", "mixed.dat", "text.txt"]
    );

    // Name-based queries work on binary files
    let mut found = Vec::new();
    detect::parse_and_run_fs(
        test_logger(),
        tmp_dir.path(),
        false,
        "ext == dat".to_owned(),
        detect::RuntimeConfig::default(),
        |p| found.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    found.sort();
    assert_eq!(found, vec!["binary.dat", "mixed.dat"]);
}

#[tokio::test]
async fn test_regex_pcre2_fallback() {
    let pcre2_test_files = vec![
        f("test1.txt", "foo bar baz"),
        f("test2.txt", "foobar"),
        f("test3.txt", "bar foo"),
        f("test4.txt", "function query() { }"),
        f("test5.txt", "SELECT * FROM table"),
    ];

    let cases = vec![
        // Lookbehind (PCRE2 fallback required)
        (
            r"content ~= (?<=foo)bar",
            &["test2.txt"][..],
            pcre2_test_files.clone(),
        ),
        // Lookahead (supported in both)
        (
            r"content ~= foo(?=bar)",
            &["test2.txt"][..],
            pcre2_test_files.clone(),
        ),
        // Escaped parens
        (
            r"content ~= query\(\)",
            &["test4.txt"][..],
            pcre2_test_files.clone(),
        ),
        // Basic patterns (no fallback needed)
        (
            r"content ~= foo.*bar",
            &["test1.txt", "test2.txt"][..],
            pcre2_test_files.clone(),
        ),
        // Word boundaries
        (
            r"content ~= \bbar\b",
            &["test1.txt", "test3.txt"][..],
            pcre2_test_files.clone(),
        ),
        // Case-insensitive flag
        (
            r"content ~= (?i)SELECT",
            &["test5.txt"][..],
            pcre2_test_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}
