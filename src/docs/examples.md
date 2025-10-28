# detect Examples

Quick examples for common file search tasks.

## Basic File Finding

```bash
# All Rust files
detect 'ext == rs'

# JavaScript/TypeScript files
detect 'ext in [js,ts,jsx,tsx]'

# Files with "test" in name
detect 'name contains test'

# README files
detect 'name == README.md'
```

## Content Searching

```bash
# Files containing TODO comments
detect 'content contains TODO'

# Files with class definitions (regex)
detect 'content ~= "class.*Service"'

# Angular decorators
detect 'content ~= "@(Component|Injectable|Directive)"'

# TODOs outside test directories
detect 'content contains TODO AND NOT path ~= test'
```

## Structured Data Queries

Query configuration file contents with path-based selectors:

```bash
# Find YAML files with specific port
detect 'yaml:.server.port == 8080'

# Find package.json with React dependency
detect 'json:.dependencies.react'

# Find Cargo.toml with specific edition
detect 'toml:.package.edition == "2021"'

# Version range matching with regex
detect 'json:.dependencies.serde ~= "^1\\."'

# Find configs with debugging enabled
detect 'yaml:.debug == true'

# Array wildcard - match ANY enabled feature
detect 'yaml:.features[*].enabled == true'

# Recursive descent - find ANY port field > 8000
detect 'yaml:..port > 8000 OR json:..port > 8000 OR toml:..port > 8000'

# Combine with file filters
detect 'size < 50kb AND yaml:.database.host contains prod'

# Find configs outside test directories
detect 'yaml:.server AND NOT path contains test'

# Type coercion - matches both int and string
detect 'yaml:.version == "1.0"'  # matches 1.0 or "1.0"

# Nested field access
detect 'json:.metadata.author == "test"'

# Array indexing
detect 'yaml:.features[0].name == "auth"'

# String matcher on structured data
detect 'toml:.dependencies.* contains "async"'

# Find Kubernetes manifests with high replica counts
detect 'yaml:.spec.replicas > 3'

# Security: find configs with production credentials
detect 'yaml:..password contains prod AND NOT path contains test'
```

## Size and Time Filters

```bash
# Large files (over 1MB)
detect 'size > 1mb'

# Large recent files
detect 'size > 100kb AND modified > -7d'

# Files changed in last hour
detect 'modified > -1h'

# Small documentation files
detect 'ext == md AND size < 10kb'
```

## Complex Combinations

```bash
# Rust files with async code
detect 'ext == rs AND content ~= async'

# Large TypeScript files without tests
detect 'ext == ts AND size > 10kb AND NOT content contains test'

# Recent changes to config files
detect 'ext in [json,yaml,toml,ini] AND modified > -3d'

# Build files and scripts
detect 'name in [Makefile,Dockerfile,package.json] OR ext in [sh,py,js]'

# Find potential secrets (be careful!)
detect 'content ~= "(password|secret|api_key)" AND NOT path ~= test'
```

## Directory and Path Filtering

```bash
# Files only in src directory
detect 'path ~= "^./src/" AND type == file'

# Exclude node_modules and target directories
detect 'ext == js AND NOT path ~= "(node_modules|target)"'

# Files at specific depth
detect 'depth == 2 AND ext == rs'
```

## Migration from Common Tools

```bash
# find . -name "*.js" -size +1M
detect 'ext == js AND size > 1mb'

# find . -type f -exec grep -l "TODO" {} \;
detect 'type == file AND content contains TODO'

# find . -name "*test*" -mtime -7
detect 'name contains test AND modified > -7d'

# find . -type d
detect 'type == dir'

# grep -r "class.*Service" --include="*.ts" .
detect 'ext == ts AND content ~= "class.*Service"'
```

## File Type Aliases (Convenience Shortcuts)

For common file type queries, single-word aliases provide shorthand:

```bash
# Regular files only
detect 'file'

# Directories only
detect 'dir'

# Symbolic links
detect 'symlink'

# Subdirectories (not root)
detect 'dir && depth > 0'

# Large regular files
detect 'file && size > 10mb'

# Recent directories
detect 'dir && modified > -7d'
```

**Available aliases:** `file`, `dir`/`directory`, `symlink`/`link`, `socket`/`sock`, `fifo`/`pipe`, `block`/`blockdev`, `char`/`chardev` (case-insensitive)

**Equivalence:**
- `file` is shorthand for `type == file`
- `dir && depth > 0` is shorthand for `type == dir AND depth > 0`

## Tips

- Use `ext == value` for extension matching (not wildcards)
- Use `name contains text` for filename substring searches
- Use regex for complex patterns: `name ~= "pattern"`
- File type aliases (`file`, `dir`, `symlink`) are convenient shortcuts
- Combine aliases with predicates: `file && size > 1mb`
- Quote expressions with spaces or special characters
- Use `-i` flag to include gitignored files
- Start with metadata filters before content searches for performance
