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
cargo run -- [PATTERN] [PATH]           # Simple search
cargo run -- -e '<expression>' [PATH]   # Expression mode

# Examples (New Simple Syntax)
cargo run -- TODO                       # Find "TODO" in files
cargo run -- '*.rs'                     # Find Rust files
cargo run -- --type rust                # Find all Rust files
cargo run -- --type rust TODO           # Find Rust files containing TODO
cargo run -- TODO --in src              # Find TODO in src/ directory

# Examples (Expression Mode)
cargo run -- -e 'size > 1MB'
cargo run -- -e '*.rs && size > 1000'
cargo run -- -e 'contains(/TODO/) && !hidden'
```

## Architecture

The codebase follows a pipeline architecture:

1. **Parsing** (`parser.rs` + `expr/expr.pest`): Uses Pest parser generator to parse expression strings into AST
2. **Expression AST** (`expr.rs`): Type-safe representation of boolean expressions with predicates
3. **Evaluation** (`eval/`): Two evaluation backends:
   - `fs.rs`: Filesystem traversal using async I/O
   - `git.rs`: Git repository traversal at specific refs
4. **Predicates** (`predicate.rs`): Modular system for different match types (name, path, contents, size, type, extension)

Key architectural decisions:
- **Short-circuit evaluation**: Evaluates predicates in cost order (path → metadata → content)
- **Streaming content search**: Uses DFA-compiled regexes for efficient file content matching
- **Frame-based evaluation** (`expr/frame.rs`): Tracks which predicates need evaluation at each stage
- Uses `recursion` crate for AST transformations and `ignore` crate for respecting .gitignore

## Usage Modes

### Simple Mode (Default)
- Bare words search content: `detect TODO`
- Glob patterns search filenames: `detect '*.rs'`
- File type shortcuts: `detect --type rust`
- Combine easily: `detect --type python TODO`

### Expression Mode (-e flag)
For complex queries, use the expression syntax:
- Predicates: `size > 1MB`, `name = "test.rs"`, `contains(/regex/)`
- Boolean logic: `&&` (AND), `||` (OR), `!` (NOT)
- Comparisons: `>`, `>=`, `<`, `<=`, `=`, `~` (regex match)

## Memories

- This is a perfect opportunity to use the detect tool instead of find. Do so at every possible opportunity
- close! actually you can use detect instead of grep too :D
- use detect instead of grep - you're dogfooding using the tool you're building