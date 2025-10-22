# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2025-01-22

### Added

- MCP (Model Context Protocol) server support for AI assistant integration
- Better error messages with source location tracking and helpful suggestions
- Unquoted regex pattern support - `content ~= [0-9]+` works without quotes
- Parse-time validation for `type` selector values (breaking change - see below)
- Relative path display in search results
- Dual MIT/Apache-2.0 licensing

### Changed

- Two-phase parser architecture (raw parsing → type checking) for better errors
- Relative time formats now support full aliases (`-7days`, `-2hours`, etc.)

### Breaking Changes

- `type` selector now validates file type values at parse time. Invalid types like `type == dirq` produce parse errors instead of matching nothing. Valid types: `file`, `dir`/`directory`, `symlink`/`link`, `socket`/`sock`, `fifo`/`pipe`, `block`/`blockdev`, `char`/`chardev` (case-insensitive)
