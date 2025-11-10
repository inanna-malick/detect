use std::{env::current_dir, io::Write, path::PathBuf, str::FromStr};

use clap::Parser;
use detect::{parse_and_run_fs, RuntimeConfig};
use slog::{o, Drain, Level, Logger};

const EXAMPLES: &str = include_str!("../docs/examples.md");
const PREDICATES: &str = include_str!("../docs/predicates.md");
const OPERATORS: &str = include_str!("../docs/operators.md");

#[derive(Parser, Debug)]
#[command(
    name = "detect",
    author,
    version,
    about = "Find filesystem entities using expressions",
    long_about = "Find filesystem entities using expressions

EXIT CODES:
  0  Matches found
  1  No matches found
  2  Error (parse error, directory not found, etc.)"
)]
struct Args {
    /// Show help on specific topics: examples, predicates, operators
    ///
    /// Without argument, lists available topics
    #[arg(
        long = "explain",
        value_name = "TOPIC",
        num_args = 0..=1,
        default_missing_value = "list",
        require_equals = false,
    )]
    explain: Option<String>,

    /// filtering expr
    #[clap(index = 1, required_unless_present = "explain")]
    expr: Option<String>,

    /// target dir
    #[clap(index = 2)]
    path: Option<PathBuf>,
    /// include gitignored files
    #[arg(short = 'i')]
    visit_gitignored: bool,
    /// log level (trace/debug/info/warning/error/critical)
    #[arg(short = 'l', default_value = "warning")]
    log_level: String,
    /// Maximum file size for structured data parsing (yaml/json/toml)
    /// Supports units: kb, mb, gb (e.g., "10mb", "500kb")
    #[arg(long = "max-structured-size", default_value = "10mb")]
    max_structured_size: String,
}

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Handle --explain flag
    if let Some(topic) = args.explain {
        match topic.to_lowercase().as_str() {
            "list" => {
                println!("Available help topics:\n");
                println!("  examples    - Practical usage examples for common tasks");
                println!("  predicates  - Reference of all selectors (name, size, content, yaml, etc.)");
                println!("  operators   - Reference of all operators (==, contains, ~=, etc.)");
                println!("\nUsage: detect --explain <TOPIC>");
                println!("   or: detect --explain (shows this list)");
            }
            "examples" => println!("{}", EXAMPLES),
            "predicates" | "selectors" => println!("{}", PREDICATES),
            "operators" | "ops" => println!("{}", OPERATORS),
            _ => {
                eprintln!("Error: Unknown topic '{}'\n", topic);
                eprintln!("Available topics: examples, predicates, operators");
                eprintln!("Run 'detect --explain' to see all topics");
                std::process::exit(2);
            }
        }
        return Ok(());
    }

    let expr = args
        .expr
        .expect("Expression required when --explain isn't used, should be present");

    let max_structured_size =
        detect::util::parse_size(&args.max_structured_size).unwrap_or_else(|e| {
            eprintln!("Error: {e}");
            std::process::exit(1);
        });

    let config = RuntimeConfig {
        max_structured_size,
    };

    let log_level = Level::from_str(&args.log_level).unwrap_or_else(|_| {
        eprintln!(
            "Error: Invalid log level '{}'\nValid options: trace, debug, info, warning, error, critical",
            args.log_level
        );
        std::process::exit(1);
    });

    let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let logger = Logger::root(
        RuntimeLevelFilter {
            drain: slog_term::FullFormat::new(plain).build(),
            level: log_level,
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

    let result = parse_and_run_fs(
        logger,
        &root_path,
        !args.visit_gitignored,
        expr,
        config,
        |s| {
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
        },
    )
    .await;

    match result {
        Ok(match_count) => {
            if match_count > 0 {
                std::process::exit(0); // Matches found
            } else {
                std::process::exit(1); // No matches
            }
        }
        Err(e) => {
            eprintln!("{:?}", miette::Report::new(e));
            std::process::exit(2); // Error
        }
    }
}

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
