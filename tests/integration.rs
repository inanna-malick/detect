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
        assert_eq!(expected, out)
    }
}

#[tokio::test]
async fn test_name_only() {
    Case {
        expr: "path.name == foo",
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
        expr: "path.full ~= bar",
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
        expr: "!path.name == foo",
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
        expr: "path.name == foo && contents ~= foo",
        expected: &["foo"],
        files: vec![f("foo", "foo"), f("bar/foo", "baz"), f("bar/baz", "foo")],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_extension() {
    Case {
        expr: "path.suffix == rs",
        expected: &["test.rs"],
        files: vec![f("test.rs", ""), f("test2", "")],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_size() {
    Case {
        expr: "path.name == foo && size  < 5",
        expected: &["foo"],
        files: vec![f("foo", "smol"), f("bar/foo", "more than five characters")],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_size_right() {
    Case {
        expr: "path.name == foo && size < 5",
        expected: &["foo"],
        files: vec![f("foo", "smol"), f("bar/foo", "more than five characters")],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_size_left() {
    Case {
        expr: "path.name == foo && size > 5",
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
//         expr: "path.name == foo && size(1kb..2kb)",
//         expected: &["bar/foo"],
//         files: vec![f("foo", "smol"), f("bar/foo", &big_str)],
//     }
//     .run()
//     .await
// }

#[tokio::test]
async fn test_quoted_strings() {
    Case {
        expr: r#"path.name == "my file.txt""#,
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
        expr: r#"path.name ~= "test file""#,
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
        expr: r"path.name == 'config.json'",
        expected: &["config.json"],
        files: vec![f("config.json", "{}"), f("config.yaml", "test: true")],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_backward_compatibility() {
    // Ensure bare tokens still work
    Case {
        expr: "path.name == README.md",
        expected: &["README.md"],
        files: vec![f("README.md", "# Hello"), f("readme.md", "# hello")],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_not_equal_operator() {
    Case {
        expr: "path.suffix != txt",
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
        expr: r#"contents contains "TODO""#,
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
async fn test_name_regex_patterns() {
    Case {
        expr: r#"path.name ~= "test_.*\.rs$""#,
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
async fn test_path_regex_patterns() {
    Case {
        expr: r#"path.full ~= "(^|.*/)?test/.*\.rs$""#,
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
        expr: r#"path.name =~ "^test_.*\.rs$""#,
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
        expr: r#"path.name =~ test_.*"#,
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
        expr: r#"path.suffix in [js, ts, jsx, tsx]"#,
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
        expr: r#"path.name in ["my file.txt", "another file.doc", config.json]"#,
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
    // Test that 'in' requires a set literal
    Case {
        expr: r#"path.suffix in ["js"]"#,
        expected: &["app.js", "index.js"],
        files: vec![f("app.js", ""), f("index.js", ""), f("style.css", "")],
    }
    .run()
    .await
}

// ===== Complex Grammar Interaction Tests =====

#[tokio::test]
async fn test_name_character_classes() {
    Case {
        expr: r#"path.name ~= "file[1-3]\.txt$""#,
        expected: &["file1.txt", "file2.txt", "file3.txt"],
        files: vec![
            f("file1.txt", ""),
            f("file2.txt", ""),
            f("file3.txt", ""),
            f("file4.txt", ""),
            f("file0.txt", ""),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_name_single_char_patterns() {
    Case {
        expr: r#"path.name ~= "file.\.txt$""#,
        expected: &["file1.txt", "file2.txt", "fileA.txt"],
        files: vec![
            f("file1.txt", ""),
            f("file2.txt", ""),
            f("fileA.txt", ""),
            f("file10.txt", ""), // Two characters, won't match
            f("file.txt", ""),   // Zero characters, won't match
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_contains_with_regex_special_chars() {
    Case {
        expr: r#"contents contains "function(""#,
        expected: &["main.js", "lib.js"],
        files: vec![
            f("main.js", "function() { return 42; }"),
            f("lib.js", "const fn = function( arg ) { }"),
            f("class.js", "class MyClass { }"),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_multiple_in_operators() {
    Case {
        expr: r#"path.suffix in [js, ts] && path.stem in [index, main]"#,
        expected: &["index.js", "main.js", "index.ts", "main.ts"],
        files: vec![
            f("index.js", ""),
            f("main.js", ""),
            f("index.ts", ""),
            f("main.ts", ""),
            f("app.js", ""),     // Wrong name
            f("index.html", ""), // Wrong extension
            f("main.css", ""),   // Wrong extension
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_complex_nested_expression() {
    Case {
        expr: r#"(path.suffix in [rs, toml] || path.suffix == md) && (size < 1000 || contents contains "important")"#,
        expected: &["small.rs", "README.md", "config.toml", "large.rs"],
        files: vec![
            f("small.rs", "fn main() {}"),  // Small .rs file
            f("README.md", "# Project"),     // .md file (matches first part)
            f("config.toml", "[package]"),   // Small .toml file
            f("large.rs", &"x".repeat(2000).replace("xxxxx", "important")), // Large but contains "important"
            f("large.txt", &"x".repeat(2000)), // Large .txt file (no match)
            f("small.txt", "hello"),         // Small .txt file (no match)
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_mixed_quotes_in_values() {
    Case {
        expr: r#"path.name == "file with 'quotes'.txt""#,
        expected: &["file with 'quotes'.txt"],
        files: vec![
            f("file with 'quotes'.txt", ""),
            f("file with quotes.txt", ""),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_negation_with_contains() {
    // Test for the negation bug reported by beta tester
    Case {
        expr: r#"path.suffix == "rs" && !(path.name contains "test")"#,
        expected: &["main.rs", "lib.rs", "mod.rs"],
        files: vec![
            f("main.rs", "fn main() {}"),
            f("lib.rs", "pub mod foo;"),
            f("mod.rs", "// module"),
            f("test.rs", "// test file"),
            f("test_utils.rs", "// test utilities"),
            f("integration_test.rs", "// integration tests"),
            f("doc.txt", "documentation"),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_negation_variants() {
    // Test different negation patterns to isolate the bug

    // Simple negation - should work
    Case {
        expr: r#"!(path.name contains "test")"#,
        // FIXME: why? re: Include root dir
        expected: &["", "main.rs", "lib.rs", "doc.txt"],
        files: vec![
            f("main.rs", ""),
            f("test.rs", ""),
            f("lib.rs", ""),
            f("test_lib.rs", ""),
            f("doc.txt", ""),
        ],
    }
    .run()
    .await;

    // Negation with equals - should work
    Case {
        expr: r#"path.suffix == "rs" && !(path.name == "test.rs")"#,
        expected: &["main.rs", "lib.rs"],
        files: vec![
            f("main.rs", ""),
            f("test.rs", ""),
            f("lib.rs", ""),
            f("doc.txt", ""),
        ],
    }
    .run()
    .await;

    // The problematic case - negation with contains in compound expr
    Case {
        expr: r#"path.suffix == "rs" && !(path.name contains "lib")"#,
        expected: &["main.rs", "test.rs"],
        files: vec![
            f("main.rs", ""),
            f("test.rs", ""),
            f("lib.rs", ""),
            f("mylib.rs", ""),
            f("doc.txt", ""),
        ],
    }
    .run()
    .await;
}

#[tokio::test]
async fn test_empty_extension_matching() {
    // Test finding files without extensions
    Case {
        expr: r#"path.extension == """#,
        expected: &["README", "Makefile", "noext"],
        files: vec![
            f("README", ""),
            f("Makefile", ""),
            f("noext", ""),
            f("file.txt", ""),
            f("script.rs", ""),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_empty_parent_matching() {
    // Test finding files in root (no parent directory)
    Case {
        expr: r#"path.parent == "" && type == "file""#,
        expected: &["rootfile.txt"],
        files: vec![
            f("rootfile.txt", ""),
            f("dir/file.txt", ""),
            f("dir/subdir/file.txt", ""),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_depth_selector() {
    // Test finding files at specific depth
    // Depth = number of path components from base directory
    // root.txt = 1, dir/level1.txt = 2, dir/subdir/level2.txt = 3, etc.
    Case {
        expr: r#"depth == 1 && type == file"#,
        expected: &["root.txt"],
        files: vec![
            f("root.txt", ""),
            f("dir/level1.txt", ""),
            f("dir/subdir/level2.txt", ""),
            f("dir/subdir/deep/level3.txt", ""),
        ],
    }
    .run()
    .await;

    Case {
        expr: r#"depth == 3 && type == file"#,
        expected: &["dir/subdir/level2.txt"],
        files: vec![
            f("root.txt", ""),
            f("dir/level1.txt", ""),
            f("dir/subdir/level2.txt", ""),
            f("dir/subdir/deep/level3.txt", ""),
        ],
    }
    .run()
    .await;

    Case {
        expr: r#"depth > 2 && type == file"#,
        expected: &["dir/subdir/level2.txt", "dir/subdir/deep/level3.txt"],
        files: vec![
            f("root.txt", ""),
            f("dir/level1.txt", ""),
            f("dir/subdir/level2.txt", ""),
            f("dir/subdir/deep/level3.txt", ""),
        ],
    }
    .run()
    .await;
}

#[tokio::test]
async fn test_parent_dir_selector() {
    // Test matching immediate parent directory name
    Case {
        expr: r#"path.parent_dir == utils"#,
        expected: &["src/utils/helper.rs", "lib/utils/math.rs"],
        files: vec![
            f("src/utils/helper.rs", ""),
            f("lib/utils/math.rs", ""),
            f("src/main.rs", ""),
            f("utils.rs", ""), // File named utils, not in utils directory
            f("tests/utils_test.rs", ""), // In tests dir, not utils
        ],
    }
    .run()
    .await;

    // Test parent_dir with 'in' operator
    Case {
        expr: r#"path.parent_dir in [src, lib, test] && type == file"#,
        expected: &["src/main.rs", "lib/util.rs", "test/runner.rs"],
        files: vec![
            f("src/main.rs", ""),
            f("lib/util.rs", ""),
            f("test/runner.rs", ""),
            f("src/utils/helper.rs", ""), // parent_dir is utils, not src
            f("main.rs", ""),             // No parent directory
        ],
    }
    .run()
    .await;
}

#[tokio::test]
async fn test_content_selector_forms() {
    // Test all content selector forms work
    Case {
        expr: r#"content.text contains "TODO""#,
        expected: &["todo.txt", "src/main.rs"],
        files: vec![
            f("todo.txt", "TODO: finish this"),
            f("src/main.rs", "// TODO: implement"),
            f("done.txt", "All done!"),
        ],
    }
    .run()
    .await;
    
    // Test bare text shorthand
    Case {
        expr: r#"text contains "FIXME""#,
        expected: &["broken.rs", "needs_work.py"],
        files: vec![
            f("broken.rs", "// FIXME: handle error"),
            f("needs_work.py", "# FIXME: optimize this"),
            f("working.rs", "// All good"),
        ],
    }
    .run()
    .await;
    
    // Test legacy contents form still works
    Case {
        expr: r#"contents contains "HACK""#,
        expected: &["workaround.js"],
        files: vec![
            f("workaround.js", "// HACK: temporary solution"),
            f("clean.js", "// Clean implementation"),
        ],
    }
    .run()
    .await;
}

#[tokio::test]
async fn test_escape_sequences_in_regex() {
    Case {
        expr: r#"contents ~= "\$\d+\.\d{2}""#, // Matches dollar amounts like $19.99
        expected: &["prices.txt", "invoice.txt"],
        files: vec![
            f("prices.txt", "Item costs $19.99"),
            f("invoice.txt", "Total: $1234.56"),
            f("notes.txt", "Price is around 20 dollars"),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_path_with_extension_pattern() {
    Case {
        expr: r#"path.full ~= ".*\.test\.js$""#,
        expected: &[
            "unit.test.js",
            "src/component.test.js",
            "src/utils/helper.test.js",
        ],
        files: vec![
            f("unit.test.js", ""),
            f("src/component.test.js", ""),
            f("src/utils/helper.test.js", ""),
            f("src/component.js", ""), // Not a test file
            f("test.js", ""),          // Missing .test pattern
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_complex_path_patterns() {
    Case {
        expr: r#"path.full ~= "src/.*/(test|spec)/.*\.js$""#,
        expected: &["src/components/test/button.js", "src/utils/spec/helper.js"],
        files: vec![
            f("src/components/test/button.js", ""),
            f("src/utils/spec/helper.js", ""),
            f("src/components/button.js", ""), // Not in test/spec dir
            f("test/components/button.js", ""), // test not under src
            f("src/test/button.ts", ""),       // Wrong extension
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_set_with_special_filenames() {
    Case {
        expr: r#"path.name in ["Makefile", "Dockerfile"]"#,
        expected: &["Makefile", "Dockerfile"],
        files: vec![f("Makefile", ""), f("Dockerfile", ""), f("README.md", "")],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_combining_all_new_operators() {
    // Use regex patterns, in, contains, and != all in one expression
    Case {
        expr: r#"path.name contains ".config." && path.suffix in [js, json, yaml, yml] && contents contains "version" && size != 0"#,
        expected: &["app.config.js", "db.config.json"],
        files: vec![
            f("app.config.js", "module.exports = { version: '1.0.0' }"),
            f("db.config.json", r#"{ "version": "2.0" }"#),
            f("test.config.yaml", "name: test"),  // No "version"
            f("empty.config.yml", ""),            // Size is 0
            f("config.txt", "version: 1.0"),      // Wrong extension
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_c_extension_patterns() {
    Case {
        expr: r#"path.name ~= ".*\.[ch]$""#, // C source and header files
        expected: &["main.c", "utils.c", "main.h", "utils.h"],
        files: vec![
            f("main.c", ""),
            f("utils.c", ""),
            f("main.h", ""),
            f("utils.h", ""),
            f("test.cpp", ""), // Not .c or .h
        ],
    }
    .run()
    .await
}

// ===== File Edge Cases Tests =====

#[tokio::test]
async fn test_files_without_extensions() {
    // Since we can't match empty extensions directly,
    // test by excluding files with known extensions
    Case {
        expr: r#"path.name in [README, Makefile, LICENSE] && !path.name contains ".""#,
        expected: &["README", "Makefile", "LICENSE"],
        files: vec![
            f("README", "# Project"),
            f("Makefile", "all:\n\tbuild"),
            f("LICENSE", "MIT"),
            f("main.rs", "fn main() {}"),
            f(".gitignore", "target/"),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_files_with_multiple_dots() {
    Case {
        expr: r#"path.suffix == gz"#,
        expected: &["archive.tar.gz", "backup.sql.gz"],
        files: vec![
            f("archive.tar.gz", ""),
            f("backup.sql.gz", ""),
            f("data.tar", ""),
            f("compressed.gz.txt", ""), // .txt is the extension
        ],
    }
    .run()
    .await
}

// TODO: hidden files ignored by default, add flag
// #[tokio::test]
// async fn test_hidden_files() {
//     Case {
//         expr: r#"path.name ~= "^\.""#,  // Names starting with dot
//         expected: &[".gitignore", ".env", ".bashrc", ".config.json"],
//         files: vec![
//             f(".gitignore", "node_modules/"),
//             f(".env", "API_KEY=secret"),
//             f(".bashrc", "export PATH"),
//             f(".config.json", "{}"),
//             f("visible.txt", "not hidden"),
//         ],
//     }
//     .run()
//     .await
// }

#[tokio::test]
async fn test_unicode_filenames() {
    Case {
        expr: r#"path.name ~= "[α-ω]""#, // Greek letters
        expected: &["αlpha.txt", "βeta.doc", "γamma.rs"],
        files: vec![
            f("αlpha.txt", "Greek alpha"),
            f("βeta.doc", "Greek beta"),
            f("γamma.rs", "Greek gamma"),
            f("regular.txt", "No Greek"),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_emoji_filenames() {
    Case {
        expr: r#"path.name contains "📄""#,
        expected: &["📄document.txt", "report📄.md"],
        files: vec![
            f("📄document.txt", "Document with emoji"),
            f("report📄.md", "Report with emoji"),
            f("regular.txt", "No emoji"),
            f("🎵music.mp3", "Different emoji"),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_directories_vs_files() {
    Case {
        expr: r#"type == dir && path.name ~= test"#,
        expected: &["test", "tests", "src/test"],
        files: vec![
            f("test/dummy.txt", ""),      // Creates test directory
            f("tests/unit.rs", ""),       // Creates tests directory
            f("src/test/lib.rs", ""),     // Creates src/test directory
            f("test.rs", "// Not a dir"), // File, not directory
            f("testing.txt", ""),         // File, not directory
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_case_sensitivity() {
    Case {
        expr: r#"path.name == README.md"#, // Exact case match
        expected: &["README.md"],
        files: vec![
            f("README.md", "# Title"),
            f("readme.md", "# title"),
            f("Readme.md", "# Title"),
            f("README.MD", "# TITLE"),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_files_with_spaces_in_names() {
    Case {
        expr: r#"path.name ~= " ""#, // Contains space
        expected: &["my file.txt", "another file.doc", "file with spaces.rs"],
        files: vec![
            f("my file.txt", ""),
            f("another file.doc", ""),
            f("file with spaces.rs", ""),
            f("no_spaces.txt", ""),
            f("also-no-spaces.doc", ""),
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_extension_edge_cases() {
    Case {
        expr: r#"path.suffix in [d, "2", "1"]"#,
        expected: &[".gitignore.d", "file.2", "archive.1"],
        files: vec![
            f(".gitignore.d", ""), // Extension is "d"
            f("file.2", ""),       // Numeric extension "2"
            f("archive.1", ""),    // Numeric extension "1"
            f("test.txt", ""),     // Regular extension
            f("no_ext", ""),       // No extension
        ],
    }
    .run()
    .await
}

#[tokio::test]
async fn test_symlink_names() {
    let tmp_dir = TempDir::new("detect-symlink-names").unwrap();

    // Create a target file
    let target = tmp_dir.path().join("target.txt");
    std::fs::write(&target, "target content").unwrap();

    // Create symlinks with various names
    let link1 = tmp_dir.path().join("link_to_target.txt");
    let link2 = tmp_dir.path().join("short");

    #[cfg(unix)]
    {
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
