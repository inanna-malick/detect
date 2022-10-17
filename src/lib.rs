mod eval;
mod expr;
mod parser;
mod predicate;
mod util;

use std::path::Path;

use crate::eval::eval;
use combine::stream::position;

pub fn parse_and_run<F: FnMut(&Path)>(
    root: String,
    s: String,
    mut on_match: F,
) -> Result<(), anyhow::Error> {
    use walkdir::WalkDir;

    match combine::EasyParser::easy_parse(&mut parser::or(), position::Stream::new(&s[..])) {


        Ok((e, _)) => {
        // println!("expr: {}", e);
            let walker = WalkDir::new(root).into_iter();
            for entry in walker {
                let entry = entry?;
                let path = entry.path();
                if eval(&e, path)? {
                    on_match(path);
                }
            }

            Ok(())
        }
        Err(err) => panic!("parse error: {}", err),
    }
}
