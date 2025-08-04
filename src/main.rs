use std::{env::current_dir, path::PathBuf, str::FromStr};

use clap::{command, Parser};
use detect::{output::safe_stdout, parse_and_run_fs};
use miette::{IntoDiagnostic, Report};
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
    // /// search within git ref (commit/branch/tag)
    // #[arg(short = 'g', long = "gitref")]
    // gitref: Option<String>,
    /// log level (error/warn/info/debug)
    #[arg(short = 'l', default_value = "warn")]
    log_level: String,
}

#[tokio::main]
pub async fn main() -> miette::Result<()> {
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
        None => current_dir().into_diagnostic()?,
    };

    // Create safe output handler that manages broken pipe errors
    let mut output = safe_stdout();

    // let expr = parse_expr(&args.expr)?;

    // if let Some(ref_) = args.gitref {
    //     run_git(logger, &root_path, &ref_, expr, |s| {
    //         if let Err(e) = output.writeln(s) {
    //             eprintln!("Output error: {}", e);
    //             std::process::exit(1);
    //         }
    //     })?;
    // } else {
    let result = parse_and_run_fs(logger, &root_path, !args.visit_gitignored, args.expr, |s| {
        if let Err(e) = output.writeln(&s.to_string_lossy()) {
            eprintln!("Output error: {}", e);
            std::process::exit(1);
        }
    })
    .await;

    // Convert DetectError to Miette diagnostic if possible
    match result {
        Ok(()) => Ok(()),
        Err(detect_err) => {
            // Try to convert to a diagnostic for rich error display
            if let Some(diagnostic) = detect_err.to_diagnostic() {
                Err(Report::new(diagnostic))
            } else {
                // Fall back to regular error display
                Err(miette::Error::msg(detect_err.to_string()))
            }
        }
    }
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
