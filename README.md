[![Crates.io](https://img.shields.io/crates/v/detect.svg)](https://crates.io/crates/detect)

# detect

A modern replacement for find/grep using an intuitive expression language.

- **Readable syntax**: `ext == ts AND size > 50kb` instead of `find . -name "*.ts" -size +50k`
- **Unified queries**: Combine filename + content + metadata instead of chaining multiple processes
- **Lazy evaluation**: Detect checks cheap predicates first (filename, metadata) and short circuits whenever possible

[Quick start](#quick-start) • [Installation](#installation) • [Query language](#query-language) • [Examples](#examples)

Traditional Unix tools require chaining multiple commands with cryptic syntax:

```bash
# Find Rust files importing BOTH tokio and serde
detect 'ext == rs 
        AND content contains "use tokio" 
        AND content contains "use serde"'

# Traditional approach, requiring two passes per matching .rs file
grep -rl 'use tokio' --include="*.rs" | xargs grep -l 'use serde'
```

Detect also supports searches inspecting structured data in YAML, TOML, and JSON files:

```bash
# Find Cargo.toml files with package edition 2018
detect 'name == "Cargo.toml" AND toml:.package.edition == 2018'

# using regexes (may result in false positives)
find . -name "Cargo.toml" -exec grep -q 'edition.*"2018"' {} \; -print

# using cryptaliagy's tomlq crate
find . -name "Cargo.toml" -exec sh -c '
  tq -f "$1" -r ".package.edition" 2>/dev/null | grep -q "2018"
' _ {} \; -print
```


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

## Quick start

```bash
detect 'ext == rs'                                    # selector + operator
detect 'ext in [rs,toml] AND size > 1mb'             # sets, AND, numeric
detect 'ext == ts AND modified > -7d'                 # temporal predicates
detect 'ext == ts AND content ~= "class.*Service"'   # content, regex
detect '(file OR dir) AND NOT path ~= test'          # aliases, grouping, NOT
detect 'yaml:.server.port > 8000 AND size < 0.5mb'   # structured data
```

## Query language

### Selectors

#### File Identity
| Selector | Type | Description | Example |
|----------|------|-------------|---------|
| `name` / `filename` | String | Full filename with extension | `name == "README.md"` |
| `basename` / `stem` | String | Filename without extension | `basename == README` |
| `ext` / `extension` | String | File extension (no dot) | `ext == rs` |
| `path` | String | Full absolute path | `path contains /src/` |
| `dir` / `parent` / `directory` | String | Parent directory path | `dir contains lib` |

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
| `modified` / `mtime` | Temporal | Last modification time | `modified > -7d` |
| `created` / `ctime` | Temporal | File creation time | `created > 2024-01-01` |
| `accessed` / `atime` | Temporal | Last access time | `accessed < -1h` |

**Time formats:** Relative `-7d`/`-7days`, `-2h`/`-2hours`, `-1w`/`-1week` (units: `s`, `m`/`min`, `h`/`hr`, `d`/`day`, `w`/`week` + plurals). Absolute `2024-01-15`, `2024-01-15T10:30:00`.

#### Content
| Selector | Type | Description | Example |
|----------|------|-------------|---------|
| `content` / `text` / `contents` | String | File text contents | `content contains TODO` |

#### Structured Data

Query YAML, JSON, and TOML:

```bash
yaml:.server                        # existence check (no operator needed)
yaml:.server.port == 8080           # nested field value
toml:.package.edition == "2021"     # value match
yaml:.features[*].enabled == true   # wildcard - any array element
json:..password contains prod       # recursive - any depth
```

Navigate with `.field`, `.nested.field`, `[0]`, `[*]`, `..field`. Auto-converts between numbers and strings (`yaml:.port == 8080` matches both `8080` and `"8080"`). Default max file size: 10MB (configurable with `--max-structured-size`).

### Operators

| Type | Operators | Example |
|------|-----------|---------|
| String | `==`, `!=`, `contains`, `~=`, `in [a,b]` | `content contains TODO` |
| Numeric | `==`, `!=`, `>`, `<`, `>=`, `<=` | `size > 1mb` |
| Temporal | `>`, `<`, `>=`, `<=`, `==`, `!=` | `modified > -7d` |
| Enum | `==`, `!=`, `in [a,b]` | `type == file` |
| Boolean | `AND`/`&&`, `OR`/`||`, `NOT`/`!`, `()` | `a AND (b OR c)` |

**Precedence:** `NOT` > `AND` > `OR`

Full reference: `detect --operators`

## Examples

```bash
# File metadata combinations
detect 'ext == rs AND size > 1mb AND modified > -7d'

# Content matching with regex
detect 'ext == ts AND content ~= "class.*Service"'

# Structured data navigation
detect 'yaml:.server.port == 8080'
detect 'toml:.package.edition == "2021"'

# Multi-feature real-world queries
detect 'size > 10kb AND modified > -7d AND content contains TODO AND NOT path ~= test'
detect 'yaml:.spec.replicas > 3 AND size < 100kb'

# Security scanning
detect 'name ~= "^\.env" AND content ~= "(password|secret|key)" AND NOT path ~= node_modules'

# Migration from find/grep
find . -name "*.ts" -size +1M -mtime -7  →  detect 'ext == ts AND size > 1mb AND modified > -7d'

# CLI options
detect 'ext == rs' ./src                              # search specific directory
detect -i 'content contains SECRET'                   # include gitignored files
detect --max-structured-size 50mb 'yaml:.config'      # configure size limit for structured files
```

**More examples:** `detect --examples`

## Exit codes

Compatible with scripting and CI/CD pipelines (same as `grep`/`ripgrep`):

- **0** - Matches found
- **1** - No matches
- **2** - Error (parse error, directory not found, etc.)

```bash
# Use in conditionals
if detect 'size > 100mb'; then
    echo "Found large files"
fi

# CI: fail build if TODOs found
detect 'path contains src AND content contains TODO' && exit 1
```

## Performance

Queries are evaluated in four phases: name → metadata → structured → content. Each phase can eliminate files before more expensive operations. Content is never read unless the file passes all earlier checks.

Respects `.gitignore` by default. Traverses directories in parallel. Structured data parsing is limited to 10MB files (configurable).

## Contributing

Contributions welcome. File an issue before major changes.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
