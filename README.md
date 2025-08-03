# detect

A fast, powerful tool for finding files by name, content, and metadata using an expressive query language.

```shell
➜  detect 'path.extension == rs && contents ~= async'
./src/main.rs
./src/lib.rs
./src/eval/fs.rs

➜  detect 'size > 50000 && modified > -7.days && contents contains TODO'
./target/debug/build/main.rs
./docs/planning.md
```

## Quick Start

```bash
# Install
cargo install detect

# Find files by name
detect 'path.name == README.md'

# Search file contents  
detect 'contents contains TODO'

# Complex queries
detect 'path.extension == ts && contents ~= @Injectable && !path.full contains test'
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
path.extension == ts && contents ~= @(Component|Injectable|Directive)

# Security audit
path.extension in [env, json, yml] && contents ~= (password|secret|api_key)

# Large files without documentation
size > 10000 && !contents contains TODO && !contents contains @doc

# Recent changes
modified > -7.days && (contents contains FIXME || contents contains TODO)

# Complex grouping with parentheses
(path.extension == js || path.extension == ts) && (contents contains import || contents contains require) && size > 1000
```

### Operators  
Any operator works with any selector of compatible type:
- **String ops** (for name/path/contents): `==`, `!=`, `contains`, `~=`, `in [...]`
- **Number ops** (for size): `==`, `!=`, `>`, `<`, `>=`, `<=`, `in [...]`
- **Time ops** (for dates): `==`, `!=`, `>`, `<`, `>=`, `<=`, `in [...]`
- **Boolean**: `&&`, `||`, `!`, `()`

### Selectors
- **String type**: `path.stem`, `path.name`, `path.parent`, `path.full`, `path.extension`, `contents`
- **Number type**: `size`
- **Time type**: `modified`, `created`, `accessed`
- **Enum type**: `type` (file/dir/symlink)

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
detect 'path.extension == ts && size > 5000 && modified > -7.days && contents contains TODO'
```

More readable, more powerful, and often faster.
