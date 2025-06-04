# Simple Syntax Guide - Always Prefer the Simplest Form

This guide shows how to use detect's simple, ergonomic syntax instead of verbose predicates.

## 🚫 DON'T use verbose predicates when simple alternatives exist

### Content Search
```bash
# ❌ AVOID
detect 'contains(TODO)'
detect 'contains(/TODO/)'
detect 'contains("TODO")'

# ✅ PREFER
detect TODO
```

### Filename Patterns
```bash
# ❌ AVOID
detect 'ext == rs'
detect 'ext = "rs"'
detect 'name ~= /\.rs$/'

# ✅ PREFER
detect '*.rs'
```

### Exact Filename
```bash
# ❌ AVOID
detect 'name == parser.rs'
detect 'name = "parser.rs"'

# ✅ PREFER
detect '"parser.rs"'
```

### Size Filters
```bash
# ❌ AVOID
detect 'size > 1000'
detect 'size > 1048576'  # 1MB in bytes

# ✅ PREFER
detect '>1000'
detect '>1MB'
```

### Empty Files
```bash
# ❌ AVOID
detect 'size == 0'
detect 'size = 0'

# ✅ PREFER
detect empty
```

### File Types
```bash
# ❌ AVOID
detect 'ext == rs || ext == toml'
detect 'type = file && ext = py'

# ✅ PREFER
detect '*.{rs,toml}'
detect 'python'  # or 'py' for short
```

### Combined Searches
```bash
# ❌ AVOID
detect 'ext == rs && contains(TODO)'
detect 'type = file && size > 1MB && contains(/unsafe/)'

# ✅ PREFER
detect '*.rs TODO'
detect '>1MB unsafe'
```

## ✅ When to use expression syntax

Use the `-e` flag and expression syntax ONLY when you truly need:

### 1. Parentheses for Grouping
```bash
detect -e '(*.rs || *.go) && TODO'
```

### 2. Complex Boolean Logic
```bash
detect -e '(python || javascript) && (TODO || FIXME) && !test'
```

### 3. Specific Predicates Not Available as Filters
```bash
detect -e 'lines > 100 && lines < 500'  # line count not available as simple filter
```

## Summary

The detect tool is designed for simplicity. In 99% of cases, you should use:
- Bare words for content search
- Glob patterns for filenames
- Simple filters for size/type/properties
- Space-separated terms for AND operations

Only reach for expression syntax when the simple syntax genuinely can't express what you need.