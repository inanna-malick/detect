#![feature(box_patterns)]

mod eval;
mod expr;
mod matcher;
mod operator;
mod parser;
mod util;

use crate::eval::eval;

use combine::{stream::position, Parser};
pub fn parse_and_run(s: String) -> Result<(), anyhow::Error> {
    use walkdir::WalkDir;

    let e = parser::or()
        .parse(position::Stream::new(&s[..]))
        .expect("parse fail")
        .0;

    println!("running with expression: {:?}", e);

    let walker = WalkDir::new(".").into_iter();
    for entry in walker {
        let entry = entry?;
        if !entry.metadata()?.is_dir() {
            let path = format!("{:?}", entry.path());
            if eval(&e, entry)? {
                println!("{}", path);
            }
        }
    }

    Ok(())
}
