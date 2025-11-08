pub mod aliases;
pub mod ast;
pub mod error;
pub mod raw;
pub mod structured_path;
pub mod time;
pub mod typechecker;
pub mod typed;

// Re-exports
pub use aliases::{resolve_alias, suggest_aliases};
pub use ast::{test_utils, RawExpr, RawPredicate, RawValue};
pub use error::DetectError;
pub use raw::RawParser;
pub use structured_path::{parse_path, PathComponent, PathParseError};
pub use time::parse_time_value;
pub use typechecker::Typechecker;
