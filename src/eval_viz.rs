use recursion_visualize::visualize::{CollapsibleV, Viz};

use crate::expr::frame::Operator;
use crate::expr::short_circuit::ShortCircuit;
use crate::expr::{frame::ExprFrame, Expr};
use crate::expr::{ContentPredicate, MetadataPredicate, NamePredicate};
use crate::predicate::Predicate;
use crate::util::Done;
use std::fs::{self};
use std::path::Path;

pub struct VisualizedEval {
    name_matcher: Option<(String, Viz)>,
    metadata_matcher: Option<(String, Viz)>,
    content_matcher: Option<(String, Viz)>,
}

impl VisualizedEval {
    pub fn write_viz(self, dir: String) {
        let (s, viz) =  self.name_matcher.unwrap();
        let mut viz = viz.label("Eval File Path Predicates".to_string(), s);
        println!("match on file name");

        if let Some((s, x)) = self.metadata_matcher {
            println!("and file meta");
            viz = viz.fuse(x, "Eval File Metadata Predicates".to_string(), s);
        }

        if let Some((s, x)) = self.content_matcher {
            println!("and file contents");
            viz = viz.fuse(x, "Eval File Content Predicates".to_string(), s);
        }

        viz.write(format!("{}/viz.html", dir))

       }
}

/// multipass evaluation with short circuiting, runs, in order:
/// - file name matchers
/// - metadata matchers
/// - file content matchers
pub fn eval_v(
    e: &Expr<Predicate<NamePredicate, MetadataPredicate, ContentPredicate>>,
    path: &Path,
) -> std::io::Result<(bool, VisualizedEval)> {
    let mut ve = VisualizedEval { name_matcher: None, metadata_matcher: None, content_matcher: None };

    let ev = format!("{}", e);
    let e: Expr<Predicate<Done, &MetadataPredicate, &ContentPredicate>> = {
        let (e, v) = e.collapse_frames_v(|frame| match frame {
            ExprFrame::Operator(op) => op.attempt_short_circuit(),
            ExprFrame::Predicate(p) => p.eval_name_predicate(path),
        }) ;

        ve.name_matcher = Some((ev, v));

        match e {
            ShortCircuit::Known(x) => return Ok((x, ve)),
            ShortCircuit::Unknown(e) => e,
        }
    };

    // read metadata via STAT syscall
    let metadata = fs::metadata(path)?;

    let ev = format!("{}", e);
    let e: Expr<Predicate<Done, Done, &ContentPredicate>> = {
        let (e,v) = e.collapse_frames_v(|frame| match frame {
            ExprFrame::Operator(op) => op.attempt_short_circuit(),
            ExprFrame::Predicate(p) => p.eval_metadata_predicate(&metadata),
        }) ;

        ve.metadata_matcher = Some((ev, v));

        match e {
            ShortCircuit::Known(x) => return Ok((x, ve)),
            ShortCircuit::Unknown(e) => e,
        }
    };

    // only try to read contents if it's a file according to entity metadata
    let utf8_contents = if metadata.is_file() {
        // read file contents via multiple syscalls
        let contents = fs::read(path)?;
        String::from_utf8(contents).ok()
    } else {
        None
    };

    let ev = format!("{}", e);
    let (res, v) = e.collapse_frames_v::<bool>(|frame| match frame {
        // don't attempt short circuiting, because we know we can calculate a result here
        ExprFrame::Operator(op) => match op {
            Operator::Not(x) => !x,
            Operator::And(a, b) => a && b,
            Operator::Or(a, b) => a || b,
        },
        ExprFrame::Predicate(p) => p.eval_file_content_predicate(utf8_contents.as_deref()),
    });

    ve.content_matcher = Some((ev, v));

    Ok((res, ve))
}
