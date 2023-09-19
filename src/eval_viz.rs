use recursion_schemes::recursive::collapse::Collapsable;
use recursion_visualize::visualize::{CollapsableV, Viz};

use crate::expr::frame::Operator;
use crate::expr::short_circuit::ShortCircuit;
use crate::expr::{frame::ExprFrame, Expr};
use crate::expr::{ContentPredicate, MetadataPredicate, NamePredicate};
use crate::predicate::Predicate;
use crate::util::Done;
use std::fs::{self};
use std::path::{Path, PathBuf};

pub struct VisualizedEval {
    name_matcher: Option<(String, Viz)>,
    metadata_matcher: Option<(String, Viz)>,
    content_matcher: Option<(String, Viz)>,
}

impl VisualizedEval {
    pub fn do_thing(self, dir: String) {
        let (s, viz) =  self.name_matcher.unwrap();
        let mut viz = viz.label(format!("eval using file path: {}", s));

        if let Some((s, x)) = self.metadata_matcher {
            viz = viz.fuse(x, format!("eval using file metadata: {}", s));
        }

        if let Some((s, x)) = self.content_matcher {
            viz = viz.fuse(x, format!("eval using file contents: {}", s));
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

    println!("{}", e);

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
