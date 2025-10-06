use std::{env::current_dir, io::Write, path::PathBuf, str::FromStr};

use clap::{command, Parser};
use detect::parse_and_run_fs;
use slog::{o, Drain, Level, Logger};

const EXAMPLES: &str = include_str!("docs/examples.md");
const PREDICATES: &str = include_str!("docs/predicates.md");
const OPERATORS: &str = include_str!("docs/operators.md");

#[derive(Parser, Debug)]
#[command(
    name = "detect",
    author,
    version,
    about = "Find filesystem entities using expressions"
)]
struct Args {
    /// Run as MCP server for AI assistants
    #[cfg(feature = "mcp")]
    #[arg(long = "mcp")]
    mcp: bool,

    /// Show practical examples
    #[arg(long = "examples")]
    examples: bool,

    /// Show available predicates/selectors
    #[arg(long = "predicates")]
    predicates: bool,

    /// Show operator reference
    #[arg(long = "operators")]
    operators: bool,

    /// filtering expr
    #[cfg(feature = "mcp")]
    #[clap(index = 1, required_unless_present_any = ["mcp", "examples", "predicates", "operators"])]
    expr: Option<String>,
    #[cfg(not(feature = "mcp"))]
    #[clap(index = 1, required_unless_present_any = ["examples", "predicates", "operators"])]
    expr: Option<String>,
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

    // Handle info flags first
    if args.examples {
        println!("{}", EXAMPLES);
        return Ok(());
    }

    if args.predicates {
        println!("{}", PREDICATES);
        return Ok(());
    }

    if args.operators {
        println!("{}", OPERATORS);
        return Ok(());
    }

    // If --mcp flag is set, run as MCP server
    #[cfg(feature = "mcp")]
    if args.mcp {
        return match detect::mcp_server::run_mcp_server().await {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("MCP server error: {}", e);
                std::process::exit(1);
            }
        };
    }

    // Normal detect mode
    let expr = args
        .expr
        .expect("Expression is required when not in MCP mode");

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

    // Canonicalize root path for relative path computation
    let canonical_root = root_path
        .canonicalize()
        .unwrap_or_else(|_| root_path.clone());

    let mut output = std::io::stdout();

    let result = parse_and_run_fs(logger, &root_path, !args.visit_gitignored, expr, |s| {
        // Convert to relative path for cleaner output
        let display_path = s
            .strip_prefix(&canonical_root)
            .unwrap_or(s)
            .to_string_lossy();

        if let Err(e) = writeln!(output, "{}", display_path) {
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

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            // Display the error using miette's formatting
            eprintln!("{:?}", miette::Report::new(e));
            std::process::exit(1);
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
