pub mod aliases;
pub mod ast;
pub mod error;
pub mod raw;
pub mod typechecker;
pub mod typed;

// Re-exports for clean API
pub use aliases::{resolve_alias, suggest_aliases};
pub use ast::{test_utils, RawExpr, RawPredicate, RawValue};
pub use error::RawParseError;
pub use raw::RawParser;
pub use typechecker::{TypecheckError, Typechecker};
