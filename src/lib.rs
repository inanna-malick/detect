mod eval;
mod expr;
mod parser;
mod predicate;
mod util;
mod eval_viz;

use std::path::Path;

use combine::stream::position;

use crate::eval_viz::eval_v;

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
                // TODO: integrate via switch (mb with compile flag?)
                let (is_match, viz) = eval_v(&e, path)? ;

                let out_dir = format!("/home/inanna/viz_output_tmp/expr_for_{}", path.display());
                println!("create dir: {}", out_dir);
                std::fs::create_dir(&out_dir)?;

                viz.do_thing(out_dir);

                if is_match {
                    on_match(path);
                }
            }

            Ok(())
        }
        Err(err) => panic!("parse error: {}", err),
    }
}
