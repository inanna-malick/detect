# Detect Advanced Reference

## Complete Operator Set

### Equality Operators
- `==` or `=` - Exact match
- `!=` - Not equal

### String Pattern Operators  
- `~=` - Regex match (uses Rust regex syntax)
- `contains` - Substring search

### Comparison Operators
- `>` - Greater than
- `<` - Less than  
- `>=` - Greater than or equal
- `<=` - Less than or equal

### Set Membership
- `in [...]` - Value in set

## Regex Patterns

The `~=` operator supports full regex syntax:

```
name ~= "^test_"           - Files starting with test_
name ~= "\.rs$"            - Files ending with .rs
name ~= "v\d+\.\d+\.\d+"   - Version patterns like v1.2.3
contents ~= "fn\s+main"    - Rust main functions
path ~= ".*/src/.*\.rs$"   - Rust files in src directories
```

### Important Regex Notes
- Patterns are NOT anchored by default
- Use `^` and `$` for start/end anchoring
- Escape special regex chars: `\.` for literal dot
- Case-sensitive by default

## Boolean Operator Precedence

From highest to lowest:
1. `!` (negation)
2. `&&` (and)
3. `||` (or)

Examples:
```
!name contains test && ext == rs || size > 1000
# Parses as: ((!name contains test) && (ext == rs)) || (size > 1000)

ext == rs && size > 1000 || name contains test
# Parses as: ((ext == rs) && (size > 1000)) || (name contains test)
```

Use parentheses to override precedence:
```
!(name contains test && ext == rs)
ext == rs && (size > 1000 || name contains test)
```

## All Selectors

### Name/Path Selectors
- `name`, `filename` - Match against filename
- `path`, `filepath` - Match against full path
- `ext`, `extension` - File extension (without dot)

### Metadata Selectors
- `size`, `filesize` - Size in bytes
- `type`, `filetype` - Entity type (file, dir, symlink)

### Content Selectors
- `contents`, `file` - Search file contents

### Temporal Selectors
- `modified`, `mtime` - Modification time
- `created`, `ctime` - Creation time
- `accessed`, `atime` - Access time

## Temporal Query Syntax

### Relative Time
Format: `"-N.unit"` (quotes required)

Units:
- seconds, minutes, hours, days, weeks, months

Examples:
```
modified > "-30.seconds"
modified > "-5.minutes"  
modified > "-2.hours"
modified > "-7.days"
modified > "-1.week"
modified > "-3.months"
```

### Absolute Time
Format: `"YYYY-MM-DD"` (quotes required)

```
modified > "2024-01-01"
created < "2023-12-31"
```

### Special Keywords
```
modified >= "today"
modified < "yesterday"
```

## Case Sensitivity

**IMPORTANT**: All string comparisons in detect are case-sensitive. This affects:

- Name/path matching: `name == "README.md"` won't match "readme.md"
- Extension matching: `ext == "MD"` won't match ".md" files
- Contains operator: `name contains "Test"` won't match "test"
- Content searches: `contents contains "TODO"` won't match "todo"

For case-insensitive matching with regex:
```
# Add (?i) flag at the start of regex pattern
name ~= "(?i)readme"     # Matches README, readme, ReadMe, etc.
contents ~= "(?i)todo"   # Matches TODO, todo, Todo, etc.
```

Platform note: While detect's comparisons are case-sensitive, the underlying filesystem may not be (e.g., macOS and Windows are typically case-insensitive).

## Complex Pattern Examples

### Security Scanning
```
# AWS credentials in any file
contents ~= "AKIA[0-9A-Z]{16}"

# Private keys
contents contains "BEGIN RSA PRIVATE KEY"

# Hardcoded passwords in code
ext in [js, py, java] && contents ~= "password\s*=\s*[\"'][^\"']+[\"']"
```

### Code Analysis
```
# Large files with TODO comments
ext in [js, py, rs] && size > 100000 && contents ~= "//\s*TODO|#\s*TODO"

# Python files with multiple classes
ext == py && !path contains __pycache__ && contents ~= "class\s+\w+"

# JavaScript files importing React
ext in [js, jsx] && !path contains node_modules && contents ~= "import.*React|from.*react"
```

### Project Maintenance
```
# Stale test files
name contains test && modified < "-90.days"

# Config files that might need review
(name ~= "config|settings" || ext in [yml, yaml, json]) && modified < "-180.days"

# Large generated files
size > 1000000 && (name contains generated || path contains "/dist/")
```

## Performance Optimization

### Query Order for Speed
Filters are evaluated left-to-right with short-circuiting:

```
# FAST: Path/name filters eliminate files immediately
ext == js && !path contains node_modules && contents contains TODO

# SLOW: Searches all file contents before filtering
contents contains TODO && ext == js && !path contains node_modules
```

### Three Tiers of Performance
```
# Tier 1 (instant): Path/name/extension checks
name contains test && ext == py

# Tier 2 (fast): Metadata checks  
ext == log && size > 100000000 && modified < "-30.days"

# Tier 3 (slow): Content searches
ext == rs && size < 10000 && contents contains "fn main"
```

### Real-World Performance Patterns
```
# Skip build artifacts first
ext == py && !path contains "build/" && !path contains ".egg" && contents contains import

# Target specific files before content search
name ~= "webpack\.config" && modified > "-7.days" && contents contains "devServer"

# Combine metadata to narrow search space
ext in [yml, yaml] && size < 50000 && contents contains "version:"
```

### Limit Scope
```
# Search in specific directory
detect 'name contains test' /src/tests
```

## Common Pitfalls

### Regex vs Literal Matching
```
# WRONG: Trying to use shell wildcard syntax
name ~= "*.rs"  

# RIGHT: Proper regex
name ~= "\.rs$"

# RIGHT: Or just use extension selector
ext == rs
```

### Quote Usage
```
# Quotes needed for:
- Strings with spaces: name == "my file.txt"
- Temporal expressions: modified > "-7.days"
- Regex with spaces: contents ~= "class \w+"

# Quotes optional for:
- Simple values: type == file
- Single words: ext == rs
```

### Content Search Limitations
- Content search works on text files
- Binary files are skipped
- Large files may be slower
- Use file metadata filters first when possible

## Troubleshooting

### No Results?
1. Check quote usage
2. Verify regex syntax
3. Try simpler query first
4. Use absolute paths

### Unexpected Results?
1. Check operator precedence
2. Add parentheses for clarity
3. Test each condition separately
4. Remember: patterns are case-sensitive

### Performance Issues?
1. Add metadata filters first
2. Limit search scope with path
3. Avoid broad content searches
4. Use specific file extensions