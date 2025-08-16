# detect

A fast, powerful tool for finding files by name, content, and metadata using an expressive query language. Drop-in replacement for complex `find`/`grep` pipelines.

```shell
➜  detect 'ext == rs && content ~= async'
./src/main.rs
./src/lib.rs
./src/eval/fs.rs

➜  detect 'size > 50kb && modified > -7d && content contains TODO'
./docs/planning.md
./src/experimental.rs
```

## Quick Start

```bash
cargo install detect

# Find files by name pattern (name without extension)
detect 'basename == README && ext == md'

# Find files by exact filename  
detect 'name == README.md'

# Search file contents
detect 'content contains TODO'

# Complex queries with boolean logic
detect 'ext == ts AND content ~= @Injectable AND NOT path contains test'
```

## Why detect?

Traditional Unix tools require chaining multiple commands with complex syntax:

```bash
# Old way: find TypeScript files >5KB modified in last week containing TODO
find . -name "*.ts" -type f -size +5k -mtime -7 -exec grep -l "TODO" {} \;

# New way: same query, readable syntax
detect 'ext == ts AND size > 5kb AND modified > -7d AND content contains TODO'
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
detect 'ext in [env,json,yml] AND content ~= (password|secret|api_key)'

# Code quality: large files without tests or docs
detect 'size > 10kb AND NOT content contains test AND NOT content contains TODO'

# Angular components with specific decorators
detect 'ext == ts AND content ~= @(Component|Injectable|Directive)'

# Recent changes to build files
detect 'modified > -7d AND name ~= (Makefile|.*\.mk|build\.)'

# Complex boolean logic with grouping
detect '(ext == js OR ext == ts) AND (content contains import OR content contains require) AND size > 1kb'
```

**Full syntax reference and advanced features**: `detect --help`

## Performance

detect optimizes query execution automatically:
- Applies fast metadata filters first (name, size, dates)
- Only scans file content for files passing metadata filters
- Uses streaming regex engines for large file content matching
- Respects `.gitignore` by default (override with `-i`)