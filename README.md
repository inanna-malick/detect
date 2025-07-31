# detect

A fast, powerful tool for finding files by name, content, and metadata using an expressive query language.

```shell
➜  detect 'ext == rs && contents ~= async'
./src/main.rs
./src/lib.rs
./src/eval/fs.rs

➜  detect 'size > 50000 && modified > "-7.days" && contents contains TODO'
./target/debug/build/main.rs
./docs/planning.md
```

## Quick Start

```bash
# Install
cargo install detect

# Find files by name
detect 'name == README.md'

# Search file contents  
detect 'contents contains TODO'

# Complex queries
detect 'ext == ts && contents ~= @Injectable && !path contains test'
```

## Key Features

- **Search by content AND metadata** in a single query
- **Regex support** for powerful pattern matching
- **Time-based queries** (e.g., files modified in last 7 days)
- **Boolean logic** with AND, OR, NOT, and grouping
- **Fast** - optimized for large codebases

## Expression Language

Every query follows: `selector operator value`

### Common Patterns

```bash
# Find TypeScript files with decorators
ext == ts && contents ~= @(Component|Injectable|Directive)

# Security audit
ext in [env, json, yml] && contents ~= (password|secret|api_key)

# Large files without documentation
size > 10000 && !contents contains TODO && !contents contains @doc

# Recent changes
modified > "-7.days" && (contents contains FIXME || contents contains TODO)
```

### Operators
- **Comparison**: `==`, `!=`, `>`, `<`, `>=`, `<=`
- **String**: `contains`, `~=` (regex), `in [...]`
- **Boolean**: `&&`, `||`, `!`, `()`

### Selectors
- **Name/Path**: `name`, `path`, `ext`
- **Metadata**: `size`, `type`, `modified`, `created`
- **Content**: `contents`

## Documentation

For comprehensive documentation, run:
```bash
detect --help
```

This shows:
- All operators and selectors
- Building complex queries
- Performance tips
- Common patterns
- Unix pipeline integration

## MCP Integration

Detect includes an MCP (Model Context Protocol) server for integration with Claude Desktop and other MCP clients. See [MCP documentation](src/docs/mcp_basic.md) for details.

## Why detect?

Traditional tools require multiple commands:
```bash
# Old way
find . -name "*.ts" -type f -size +5k -mtime -7 -exec grep -l "TODO" {} \;
```

With detect:
```bash
# New way  
detect 'ext == ts && size > 5000 && modified > "-7.days" && contents contains TODO'
```

More readable, more powerful, and often faster.
