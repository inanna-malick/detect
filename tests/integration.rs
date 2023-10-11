use std::{env::set_current_dir, fs::create_dir_all};

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
        let tmp_path = tmp_dir.path().to_str().unwrap();
        let mut out = Vec::new();
        set_current_dir(tmp_path).unwrap();
        detect::parse_and_run(tmp_path.to_owned(), self.expr.to_owned(), |p| {
            let s = p
                .strip_prefix(&format!("{tmp_path}/"))
                .unwrap()
                .as_os_str()
                .to_string_lossy()
                .into_owned();
            out.push(s)
        })
        .await
        .unwrap();
        assert_eq!(self.expected, out)
    }
}


#[tokio::test]
async fn test_foo() {
    Case {
        expr: "filename(foo)",
        // we get the dir z/foo but not the file z/foo/bar,
        // so it really is just operating on filenames - nice
        expected: &["foo", "z/foo", "bar/foo"],
        files: vec![f("foo", "foo"), f("bar/foo", "baz"), f("bar/baz", "foo"), f("z/foo/bar", "")],
    }
    .run().await
}

#[tokio::test]
async fn test_not_foo() {
    Case {
        expr: "!filename(foo)",
        // note: weird inclusion of "" (empty str) in the results
        expected: &["bar/baz", "bar/baz"],
        files: vec![f("foo", "foo"), f("bar/foo", "baz"), f("bar/baz", "foo")],
    }
    .run().await
}

#[tokio::test]
async fn test_name_and_contents() {
    Case {
        expr: "filename(foo) && contains(foo)",
        expected: &["foo"],
        files: vec![f("foo", "foo"), f("bar/foo", "baz"), f("bar/baz", "foo")],
    }
    .run().await
}

#[tokio::test]
async fn test_name_and_async_program_invocation() {
    Case {
        expr: "filename(foo) && process(cat, foo)", // very simple case, equivalent to just checking file contents tbh
        expected: &["foo"],
        files: vec![f("foo", "foo"), f("bar/foo", "baz"), f("bar/baz", "foo")],
    }
    .run().await
}


#[tokio::test]
async fn test_extension_and_contents() {
    Case {
        expr: "extension(.rs)",
        expected: &["test.rs"],
        files: vec![f("test.rs", ""), f("test2", "")],
    }
    .run().await
}

#[tokio::test]
async fn test_size() {
    Case {
        expr: "filename(foo) && size(0..5)",
        expected: &["foo"],
        files: vec![f("foo", "smol"), f("bar/foo", "more than five characters")],
    }
    .run().await
}

#[tokio::test]
async fn test_size_right() {
    Case {
        expr: "filename(foo) && size(..5)",
        expected: &["foo"],
        files: vec![f("foo", "smol"), f("bar/foo", "more than five characters")],
    }
    .run().await
}

#[tokio::test]
async fn test_size_left() {
    Case {
        expr: "filename(foo) && size(5..)",
        expected: &["bar/foo"],
        files: vec![f("foo", "smol"), f("bar/foo", "more than five characters")],
    }
    .run().await
}

#[tokio::test]
async fn test_size_kb() {
    let big_str = "x".repeat(1025);
    Case {
        expr: "filename(foo) && size(1kb..2kb)",
        expected: &["bar/foo"],
        files: vec![f("foo", "smol"), f("bar/foo", &big_str)],
    }
    .run().await
}
