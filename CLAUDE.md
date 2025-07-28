# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`detect` is a command-line tool for finding filesystem entities using an expression language. It's written in Rust and provides powerful search capabilities through a custom query language with boolean operators, string matching, regex support, and temporal selectors. The project also includes an MCP (Model Context Protocol) server for integration with Claude Desktop.

## Build and Development Commands

### Basic Commands
- **Build**: `cargo build` (debug) or `cargo build --release` (optimized)
- **Run**: `cargo run -- '<expression>' [path]`
- **Test**: `cargo test`
- **Format**: `cargo fmt`
- **Lint**: `cargo clippy`
- **Type Check**: `cargo check`

### Running Tests
- **All tests**: `cargo test`
- **Specific test**: `cargo test test_name`
- **Integration tests only**: `cargo test --test integration`
- **With output**: `cargo test -- --nocapture`
- **Single-threaded**: `cargo test -- --test-threads=1`

### MCP Server
The MCP server binary is built separately:
```bash
cargo build --release --bin detect-mcp
```

## Architecture Overview

The codebase follows a modular architecture with clear separation of concerns:

### Core Components

1. **Expression Parser** (`src/parser.rs`, `src/expr/expr.pest`)
   - Uses the Pest parsing library with a PEG grammar
   - Implements a Pratt parser for operator precedence
   - Converts string expressions into AST structures

2. **Expression Evaluation** (`src/eval/`)
   - `fs.rs`: Filesystem-based evaluation
   - `git.rs`: Git repository evaluation
   - Handles different predicate types (name, metadata, content)

3. **Predicate System** (`src/predicate.rs`)
   - Defines selectors (@name, @path, @size, etc.)
   - Implements operators (==, ~=, >, <, in, contains, glob)
   - Supports streaming content evaluation for large files

4. **Expression Types** (`src/expr.rs`)
   - Generic expression tree supporting different predicate types
   - Implements short-circuit evaluation for performance
   - Frame-based evaluation for complex expressions

### Key Design Patterns

1. **Generic Expression Tree**: The `Expr<T>` type is generic over predicate types, allowing the same expression structure to work with different evaluation contexts (filesystem vs git).

2. **Streaming Content Evaluation**: Content predicates use `StreamingCompiledContentPredicate` to handle large files efficiently without loading entire contents into memory.

3. **Visitor Pattern**: The expression tree uses a visitor-like pattern for evaluation, with different evaluators for filesystem and git contexts.

4. **Error Propagation**: Uses `anyhow` for error handling with contextual information throughout the evaluation pipeline.

## Expression Language

The tool uses a custom expression language defined in `src/expr/expr.pest`:

- **Boolean operators**: `&&`, `||`, `!`, `()`
- **String operators**: `==`, `~=` (regex), `contains`, `glob`
- **Numeric operators**: `>`, `>=`, `<`, `<=`, `==`
- **Set operators**: `in [item1, item2, ...]`
- **Selectors**: All start with `@` (e.g., `@name`, `@size`, `@contents`)
- **Temporal selectors**: `@modified`, `@created`, `@accessed`

## Testing Strategy

- **Unit tests**: Located alongside implementation files
- **Integration tests**: In `tests/integration.rs`
- **Temporal tests**: In `tests/temporal_tests.rs` for time-based queries
- **Test utilities**: Helper functions for creating test filesystems

## Important Implementation Details

1. **Git Integration**: Uses `git2` library for repository traversal
2. **Ignore Files**: Respects `.gitignore` by default (can be overridden with `-i`)
3. **Async Runtime**: Uses Tokio for async filesystem operations
4. **Logging**: Configurable logging with slog framework
5. **MCP Protocol**: Implements JSON-RPC based Model Context Protocol for Claude Desktop integration