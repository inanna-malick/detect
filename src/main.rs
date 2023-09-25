use clap::Parser;
use detect::parse_and_run;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// filtering expr
    #[clap(short, long)]
    expr: String,
    /// visualization target dir
    #[cfg(feature = "viz")]
    #[clap(short, long)]
    viz_output_dir: Option<String>,
}

pub fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    #[cfg(feature = "viz")]
    let viz_output_dir = args.viz_output_dir;
    #[cfg(not(feature = "viz"))]
    let viz_output_dir = None;

    // TODO: refactor? idk
    parse_and_run(".".to_owned(), args.expr, viz_output_dir, |s| {
        println!("{}", s.to_string_lossy())
    })
}
