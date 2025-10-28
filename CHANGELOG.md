# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2025-01-22

### Added

- **Structured data selectors** for querying YAML, JSON, and TOML file contents
  - Dot notation for nested field access: `yaml:.server.port`
  - Array indexing: `json:.dependencies[0]`
  - Wildcards with OR semantics: `yaml:.features[*].enabled`
  - Recursive descent: `yaml:..field` finds field at any depth
  - Comparison operators: `==`, `!=`, `>`, `<`, `>=`, `<=`
  - String matchers: `contains`, `~=` (regex)
  - Automatic type coercion between numbers and strings
  - Fully composable with other predicates: `size < 50kb AND yaml:.server.port > 8000`
- `--max-structured-size` CLI flag to configure maximum file size for structured parsing (default: 10MB)
- Support for multi-document YAML with OR semantics (matches if ANY document matches)
- **Single-word file type aliases**: Use `file`, `dir`, `symlink`, etc. as shorthand for `type == file`, `type == dir`, etc. Enables natural queries like `dir && depth > 0` or `file && size > 1mb`. All file type values work as aliases (case-insensitive).
- MCP (Model Context Protocol) server support for AI assistant integration
- Better error messages with source location tracking and helpful suggestions
- Unquoted regex pattern support - `content ~= [0-9]+` works without quotes
- Parse-time validation for `type` selector values (breaking change - see below)
- Relative path display in search results
- Dual MIT/Apache-2.0 licensing
- Greater than or equal (`>=`) and less than or equal (`<=`) operators for temporal selectors

### Changed

- Two-phase parser architecture (raw parsing → type checking) for better errors
- Relative time formats now support full aliases (`-7days`, `-2hours`, etc.)

### Breaking Changes

- `type` selector now validates file type values at parse time. Invalid types like `type == dirq` produce parse errors instead of matching nothing. Valid types: `file`, `dir`/`directory`, `symlink`/`link`, `socket`/`sock`, `fifo`/`pipe`, `block`/`blockdev`, `char`/`chardev` (case-insensitive)
