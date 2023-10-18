use recursion_visualize::visualize::Viz;

use crate::expr::Expr;
use crate::expr::{ContentPredicate, MetadataPredicate, NamePredicate};
use crate::util::Done;
use std::fs::{self};
use std::path::Path;

/// multipass evaluation with short circuiting, runs, in order:
/// - file name matchers
/// - metadata matchers
/// - file content matchers
pub fn eval(
    e: &Expr<NamePredicate, MetadataPredicate, ContentPredicate>,
    path: &Path,
) -> std::io::Result<bool> {
    let mut viz: Option<Viz> = None;

    let e: Expr<Done, MetadataPredicate, ContentPredicate> = e
        .map_predicate(
            &mut viz,
            "evaluate name predicates".to_string(),
            String::new(),
            |p| p.eval_name_predicate(path),
        );

    if let Expr::Literal(b) = e {

        if let Some(v) = viz {
            let v = v.append_label("Done".to_string(), "evaluation required examining filename only".to_string());
           // v.write(format!("visualized"))
        }
        return Ok(b);
    }

    // read metadata
    let metadata = fs::metadata(path)?;

    let e: Expr<Done, Done, ContentPredicate> = e
        .map_predicate(
            &mut viz,
            "evaluate metadata predicates".to_string(),
            String::new(),
            |p| p.eval_metadata_predicate(&metadata),
        );

    if let Expr::Literal(b) = e {

        if let Some(v) = viz {
            let v = v.append_label("Done".to_string(), "evaluation required examining filename and metadata only".to_string());
           // v.write(format!("visualized"))
        }
        return Ok(b);
    }

    // only try to read contents if it's a file according to entity metadata
    let utf8_contents = if metadata.is_file() {
        // read contents
        let contents = fs::read(path)?;
        String::from_utf8(contents).ok()
    } else {
        None
    };

    let e: Expr<Done, Done, Done> = e
        .map_predicate(
            &mut viz,
            "evaluate file content predicates".to_string(),
            String::new(),
            |p| p.eval_file_content_predicate(utf8_contents.as_ref()),
        );

        if let Some(v) = viz {
            let v = v.append_label("Done".to_string(), "evaluation required examining filename, metadata, and file contents".to_string());
        //    v.write(format!("visualized {}", path.to_str().unwrap()))
        }


    if let Expr::Literal(b) = e {
        Ok(b)
    } else {
        // this is unreachable because at this point we've replaced every
        // predicate with boolean literals and reduced all binary operators
        unreachable!("programmer error")
    }
}
