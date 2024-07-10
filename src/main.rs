use std::env::current_dir;

use clap::{command, Parser};
use detect::parse_and_run;

/// operators
/// - `a && b` a and b
/// - `a || b`: a or b
/// - `!a`: not a
/// - `(a)`: parens to clarify grouping
///
/// ## string operators
/// - `==`
/// - `~=` (regex match)
/// - `contains`
/// ## numeric operators
/// - `>`, `>=`, `<`, `<=`
/// - `==`
/// ## file path selectors
/// - name
/// - path
/// - extension
/// ## metadata selectors
/// - size
/// - type
/// ## file contents predicates
/// - contents
 #[derive(Parser, Debug)]
 #[command(
     name = "detect",
     author,
     version,
     about,
     long_about,
     verbatim_doc_comment
 )]
 struct Args {
     /// filtering expr
     #[clap(index = 1)]
     expr: String,
    #[arg(short = 'i', long)]
     visit_gitignored: bool,
 }

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    parse_and_run(current_dir()?, !args.visit_gitignored, args.expr, |s| {
        println!("{}", s.to_string_lossy())
    })
    .await
}
