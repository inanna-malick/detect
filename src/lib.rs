mod eval;
mod expr;
mod parser;
mod predicate;
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
            let walker = WalkDir::new(root).into_iter();
            for entry in walker {
                let entry = entry?;
                if eval(&e, entry.path())? {
                    // NOTE: need this for tests, but that's ok - can just make optional compilation flag as used by main, I think
                    // TODO: can have multi-crate repo, it's fine. probably for the best
                    // hacky, can panic sometimes if bad OsStr (FIXME, move conversion to on_match)
                    let path = entry.path().to_str().unwrap().to_owned();
                    on_match(path);
                }
            }

            Ok(())
        }
        Err(err) => panic!("parse error: {}", err),
    }
}
