use std::{env::current_dir, io::Write, path::PathBuf, str::FromStr};

use clap::{command, Parser};
use detect::parse_and_run_fs;
use slog::{o, Drain, Level, Logger};

const EXPR_GUIDE: &str = include_str!("docs/expr_guide.md");

#[derive(Parser, Debug)]
#[command(
    name = "detect",
    author,
    version,
    about = "Find filesystem entities using expressions",
    long_about = EXPR_GUIDE
)]
struct Args {
    /// filtering expr
    #[clap(index = 1)]
    expr: String,
    /// target dir
    #[clap(index = 2)]
    path: Option<PathBuf>,
    /// include gitignored files
    #[arg(short = 'i')]
    visit_gitignored: bool,
    /// log level (error/warn/info/debug)
    #[arg(short = 'l', default_value = "warn")]
    log_level: String,
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let root_path = match args.path {
        Some(path) => path,
        None => current_dir()?,
    };

    let mut output = std::io::stdout();

    let result = parse_and_run_fs(logger, &root_path, !args.visit_gitignored, args.expr, |s| {
        if let Err(e) = writeln!(output, "{}", &s.to_string_lossy()) {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                // Unix convention: exit 0 on SIGPIPE/BrokenPipe
                std::process::exit(0);
            } else {
                eprintln!("Output error: {}", e);
                std::process::exit(1);
            }
        }
    })
    .await;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
    Ok(())
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
