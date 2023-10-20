use clap::{Parser, command};
use detect::parse_and_run;


/// operators
/// - `a && b` a and b
/// - `a || b`: a or b
/// - `!a`: not a
/// - `(a)`: parens to clarify grouping
/// 
/// file path predicates
/// - `filename($REGEX)`: file name
/// - `filepath($REGEX)`: file path
/// - `extension($STRING)` exact match on extension
/// 
/// metadata predicates
/// - `dir()`: is dir
/// - `executable()`: is executable
/// - `size(n1..n2)`: file size in range n1 to n2
/// - `size(..n)`: file size less than n
/// - `size(n..)`: file size larger than n
/// 
/// file contents predicates
/// - `contains($REGEX)`: file contents
/// - `utf8()`: file contents are utf8
/// 
/// for example:
#[derive(Parser, Debug)]
#[command(name = "detect", author, version, about, long_about, verbatim_doc_comment)]
struct Args {
    /// filtering expr
    #[clap(index=1)]
    expr: String,
}

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    parse_and_run(".".to_owned(), args.expr, |s| {
        println!("{}", s.to_string_lossy())
    })
    .await
}
