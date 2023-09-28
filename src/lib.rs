mod eval;
mod expr;
mod parser;
mod predicate;
mod util;

#[cfg(feature = "viz")]
mod eval_viz;

use std::path::Path;

use combine::stream::position;


pub async fn parse_and_run<F: FnMut(&Path)>(
    root: String,
    s: String,
    viz_output_dir: Option<String>,
    mut on_match: F,
) -> Result<(), anyhow::Error> {
    use walkdir::WalkDir;

    match combine::EasyParser::easy_parse(&mut parser::or(), position::Stream::new(&s[..])) {
        Ok((e, _)) => {
            let walker = WalkDir::new(root).into_iter();
            for entry in walker {
                let entry = entry?;
                let path = entry.path();

                let is_match = if let Some(_viz_output_dir) = &viz_output_dir {
                    #[cfg(not(feature = "viz"))]
                    unimplemented!("todo: organize code better!");

                    #[cfg(feature = "viz")]
                    {
                    let (is_match, viz) = eval_viz::eval_v(&e, path)? ;

                    let out_dir = format!("{}/expr_for_{}", _viz_output_dir, path.display());
                    // println!("create dir: {}", out_dir);
                    std::fs::create_dir(&out_dir)?;

                    viz.write_viz(out_dir);

                    is_match
                    }
                } else {
                    eval::eval(&e, path).await?
                };

                if is_match {
                    on_match(path);
                }
            }

            Ok(())
        }
        Err(err) => panic!("parse error: {}", err),
    }
}
