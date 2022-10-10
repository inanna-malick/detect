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

    let e: Result<
        (
            expr::ExprTree,
            position::Stream<&str, position::SourcePosition>,
        ),
        combine::easy::Errors<_, _, _>,
    > = combine::EasyParser::easy_parse(&mut parser::or(), position::Stream::new(&s[..]));

    let e = match e {
        Ok(e) => e.0,
        Err(err) => panic!("parse error: {}", err),
    };

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
