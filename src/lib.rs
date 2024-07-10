mod eval;
pub mod expr;
mod parser;
pub mod predicate;
mod util;

use std::{path::{Path, PathBuf}, sync::Arc};

use combine::stream::position::{self, SourcePosition};
use expr::{ContentPredicate, Expr, MetadataPredicate, NamePredicate};
use ignore::{WalkBuilder, WalkParallel, WalkState};
use predicate::Predicate;

use crate::eval::eval;

pub fn parse(
    s: &str,
) -> Result<
    Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>,
    combine::easy::Errors<char, &str, SourcePosition>,
> {
    let (e, _source_position) =
        combine::EasyParser::easy_parse(&mut parser::or(), position::Stream::new(s))?;

    Ok(e)
}

pub async fn parse_and_run<F: FnMut(&Path)> (
    root: PathBuf,
    respect_gitignore: bool,
    expr: String,
    mut on_match: F,
) -> Result<(), anyhow::Error> {
    match parse(&expr) {
        Ok(expr) => {
            let walker = WalkBuilder::new(root).git_ignore(respect_gitignore).build();


            // walker.run(|| Box::new(on_match));

            // TODO: debug loggin switch? tracing? something of that nature, yes
            // println!("expr: {:?}", e);
            let walker = walker.into_iter();
            for entry in walker {
                let entry = entry?;
                let path = entry.path();

                let is_match = eval(&expr, path).await?;

                if is_match {
                    on_match(path);
                }
            }

            Ok(())
        }
        Err(err) => panic!("parse error: {}", err),
    }
}
