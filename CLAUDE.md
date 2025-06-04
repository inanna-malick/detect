# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`detect` is a command-line tool for finding files and directories using an expression language. It's similar to Unix `find` but with a more intuitive query syntax supporting boolean expressions, regex matching, and various file predicates.

## Build and Test Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build

# Test
cargo test              # Run all tests

# Run
cargo run -- '<expression>' [path]

# Examples
cargo run -- '@name ~= detect'
cargo run -- '@extension == rs && @size > 1000'
cargo run -- -g HEAD '@contents ~= TODO'
```

## Architecture

The codebase follows a pipeline architecture:

1. **Parsing** (`parser.rs` + `expr/expr.pest`): Uses Pest parser generator to parse expression strings into AST
2. **Expression AST** (`expr.rs`): Type-safe representation of boolean expressions with predicates
3. **Evaluation** (`eval/`): Two evaluation backends:
   - `fs.rs`: Filesystem traversal using async I/O
   - `git.rs`: Git repository traversal at specific refs
4. **Predicates** (`predicate.rs`): Modular system for different match types (@name, @path, @contents, @size, @type, @extension)

Key architectural decisions:
- **Short-circuit evaluation**: Evaluates predicates in cost order (path → metadata → content)
- **Streaming content search**: Uses DFA-compiled regexes for efficient file content matching
- **Frame-based evaluation** (`expr/frame.rs`): Tracks which predicates need evaluation at each stage
- Uses `recursion` crate for AST transformations and `ignore` crate for respecting .gitignore

## Expression Syntax

All selectors start with `@`:
- `@name`, `@path`, `@extension` - String predicates
- `@size` - Numeric predicate (in bytes)
- `@type` - Either "file" or "dir"
- `@contents` - File content search

Operators:
- Boolean: `&&` (AND), `||` (OR), `!` (NOT)
- String: `==` (exact match), `~=` (regex match)
- Numeric: `>`, `>=`, `<`, `<=`, `==`