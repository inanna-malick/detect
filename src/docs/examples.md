# detect Examples

Quick examples for common file search tasks.

## Basic File Finding

```bash
# All Rust files
detect '*.rs'
detect 'ext == rs'

# JavaScript/TypeScript files
detect '*.{js,ts,jsx,tsx}'
detect 'ext in [js,ts,jsx,tsx]'

# Files with "test" in name
detect '*test*'
detect 'name contains test'

# README files
detect '*README*'
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

## Size and Time Filters

```bash
# Large files (over 1MB)
detect 'size > 1mb'

# Large recent files
detect 'size > 100kb AND modified > -7d'

# Files changed in last hour
detect 'modified > -1h'

# Small documentation files
detect '*.md && size < 10kb'
```

## Complex Combinations

```bash
# Rust files with async code
detect '*.rs && content ~= async'

# Large TypeScript files without tests
detect 'ext == ts AND size > 10kb AND NOT content contains test'

# Recent changes to config files
detect '*.{json,yaml,toml,ini} && modified > -3d'

# Build files and scripts
detect 'name in [Makefile,Dockerfile,package.json] OR *.{sh,py,js}'

# Find potential secrets (be careful!)
detect 'content ~= "(password|secret|api_key)" AND NOT path ~= test'
```

## Directory and Path Filtering

```bash
# Files only in src directory
detect 'path ~= "^./src/" AND type == file'

# Exclude node_modules and target directories
detect '*.js AND NOT path ~= "(node_modules|target)"'

# Files at specific depth
detect 'depth == 2 AND ext == rs'
```

## Migration from Common Tools

```bash
# find . -name "*.js" -size +1M
detect '*.js && size > 1mb'

# find . -type f -exec grep -l "TODO" {} \;
detect 'content contains TODO'

# find . -name "*test*" -mtime -7
detect '*test* && modified > -7d'

# grep -r "class.*Service" --include="*.ts" .
detect 'ext == ts AND content ~= "class.*Service"'
```

## Tips

- Use `*.pattern` for familiar shell-style matching
- Use predicates for more complex filters
- Combine both: `*.rs && size > 1kb`
- Quote expressions with spaces or special characters
- Use `-i` flag to include gitignored files