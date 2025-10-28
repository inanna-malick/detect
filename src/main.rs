use std::{env::current_dir, io::Write, path::PathBuf, str::FromStr};

use clap::{command, Parser};
use detect::{parse_and_run_fs, RuntimeConfig};
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
    /// Maximum file size for structured data parsing (yaml/json/toml)
    /// Supports units: kb, mb, gb (e.g., "10mb", "500kb")
    #[arg(long = "max-structured-size", default_value = "10mb")]
    max_structured_size: String,
}

/// Parse size values like "1mb", "100kb" from CLI arguments
fn parse_cli_size(s: &str) -> Result<u64, String> {
    let s = s.trim().to_lowercase();

    // Find where the unit starts
    let mut unit_start = 0;
    for (i, ch) in s.char_indices() {
        if !ch.is_ascii_digit() && ch != '.' {
            unit_start = i;
            break;
        }
    }

    if unit_start == 0 {
        return Err(format!(
            "Invalid size '{}': expected format like '10mb', '500kb'",
            s
        ));
    }

    let number_str = &s[..unit_start];
    let unit_str = &s[unit_start..];

    let number: f64 = number_str.parse().map_err(|_| {
        format!(
            "Invalid size '{}': cannot parse numeric value '{}'",
            s, number_str
        )
    })?;

    let multiplier = match unit_str {
        "b" | "byte" | "bytes" => 1.0,
        "k" | "kb" | "kilobyte" | "kilobytes" => 1024.0,
        "m" | "mb" | "megabyte" | "megabytes" => 1024.0 * 1024.0,
        "g" | "gb" | "gigabyte" | "gigabytes" => 1024.0 * 1024.0 * 1024.0,
        "t" | "tb" | "terabyte" | "terabytes" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => {
            return Err(format!(
                "Invalid size '{}': unknown unit '{}' (expected: b, kb, mb, gb, tb)",
                s, unit_str
            ))
        }
    };

    Ok((number * multiplier) as u64)
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

    // Parse max structured size
    let max_structured_size = parse_cli_size(&args.max_structured_size).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });

    let config = RuntimeConfig {
        max_structured_size,
    };

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

    let result = parse_and_run_fs(logger, &root_path, !args.visit_gitignored, expr, config, |s| {
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
