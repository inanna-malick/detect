use fileset_expr::parse_and_run;

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// paths to filter out
    #[clap(short, long)]
    expr: String,
}

#[tokio::main]
pub async fn main() {
    let args = Args::parse();

    let res = parse_and_run(args.expr).await;
    res.expect("failed")
}
