# detect

[![Crates.io](https://img.shields.io/crates/v/detect.svg)](https://crates.io/crates/detect)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/inanna-malick/detect)

A fast, powerful tool for finding files by name, content, and metadata using an expressive query language. A readable, intuitive replacement for complex `find`/`grep` pipelines.

```bash
# Find TypeScript files with async code
detect 'ext == ts AND content ~= "async "'

# Find large recent files with TODOs
detect 'size > 50kb AND modified > -7d AND content contains TODO'

# Find React components outside test directories
detect 'ext in [jsx,tsx] AND content ~= "function \w+\(" AND NOT path contains test'
```

## Why detect?

Traditional Unix tools require chaining multiple commands with cryptic syntax:

```bash
# find + grep
find . -name "*.ts" -type f -size +5k -mtime -7 -exec grep -l "TODO" {} \;

# detect
detect 'ext == ts AND size > 5kb AND modified > -7d AND content contains TODO'
```

One tool, one expression. Query filename, size, timestamps, and content together. Validates queries at parse-time so you catch typos immediately.

## Installation

### From crates.io

```bash
cargo install detect
```

### Building from source

**Prerequisites:** Rust toolchain (1.70+)

```bash
git clone https://github.com/inanna-malick/detect.git
cd detect
cargo build --release

# Binary will be at ./target/release/detect
# Optionally install globally:
cargo install --path .
```

## Quick Start

```bash
# Find all Rust files
detect 'ext == rs'

# Find README files by exact name
detect 'name == README.md'

# Find TypeScript or JavaScript files
detect 'ext in [ts,js,tsx,jsx]'

# Find files containing TODO comments
detect 'content contains TODO'

# Find directories (using alias)
detect 'dir'

# Find large regular files (using alias)
detect 'file && size > 1mb'
```

## Single-Word Aliases

For common file type queries, detect provides convenient single-word aliases:

```bash
# File type aliases (shorthand for type == value)
detect 'file'              # Regular files
detect 'dir'               # Directories
detect 'symlink'           # Symbolic links

# Combine with other predicates
detect 'dir && depth > 0'           # Subdirectories only
detect 'file && size > 10mb'        # Large regular files
detect 'NOT symlink && modified > -7d'  # Recent non-symlink files
```

**Available aliases:** `file`, `dir`/`directory`, `symlink`/`link`, `socket`/`sock`, `fifo`/`pipe`, `block`/`blockdev`, `char`/`chardev` (all case-insensitive)

**Equivalence:**
- `dir` is shorthand for `type == dir`
- `file && size > 1mb` is shorthand for `type == file AND size > 1mb`

## Query Language

### Selectors

#### File Identity
| Selector | Type | Description | Example |
|----------|------|-------------|---------|
| `name` | String | Full filename with extension | `name == "README.md"` |
| `basename` | String | Filename without extension | `basename == README` |
| `ext` | String | File extension (no dot) | `ext == rs` |
| `path` | String | Full absolute path | `path contains /src/` |
| `dir` | String | Parent directory path | `dir contains lib` |

**Aliases:** `filename` → `name`, `stem` → `basename`, `extension` → `ext`, `parent`/`directory` → `dir`

#### File Properties
| Selector | Type | Description | Example |
|----------|------|-------------|---------|
| `size` | Numeric | File size in bytes | `size > 1mb` |
| `type` | Enum | File type (parse-time validated) | `type == file` |
| `depth` | Numeric | Directory depth from search root | `depth <= 3` |

**Size units:** `kb`, `mb`, `gb`, `tb` (e.g., `1.5mb`, `500kb`)

**File types** (case-insensitive): `file`, `dir`/`directory`, `symlink`/`link`, `socket`/`sock`, `fifo`/`pipe`, `block`/`blockdev`, `char`/`chardev`

#### Timestamps
| Selector | Type | Description | Example |
|----------|------|-------------|---------|
| `modified` | Temporal | Last modification time | `modified > -7d` |
| `created` | Temporal | File creation time | `created > 2024-01-01` |
| `accessed` | Temporal | Last access time | `accessed < -1h` |

**Time formats:**
- Relative: `-7d` / `-7days`, `-2h` / `-2hours`, `-30m` / `-30minutes`, `-1w` / `-1week`
  - Supported units: `s`/`sec`/`second`, `m`/`min`/`minute`, `h`/`hr`/`hour`, `d`/`day`, `w`/`week` (+ plurals)
- Absolute: `2024-01-15`, `2024-01-15T10:30:00`

**Aliases:** `mtime` → `modified`, `ctime` → `created`, `atime` → `accessed`

#### Content
| Selector | Type | Description | Example |
|----------|------|-------------|---------|
| `content` | String | File text contents | `content contains TODO` |

**Aliases:** `contents`, `text`

#### Structured Data

Query YAML, JSON, and TOML:

```bash
yaml:.server.port == 8080           # nested field
toml:.package.edition == "2021"     # value match
yaml:.features[*].enabled == true   # wildcard - any array element
yaml:..password contains prod       # recursive - any depth
```

Navigate with `.field`, `.nested.field`, `[0]`, `[*]`, `..field`. Auto-converts between numbers and strings (`yaml:.port == 8080` matches both `8080` and `"8080"`). Default max file size: 10MB (configurable with `--max-structured-size`).

### Operators

#### String Operators
For: `name`, `basename`, `ext`, `path`, `dir`, `content`

| Operator | Description | Example |
|----------|-------------|---------|
| `==` | Exact match (case-sensitive) | `name == "test.rs"` |
| `!=` | Not equal | `ext != md` |
| `contains` | Substring search (literal) | `content contains TODO` |
| `~=` | Regex pattern matching | `name ~= "test.*\.rs$"` |
| `in [a,b,c]` | Match any value in set | `ext in [js,ts,jsx,tsx]` |

#### Numeric Operators
For: `size`, `depth`

| Operator | Description | Example |
|----------|-------------|---------|
| `==`, `!=` | Equals, not equals | `size == 1024` |
| `>`, `<` | Greater than, less than | `size > 1mb` |
| `>=`, `<=` | Greater/less or equal | `depth <= 2` |

#### Temporal Operators
For: `modified`, `created`, `accessed`

| Operator | Description | Example |
|----------|-------------|---------|
| `>` | After (newer than) | `modified > -7d` |
| `<` | Before (older than) | `created < 2024-01-01` |
| `>=`, `<=` | At or after/before | `modified >= -1d` |
| `==`, `!=` | Exact time match | `modified == 2024-01-01` |

#### Enum Operators
For: `type`

| Operator | Description | Example |
|----------|-------------|---------|
| `==` | Exact match (validated at parse-time) | `type == file` |
| `!=` | Not equal | `type != dir` |
| `in [a,b,c]` | Match any type in set | `type in [file,dir,symlink]` |

**Valid file types** (case-insensitive): `file`, `dir`/`directory`, `symlink`/`link`, `socket`/`sock`, `fifo`/`pipe`, `block`/`blockdev`, `char`/`chardev`

#### Boolean Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `AND` / `&&` | Both conditions must be true | `ext == rs AND size > 1kb` |
| `OR` / `\|\|` | Either condition must be true | `ext == md OR ext == txt` |
| `NOT` / `!` | Negate condition | `NOT path contains test` |
| `( )` | Group expressions | `(ext == rs OR ext == toml) AND size > 1kb` |

**Precedence:** `NOT` > `AND` > `OR` (use parentheses for clarity)

## Examples

### Getting Started

```bash
# All Rust files
detect 'ext == rs'

# Find README files
detect 'name == README.md'

# TypeScript or JavaScript files
detect 'ext in [ts,js,tsx,jsx]'

# Files containing TODO comments
detect 'content contains TODO'
```

### Common Tasks

```bash
# Find large files
detect 'size > 1mb'

# Recent changes to config files
detect 'ext in [json,yaml,toml] AND modified > -7d'

# Rust source files without tests
detect 'ext == rs AND NOT content contains #[test]'

# Find async or exported functions
detect 'content ~= "(async\s+|export\s+)?function"'
```

### Content Search with Regex

```bash
# Find TypeScript class definitions
detect 'ext == ts AND content ~= "class \w+ "'

# Find API keys (simple pattern)
detect 'content ~= "api[_-]?key"'

# Find TODO, FIXME, or NOTE comments
detect 'content ~= "(TODO|FIXME|NOTE):"'

# Find React or Vue imports
detect 'content ~= "import .* from .+(react|vue)"'
```

### Structured Data Queries

```bash
# Find YAML files with specific server port
detect 'yaml:.server.port == 8080'

# Find Cargo.toml files using 2021 edition
detect 'toml:.package.edition == "2021"'

# Version range matching with regex
detect 'json:.dependencies.serde ~= "^1\\."'

# Find configs with debugging enabled
detect 'yaml:.debug == true'

# Array wildcard example
detect 'yaml:.features[*].enabled == true'

# Recursive descent across all formats
detect 'yaml:..port > 8000 OR json:..port > 8000 OR toml:..port > 8000'

# Combine with metadata filters
detect 'size < 50kb AND yaml:.database.host contains prod'

# Kubernetes manifests with high replica counts
detect 'yaml:.spec.replicas > 3'

# Security: find production credentials outside tests
detect 'yaml:..password contains prod AND NOT path contains test'
```

### Complex Queries

```bash
# Large recent TypeScript files with TODOs, not in tests
detect 'ext == ts AND size > 10kb AND modified > -7d AND content contains TODO AND NOT path contains test'

# Security: env files with secrets outside node_modules
detect 'name ~= "^\.env" AND NOT path contains node_modules AND content ~= "(password|secret|key)"'

# Migration helper: find old API patterns in source directories
detect 'path ~= "^\./(src|lib)/" AND content ~= "(oldApi|legacyMethod)"'
```

### CLI Options

```bash
# Search specific directory
detect 'ext == rs' ./src

# Include gitignored files
detect -i 'content contains SECRET'

# Configure max file size for structured parsing
detect --max-structured-size 50mb 'yaml:.config'

# Debug logging
detect -l debug 'complex query here'
```

## Performance

Queries are evaluated in four phases: name → metadata → structured → content. Each phase can eliminate files before more expensive operations. Content is never read unless the file passes all earlier checks.

Respects `.gitignore` by default. Traverses directories in parallel. Structured data parsing is limited to 10MB files (configurable).

## MCP Server Integration

detect includes built-in MCP (Model Context Protocol) support for AI assistants like [Claude Code](https://claude.ai/code):

```bash
# Run as MCP server
detect --mcp
```

This enables AI assistants to perform filesystem searches using detect's expressive query language. See [MCP.md](MCP.md) for full configuration details.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
