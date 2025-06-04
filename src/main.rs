use std::{env::current_dir, io::Write, path::PathBuf, str::FromStr};

use clap::{command, Parser};
use detect::{parse_and_run_fs, parser::parse_expr, run_git};
use slog::{o, Drain, Level, Logger};

/// Fast file finder with intuitive syntax - always prefer simple over verbose
///
/// Examples (SIMPLE - preferred):
///   detect TODO                              # Search for "TODO" in file contents
///   detect "*.rs"                            # Find files matching pattern
///   detect --type rust                       # Find all Rust files
///   detect ">1MB"                            # Files larger than 1MB
///   detect "*.rs TODO"                       # Rust files containing TODO
///   
/// Examples (EXPRESSION MODE - use only when needed):
///   detect -e "(*.rs || *.go) && TODO"       # Parentheses require -e flag
///   detect -e "lines > 100"                  # Predicates not available as simple filters
#[derive(Parser, Debug)]
#[command(
    name = "detect",
    author,
    version,
    about = "Fast file finder with intuitive syntax",
    long_about = None,
)]
struct Args {
    /// Search pattern (searches content by default, or filenames if pattern contains wildcards)
    #[clap(index = 1)]
    pattern: Option<String>,

    /// Target directory (defaults to current directory)
    #[clap(index = 2)]
    path: Option<PathBuf>,

    /// File type filter (rust, python, js, go, etc.)
    #[arg(short = 't', long = "type")]
    file_type: Option<String>,

    /// Search only in this path
    #[arg(long = "in")]
    in_path: Option<String>,

    /// Use expression syntax for complex queries (prefer simple syntax when possible)
    #[arg(short = 'e', long = "expr")]
    expression: Option<String>,

    /// Include gitignored files
    #[arg(short = 'i')]
    visit_gitignored: bool,

    /// Git ref to search at
    #[arg(short = 'g', long = "gitref")]
    gitref: Option<String>,
    
    /// Git range to search for changes (e.g., HEAD~10..HEAD, main..feature)
    #[arg(long = "git-range")]
    git_range: Option<String>,

    /// Log level
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

    let root_path = match &args.path {
        Some(path) => path.clone(),
        None => current_dir()?,
    };

    println!("path: {:?}", root_path);

    // Build the query based on CLI arguments
    let query_str = if let Some(expr) = args.expression {
        // Explicit expression mode
        expr
    } else {
        // Build query from flags and pattern
        build_query_from_args(&args)?
    };

    let expr = parse_expr(&query_str)?;

    // Check for conflicting git options
    if args.gitref.is_some() && args.git_range.is_some() {
        anyhow::bail!("Cannot use both --gitref and --git-range at the same time");
    }

    if let Some(ref_) = args.gitref {
        run_git(logger, &root_path, &ref_, expr, |s| {
            if let Err(e) = writeln!(std::io::stdout(), "{}", s) {
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    std::process::exit(0);
                }
            }
        })?;
    } else if let Some(range) = args.git_range {
        detect::run_git_range(logger, &root_path, &range, expr, |s| {
            if let Err(e) = writeln!(std::io::stdout(), "{}", s) {
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    std::process::exit(0);
                }
            }
        })?;
    } else {
        parse_and_run_fs(logger, &root_path, !args.visit_gitignored, query_str, |s| {
            if let Err(e) = writeln!(std::io::stdout(), "{}", s.to_string_lossy()) {
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    std::process::exit(0);
                }
            }
        })
        .await?;
    }

    Ok(())
}

/// Build a query string from CLI arguments using the new syntax
fn build_query_from_args(args: &Args) -> Result<String, anyhow::Error> {
    // Handle the simple cases with the new syntax

    // Just a pattern
    if let Some(pattern) = &args.pattern {
        if args.file_type.is_none() && args.in_path.is_none() {
            // Simple case - just the pattern
            return Ok(pattern.clone());
        }

        // Pattern with filters
        let mut query = String::new();

        // File type + pattern
        if let Some(file_type) = &args.file_type {
            query.push_str(file_type);
            query.push(' ');
        }

        query.push_str(pattern);

        // Add path filter
        if let Some(in_path) = &args.in_path {
            query.push_str(&format!(" in:{}", in_path));
        }

        return Ok(query);
    }

    // Just file type
    if let Some(file_type) = &args.file_type {
        return Ok(file_type.clone());
    }

    // Default to all files
    Ok("*".to_string())
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
