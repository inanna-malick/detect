use clap::Parser;
use detect::parse_and_run;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// filtering expr
    #[clap(short, long)]
    expr: String,
}

pub fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    parse_and_run(".".to_owned(), args.expr, |s| {
        println!("{}", s.to_string_lossy())
    })
}
