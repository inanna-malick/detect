mod eval;
pub mod expr;
mod parser;
pub mod predicate;
mod util;

use std::path::Path;

use combine::stream::position::{self, SourcePosition};
use expr::{ContentPredicate, Expr, MetadataPredicate, NamePredicate};
use predicate::{Predicate, ProcessPredicate};

use crate::eval::eval;

pub fn parse<'a>(
    s: &'a str,
) -> Result<
    Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate, ProcessPredicate>>,
    combine::easy::Errors<char, &'a str, SourcePosition>,
> {
    let (e, _source_position) =
        combine::EasyParser::easy_parse(&mut parser::or(), position::Stream::new(&s[..]))?;

    Ok(e)
}

pub async fn parse_and_run<F: FnMut(&Path)>(
    root: String,
    s: String,
    mut on_match: F,
) -> Result<(), anyhow::Error> {
    use walkdir::WalkDir;

    // NOTE: top level should be and, I think - rain says that binds most tightly
    match parse(&s) {
        Ok(e) => {
            // TODO: debug loggin switch? tracing? idk hell yes
            // println!("expr: {:?}", e);
            let walker = WalkDir::new(root).into_iter();
            for entry in walker {
                let entry = entry?;
                let path = entry.path();

                let is_match = eval(&e, path).await?;

                if is_match {
                    on_match(path);
                }
            }

            Ok(())
        }
        Err(err) => panic!("parse error: {}", err),
    }
}
