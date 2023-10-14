mod eval;
pub mod expr;
mod parser;
pub mod predicate;
mod util;

use std::path::Path;

use combine::stream::position::{self, SourcePosition};
use expr::Expr;
use tokio::sync::broadcast;

use crate::eval::eval;

pub fn parse<'a>(s: &'a str) -> Result<Expr, combine::easy::Errors<char, &'a str, SourcePosition>> {
    let (e, _source_position) =
        combine::EasyParser::easy_parse(&mut parser::or(), position::Stream::new(&s[..]))?;

    Ok(e)
}

pub(crate) struct SubpocessId(u64);

#[derive(Clone)]
pub(crate) struct Cancellation();

pub async fn parse_and_run<F: FnMut(&Path)>(
    root: String,
    s: String,
    mut on_match: F,
) -> Result<(), anyhow::Error> {
    use walkdir::WalkDir;

    let expr = parse(&s)?;

    let mut ctrl_c_stream =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;

    let (cancellation_sender, _) = broadcast::channel::<Cancellation>(16);
    let (subprocessid_sender, _) = broadcast::channel::<SubpocessId>(16);

    let main_loop = async {
        // NOTE: sadly this is all synchronous, async would be nicer but the async
        //       walkdir crate is not yet mature
        // println!("expr: {:?}", e);
        let walker = WalkDir::new(root).into_iter();
        for entry in walker {
            let entry = entry?;
            let path = entry.path();

            let is_match = eval(&expr, path).await?;

            if is_match {
                on_match(path);
            }
        }

        Ok(())
    };

    Ok(())
}
