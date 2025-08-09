use std::{env::set_current_dir, fs::create_dir_all};

use slog::{o, Discard, Logger};
use tempdir::TempDir;

// Test helper functions
fn test_logger() -> Logger {
    Logger::root(Discard, o!())
}

fn f<'a>(path: &'a str, contents: &'a str) -> TestFile<'a> {
    let (path, name) = if path.contains('/') {
        path.rsplit_once('/').unwrap()
    } else {
        ("", path)
    };

    TestFile {
        path,
        name,
        contents,
    }
}

#[derive(Clone)]
struct TestFile<'a> {
    path: &'a str,
    name: &'a str,
    contents: &'a str,
}

struct Case<'a> {
    expr: &'static str,
    expected: &'static [&'static str],
    files: Vec<TestFile<'a>>,
}

impl<'a> Case<'a> {
    fn build(&self) -> TempDir {
        let t = TempDir::new("fileset-expr").unwrap();
        let tmp_path = t.path().to_str().unwrap();
        for file in self.files.iter() {
            create_dir_all(format!("{}/{}", tmp_path, file.path)).unwrap();
            std::fs::write(
                format!("{}/{}/{}", tmp_path, file.path, file.name),
                file.contents,
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

        // TODO: ordering-agnostic comparison
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
            "path.name == foo",
            &["foo", "z/foo", "bar/foo"][..],
            basic_files.clone(),
        ),
        // Test path only
        (
            "path.full ~= bar",
            &["bar", "bar/baz", "bar/foo"][..],
            vec![f("foo", "foo"), f("bar/foo", "baz"), f("bar/baz", "foo")],
        ),
        // Test not name
        (
            "!path.name == foo",
            &["", "bar", "bar/baz"][..],
            vec![f("foo", "foo"), f("bar/foo", "baz"), f("bar/baz", "foo")],
        ),
        // Test name and contents
        (
            "path.name == foo && contents ~= foo",
            &["foo"][..],
            basic_files.clone(),
        ),
        // Test parent directory
        (
            "path.parent == bar",
            &["bar/foo", "bar/baz"][..],
            basic_files.clone(),
        ),
        // Test stem
        (
            "path.stem == README",
            &["README.md", "README"][..],
            vec![
                f("README.md", "# Hello"),
                f("readme.md", "# hello"),
                f("README", "text"),
            ],
        ),
        // Test full path
        (
            "path.full == bar/foo",
            &["bar/foo"][..],
            basic_files.clone(),
        ),
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
            "path.extension == rs",
            &["main.rs", "lib.rs", "test.rs"][..],
            code_files.clone(),
        ),
        (
            "path.extension != rs",
            &[
                "",
                "style.css",
                "app.js",
                "component.jsx",
                "README.md",
                "Makefile",
            ][..],
            code_files.clone(),
        ),
        (
            "path.extension == \"\"",
            &["Makefile"][..],
            code_files.clone(),
        ),
        (
            "extension in [js, jsx, ts, tsx]",
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
            &["", "large.txt", "exact.txt"][..],
            size_files.clone(),
        ),
        (
            "size <= 5",
            &["small.txt", "exact.txt", "empty.txt"][..],
            size_files.clone(),
        ),
        (
            "path.name == small.txt && size < 5",
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
            r#"contents contains "TODO""#,
            &["main.rs", "lib.rs", "readme.md"][..],
            content_files.clone(),
        ),
        (
            r#"contents ~= "TODO|FIXME""#,
            &["main.rs", "lib.rs", "readme.md"][..],
            content_files.clone(),
        ),
        (
            r#"contents ~= "^//.*TODO""#,
            &["main.rs"][..],
            content_files.clone(),
        ),
        (
            r#"path.extension == rs && contents contains "TODO""#,
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
            r#"path.extension == "rs" && path.name contains "test""#,
            &["test.rs", "test_utils.rs"][..],
            bool_files.clone(),
        ),
        // OR operation
        (
            r#"path.name == "main.rs" || path.name == "lib.rs""#,
            &["main.rs", "lib.rs"][..],
            bool_files.clone(),
        ),
        // NOT operation
        (
            r#"!(path.name contains "test")"#,
            &["", "main.rs", "lib.rs", "doc.txt"][..],
            bool_files.clone(),
        ),
        // Complex: (A || B) && !C
        (
            r#"(path.name contains "main" || path.name contains "lib") && !path.name contains "test""#,
            &["main.rs", "lib.rs"][..],
            bool_files.clone(),
        ),
        // Nested parentheses
        (
            r#"path.extension == "rs" && !(path.name == "test.rs" || path.name == "main.rs")"#,
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
            r#"path.name ~= "test_.*\.rs$""#,
            &["test_utils.rs", "test_integration.rs"][..],
            regex_files.clone(),
        ),
        (
            r#"path.full ~= "(^|.*/)?test/.*\.rs$""#,
            &["src/test/utils.rs", "lib/test/helpers.rs", "test/main.rs"][..],
            regex_files.clone(),
        ),
        (
            r#"path.name ~= "^test""#,
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
            "path.extension in [rs, js, ts]",
            &["main.rs", "lib.rs", "app.js"][..],
            set_files.clone(),
        ),
        (
            "path.name in [main.rs, app.js, index.html]",
            &["main.rs", "app.js", "index.html"][..],
            set_files.clone(),
        ),
        (
            r#"path.stem in ["main", "lib", "app"]"#,
            &["main.rs", "lib.rs", "app.js"][..],
            set_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_type_selectors() {
    // Type tests need special handling for directories
    let tmp_dir = TempDir::new("detect-type").unwrap();

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
            r#"path.name == "my file.txt""#,
            &["my file.txt"][..],
            quoted_files.clone(),
        ),
        (
            r#"path.name ~= "test file""#,
            &["test file 1.txt", "test file 2.doc"][..],
            quoted_files.clone(),
        ),
        (
            r#"path.name == 'config.json'"#,
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
            r#"path.extension == """#,
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
            r#"path.parent == "" && type == "file""#,
            &["rootfile.txt"][..],
            vec![
                f("rootfile.txt", ""),
                f("dir/file.txt", ""),
                f("dir/subdir/file.txt", ""),
            ],
        ),
        // Empty content
        (
            r#"contents == """#,
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
            r#"path.full contains "src" && contents contains "todo""#,
            &["src/main.rs"][..],
            complex_files.clone(),
        ),
        // Multiple conditions with negation
        (
            r#"path.extension == "rs" && !path.parent contains "test" && contents ~= "fn|pub""#,
            &["src/main.rs", "src/lib.rs"][..],
            complex_files.clone(),
        ),
        // Set membership with other conditions
        (
            r#"path.extension in [rs, md] && (contents contains "TODO" || contents contains "todo")"#,
            &["src/main.rs", "tests/integration.rs", "docs/README.md"][..],
            complex_files.clone(),
        ),
    ];

    run_test_cases(cases).await;
}

#[tokio::test]
async fn test_symlinks() {
    let tmp_dir = TempDir::new("detect-symlinks").unwrap();

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
        r#"path.name ~= link"#.to_owned(),
        |p| found.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    assert!(found.contains(&"link_to_target.txt".to_string()));
    assert!(!found.contains(&"short".to_string()));
    assert!(!found.contains(&"target.txt".to_string()));
}
