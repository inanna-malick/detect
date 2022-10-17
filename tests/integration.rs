use std::{env::set_current_dir, fs::create_dir_all};

use tempdir::TempDir;

fn f(path: &'static str, contents: &'static str) -> TestFile {
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

struct TestFile {
    path: &'static str,
    name: &'static str,
    contents: &'static str,
}

struct Case {
    expr: &'static str,
    expected: &'static [&'static str],
    files: Vec<TestFile>,
}

impl Case {
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

    fn run(&self) {
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
        .unwrap();
        assert_eq!(self.expected, out)
    }
}

#[test]
fn test_name_and_contents() {
    Case {
        expr: "filename(foo) && contains(foo)",
        expected: &["foo"],
        files: vec![f("foo", "foo"), f("bar/foo", "baz"), f("bar/baz", "foo")],
    }
    .run()
}

#[test]
fn test_extension_and_contents() {
    Case {
        expr: "extension(.rs)",
        expected: &["test.rs"],
        files: vec![f("test.rs", ""), f("test2", "")],
    }
    .run()
}

#[test]
fn test_size() {
    Case {
        expr: "filename(foo) && size(0..5)",
        expected: &["foo"],
        files: vec![f("foo", "smol"), f("bar/foo", "more than five characters")],
    }
    .run()
}
