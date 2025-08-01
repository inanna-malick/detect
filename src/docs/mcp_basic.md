Search filesystem entities by name, metadata AND contents in a single query.

Every query follows: selector operator value

## OPERATORS

**Equality & Comparison**
- `==` or `=` - exact match
- `!=` - not equal
- `>` - greater than
- `<` - less than
- `>=` - greater or equal
- `<=` - less or equal

**String Matching**
- `contains` - substring search
- `~=` or `~` or `=~` - regex match
- `in [v1, v2, ...]` - set membership

**Boolean Logic**
- `&&` - AND
- `||` - OR
- `!` - NOT
- `()` - grouping

## SELECTORS

**Name/Path**
- `path.stem` - filename without extension
- `path.name` - complete filename with extension
- `path.parent` - directory path only
- `path.full` (or `path`) - complete path including filename
- `path.suffix` - extension without dot

**Metadata**
- `size` or `filesize` - bytes
- `type` or `filetype` - file/dir/symlink

**Content**
- `contents` or `file` - file contents

**Time**
- `modified` or `mtime` - modification time
- `created` or `ctime` - creation time
- `accessed` or `atime` - access time

## EXAMPLES

**Basic Queries**
```
path.name == README.md
path.stem == README
path.suffix == rs
size > 1000000
contents contains TODO
modified > "-7.days"
```

**Complex Patterns**
```
# Multiple patterns in contents
contents ~= (TODO|FIXME|HACK)

# TypeScript decorators
contents ~= @(Injectable|Component)

# Exclude paths
!path.parent contains node_modules
!path.full contains test

# Combined conditions
path.suffix == ts && size > 5000 && contents contains async && !path.parent contains test
```

**Time Queries**
```
modified > "-30.minutes"    # Relative
created > "2024-01-01"      # Absolute
```

**Set Membership**
```
path.suffix in [js, ts, jsx]
path.stem in [index, main, app]
```

## POWER PATTERNS

**Content Regex**
```
contents ~= class\s+\w+Service       # Service classes
contents ~= import.*from\s+['"]react # React imports
contents ~= @\w+                     # Any decorator
```

**Security Scans**
```
path.suffix in [env, json, yml] && contents ~= (password|secret|api_key)
contents ~= (BEGIN|END).*(PRIVATE|KEY)
```

**Code Quality**
```
# Large complex files
size > 10000 && contents ~= (async|await|Promise)

# Stale tests
path.name contains test && modified < "-90.days"

# Files without tests
path.name ~= \.service\.ts$ && !contents contains test
```

**Smart Exclusions**
```
path.suffix == js && !path.parent contains node_modules && contents contains TODO
path.suffix == py && !path.parent contains __pycache__ && contents contains import
```

## NOTES

- All string comparisons are case-sensitive
- Regex uses Rust syntax (escape dots: `\.`)
- Size is in bytes
- Quotes required for: times, regex with spaces, values with special chars

Need more? Use the detect_help tool.