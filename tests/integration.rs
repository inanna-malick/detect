use std::{env::set_current_dir, fs::create_dir_all};

use slog::{o, Discard, Logger};
use tempdir::TempDir;

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
            Logger::root(Discard, o!()),
            tmp_dir.path(),
            false,
            self.expr.to_owned(),
            |p| {
                let s = p
                    .strip_prefix(&format!("{}/", tmp_dir.path().to_str().unwrap()))
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
        assert_eq!(expected, out)
    }
}

#[tokio::test]
async fn test_name_only() {
    Case {
        expr: "@name == foo",
        // we get the dir z/foo but not the file z/foo/bar,
        // proving that it really is just operating on names - nice
        expected: &["foo", "z/foo", "bar/foo"],
        files: vec![
            f("foo", "foo"),
            f("bar/foo", "baz"),
            f("bar/baz", "foo"),
            f("z/foo/bar", ""),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_path_only() {
    Case {
        expr: "@path ~= bar",
        // we get the dir z/foo but not the file z/foo/bar,
        // so it really is just operating on names - nice
        expected: &["bar", "bar/baz", "bar/foo"],
        files: vec![f("foo", "foo"), f("bar/foo", "baz"), f("bar/baz", "foo")],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_not_name_only() {
    Case {
        expr: "!@name == foo",
        // TODO: figure out if I want to filter out empty paths here I guess? currently they're included
        expected: &["", "bar", "bar/baz"],
        files: vec![f("foo", "foo"), f("bar/foo", "baz"), f("bar/baz", "foo")],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_name_and_contents() {
    Case {
        expr: "@name == foo && @contents ~= foo",
        expected: &["foo"],
        files: vec![f("foo", "foo"), f("bar/foo", "baz"), f("bar/baz", "foo")],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_extension() {
    Case {
        expr: "@extension == rs",
        expected: &["test.rs"],
        files: vec![f("test.rs", ""), f("test2", "")],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_size() {
    Case {
        expr: "@name == foo && @size  < 5",
        expected: &["foo"],
        files: vec![f("foo", "smol"), f("bar/foo", "more than five characters")],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_size_right() {
    Case {
        expr: "@name == foo && @size < 5",
        expected: &["foo"],
        files: vec![f("foo", "smol"), f("bar/foo", "more than five characters")],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_size_left() {
    Case {
        expr: "@name == foo && @size > 5",
        expected: &["bar/foo"],
        files: vec![f("foo", "smol"), f("bar/foo", "more than five characters")],
    }
    .run()
    .await
}

// #[tokio::test]
// async fn test_size_kb() {
//     let big_str = "x".repeat(1025);
//     Case {
//         expr: "name == foo && size(1kb..2kb)",
//         expected: &["bar/foo"],
//         files: vec![f("foo", "smol"), f("bar/foo", &big_str)],
//     }
//     .run()
//     .await
// }

#[tokio::test]
async fn test_quoted_strings() {
    Case {
        expr: r#"@name == "my file.txt""#,
        expected: &["my file.txt"],
        files: vec![
            f("my file.txt", "content"),
            f("myfile.txt", "other"),
            f("other.txt", "test"),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_quoted_strings_with_spaces() {
    Case {
        expr: r#"@name ~= "test file""#,
        expected: &["test file 1.txt", "test file 2.doc"],
        files: vec![
            f("test file 1.txt", "content"),
            f("test file 2.doc", "other"),
            f("testfile3.txt", "no match"),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_single_quotes() {
    Case {
        expr: r"@name == 'config.json'",
        expected: &["config.json"],
        files: vec![
            f("config.json", "{}"),
            f("config.yaml", "test: true"),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_backward_compatibility() {
    // Ensure bare tokens still work
    Case {
        expr: "@name == README.md",
        expected: &["README.md"],
        files: vec![
            f("README.md", "# Hello"),
            f("readme.md", "# hello"),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_not_equal_operator() {
    Case {
        expr: "@ext != txt",
        expected: &["script.sh", "config.json", ""],
        files: vec![
            f("readme.txt", "text"),
            f("script.sh", "#!/bin/bash"),
            f("config.json", "{}"),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_contains_operator() {
    Case {
        expr: r#"@contents contains "TODO""#,
        expected: &["main.rs", "lib.rs"],
        files: vec![
            f("main.rs", "// TODO: implement feature"),
            f("lib.rs", "/* TODO: add tests */"),
            f("done.rs", "// All tasks completed"),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_glob_operator() {
    Case {
        expr: r#"@name glob "test_*.rs""#,
        expected: &["test_utils.rs", "test_integration.rs"],
        files: vec![
            f("test_utils.rs", ""),
            f("test_integration.rs", ""),
            f("main_test.rs", ""),
            f("tests.rs", ""),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_glob_with_double_star() {
    Case {
        expr: r#"@path glob "**/test/*.rs""#,
        expected: &["src/test/utils.rs", "lib/test/helpers.rs", "test/main.rs"],
        files: vec![
            f("src/test/utils.rs", ""),
            f("lib/test/helpers.rs", ""),
            f("test/main.rs", ""),
            f("src/main.rs", ""),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_match_operator() {
    // Test the explicit =~ regex operator with quoted regex
    Case {
        expr: r#"@name =~ "^test_.*\.rs$""#,
        expected: &["test_utils.rs", "test_integration.rs"],
        files: vec![
            f("test_utils.rs", ""),
            f("test_integration.rs", ""),
            f("main_test.rs", ""),
            f("tests.rs", ""),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_match_operator_bare() {
    // Test the =~ operator with simple pattern
    Case {
        expr: r#"@name =~ test_.*"#,
        expected: &["test_utils.rs", "test_integration.rs"],
        files: vec![
            f("test_utils.rs", ""),
            f("test_integration.rs", ""),
            f("main_test.rs", ""),
            f("tests.rs", ""),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_in_operator_with_set() {
    Case {
        expr: r#"@ext in [js, ts, jsx, tsx]"#,
        expected: &["app.js", "lib.ts", "component.jsx", "page.tsx"],
        files: vec![
            f("app.js", ""),
            f("lib.ts", ""),
            f("component.jsx", ""),
            f("page.tsx", ""),
            f("style.css", ""),
            f("index.html", ""),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_in_operator_with_quoted_set() {
    Case {
        expr: r#"@name in ["my file.txt", "another file.doc", config.json]"#,
        expected: &["my file.txt", "another file.doc", "config.json"],
        files: vec![
            f("my file.txt", ""),
            f("another file.doc", ""),
            f("config.json", ""),
            f("readme.md", ""),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_in_operator_single_value() {
    // Test that 'in' works with a single value (not a set)
    Case {
        expr: r#"@ext in "js""#,
        expected: &["app.js", "index.js"],
        files: vec![
            f("app.js", ""),
            f("index.js", ""),
            f("style.css", ""),
        ],
    }
    .run()
    .await
}