mod eval;
mod expr;
mod matcher;
mod operator;
mod parser;
mod util;

use std::path::Path;

use crate::eval::eval;

use combine::{stream::position, Parser};
pub fn parse_and_run<F: FnMut(&Path)>(
    root: String,
    s: String,
    mut on_match: F,
) -> Result<(), anyhow::Error> {
    use walkdir::WalkDir;

    let expr_arena = bumpalo::Bump::new();
    let e = parser::or(&expr_arena)
        .parse(position::Stream::new(&s[..]))
        .expect("parse fail")
        .0;

    println!("running with expression: {:?}", e);

    let walker = WalkDir::new(root).into_iter();
    for entry in walker {
        let entry = entry?;
        if !entry.metadata()?.is_dir() {
            if eval(&e, &entry)? {
                on_match(entry.path());
            }
        }
    }

    Ok(())
}
