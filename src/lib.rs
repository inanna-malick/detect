mod eval;
mod expr;
mod matcher;
mod operator;
mod parser;
mod util;

use crate::eval::eval;

use combine::{stream::position, Parser};
pub fn parse_and_run<F: FnMut(String)>(
    root: String,
    s: String,
    mut on_match: F,
) -> Result<(), anyhow::Error> {
    use walkdir::WalkDir;

    let e = parser::or()
        .parse(position::Stream::new(&s[..]))
        .expect("parse fail")
        .0;

    println!("running with expression: {:?}", e);

    let walker = WalkDir::new(root).into_iter();
    for entry in walker {
        let entry = entry?;
        if !entry.metadata()?.is_dir() {
            // hacky, will panic sometimes if bad OsStr (FIXME)
            let path = entry.path().to_str().unwrap().to_owned();
            if eval(&e, entry)? {
                on_match(path);
            }
        }
    }

    Ok(())
}
