use fileset_expr::parse_and_run;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// filtering expr
    #[clap(short, long)]
    expr: String,
}

pub fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    parse_and_run(args.expr)
}
