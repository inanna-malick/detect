use std::{env::current_dir, str::FromStr};

use clap::{command, Parser};
use detect::{parse_and_run_fs, parse_and_run_github, parser::parse_expr};
use slog::{o, Drain, Level, Logger};

/// operators
/// - `a && b` a and b
/// - `a || b`: a or b
/// - `!a`: not a
/// - `(a)`: parens to clarify grouping
///
/// ## string operators
/// - `==`
/// - `~=` (regex match)
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
    #[arg(short = 'i')]
    visit_gitignored: bool,
    // TODO: expand to full owner/repo/ref input
    #[arg(short = 'g')]
    github: bool,
    #[arg(short = 'l', default_value = "warn")]
    log_level: String,
}

#[tokio::main]
pub async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let logger = Logger::root(
        RuntimeLevelFilter {
            drain: slog_term::FullFormat::new(plain).build(),
            level: Level::from_str(&args.log_level)
                .unwrap_or_else(|_| panic!("invalid log level {}", args.log_level)),
        }
        .fuse(),
        o!(),
    );

    let expr = parse_expr(&args.expr)?;

    parse_and_run_github(logger, expr, |s| println!("{}", s)).await

    // parse_and_run_fs(
    //     logger,
    //     &current_dir()?,
    //     !args.visit_gitignored,
    //     args.expr,
    //     |s| println!("{}", s.to_string_lossy()),
    // )
    // .await
}

/// Custom Drain logic
struct RuntimeLevelFilter<D> {
    drain: D,
    level: Level,
}

impl<D> Drain for RuntimeLevelFilter<D>
where
    D: Drain,
{
    type Ok = Option<D::Ok>;
    type Err = Option<D::Err>;

    fn log(
        &self,
        record: &slog::Record,
        values: &slog::OwnedKVList,
    ) -> Result<Self::Ok, Self::Err> {
        if record.level().is_at_least(self.level) {
            self.drain.log(record, values).map(Some).map_err(Some)
        } else {
            Ok(None)
        }
    }
}
