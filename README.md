# detect

A fast, powerful tool for finding files by name, content, and metadata using an expressive query language. Drop-in replacement for complex `find`/`grep` pipelines.

```shell
➜  detect 'path.extension == rs && contents ~= async'
./src/main.rs
./src/lib.rs
./src/eval/fs.rs

➜  detect 'size > 50kb && modified > -7d && contents contains TODO'
./docs/planning.md
./src/experimental.rs
```

## Quick Start

```bash
cargo install detect

# Find files by name pattern
detect 'path.name == README.md'

# Search file contents
detect 'contents contains TODO'

# Complex queries with boolean logic
detect 'path.ext == ts AND contents ~= @Injectable AND NOT path.full contains test'
```

## Why detect?

Traditional Unix tools require chaining multiple commands with complex syntax:

```bash
# Old way: find TypeScript files >5KB modified in last week containing TODO
find . -name "*.ts" -type f -size +5k -mtime -7 -exec grep -l "TODO" {} \;

# New way: same query, readable syntax
detect 'path.ext == ts AND size > 5kb AND modified > -7d AND contents contains TODO'
```

## Key Features

- **Unified search** - content AND metadata in single query
- **Natural syntax** - readable boolean expressions instead of cryptic flags  
- **Regex support** - powerful pattern matching across all text fields
- **Fast execution** - optimized query planning (metadata filters before content scanning)
- **Time queries** - intuitive relative/absolute date filtering
- **Type safety** - prevents nonsensical queries at parse time

## Practical Examples

```bash
# Security audit: find config files with secrets
detect 'path.ext in [env,json,yml] AND contents ~= (password|secret|api_key)'

# Code quality: large files without tests or docs
detect 'size > 10kb AND NOT contents contains test AND NOT contents contains TODO'

# Angular components with specific decorators
detect 'path.ext == ts AND contents ~= @(Component|Injectable|Directive)'

# Recent changes to build files
detect 'modified > -7d AND path.name ~= (Makefile|.*\.mk|build\.)'

# Complex boolean logic with grouping
detect '(path.ext == js OR path.ext == ts) AND (contents contains import OR contents contains require) AND size > 1kb'
```

**Full syntax reference and advanced features**: `detect --help`

## Performance

detect optimizes query execution automatically:
- Applies fast metadata filters first (name, size, dates)
- Only scans file contents for files passing metadata filters
- Uses streaming regex engines for large file content matching
- Respects `.gitignore` by default (override with `-i`)