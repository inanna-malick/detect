mod eval;
mod expr;
mod matcher;
mod operator;
mod parser;
mod util;

use crate::eval::eval;
use combine::stream::position;

pub fn parse_and_run<F: FnMut(String)>(
    root: String,
    s: String,
    mut on_match: F,
) -> Result<(), anyhow::Error> {
    use walkdir::WalkDir;

    match combine::EasyParser::easy_parse(&mut parser::or(), position::Stream::new(&s[..])) {
        Ok((e, _)) => {
            println!("running with expression: {:?}", e);

            let walker = WalkDir::new(root).into_iter();
            for entry in walker {
                let entry = entry?;
                if entry.path().to_str().unwrap().contains("test_output") {
                } else {
                    if !entry.metadata()?.is_dir() && eval(&e, entry.path())? {
                        // hacky, will panic sometimes if bad OsStr (FIXME)
                        let path = entry.path().to_str().unwrap().to_owned();
                        on_match(path);
                    }
                }
            }

            Ok(())
        }
        Err(err) => panic!("parse error: {}", err),
    }
}
