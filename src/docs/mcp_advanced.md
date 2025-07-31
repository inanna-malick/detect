# Detect Advanced Reference

## Complete Operator Set

**Core principle**: Any operator works with any selector of compatible type.

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
filename ~= "^test_"           - Files starting with test_
filename ~= "\.rs$"            - Files ending with .rs
filename ~= "v\d+\.\d+\.\d+"   - Version patterns like v1.2.3
contents ~= "fn\s+main"        - Rust main functions
fullpath ~= ".*/src/.*\.rs$"   - Rust files in src directories
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
!filename contains test && ext == rs || size > 1000
# Parses as: ((!filename contains test) && (ext == rs)) || (size > 1000)

ext == rs && size > 1000 || filename contains test
# Parses as: ((ext == rs) && (size > 1000)) || (filename contains test)
```

Use parentheses to override precedence:
```
!(filename contains test && ext == rs)
ext == rs && (size > 1000 || filename contains test)
(contents contains TODO || contents contains FIXME) && (size > 10000 || modified > "-1.day")
```

## All Selectors

### String Selectors
- `basename`, `base` - Filename without extension
- `filename`, `file` - Complete filename with extension
- `dirpath`, `dir` - Directory path only
- `fullpath`, `full` - Complete path including filename
- `ext`, `extension` - File extension (without dot)
- `contents`, `file` - Search file contents

### Number Selectors
- `size`, `filesize` - Size in bytes

### Time Selectors
- `modified`, `mtime` - Modification time
- `created`, `ctime` - Creation time
- `accessed`, `atime` - Access time

### Enum Selectors
- `type`, `filetype` - Entity type (file, dir, symlink)

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

- Name/path matching: `filename == "README.md"` won't match "readme.md"
- Extension matching: `ext == "MD"` won't match ".md" files
- Contains operator: `filename contains "Test"` won't match "test"
- Content searches: `contents contains "TODO"` won't match "todo"

For case-insensitive matching with regex:
```
# Add (?i) flag at the start of regex pattern
filename ~= "(?i)readme"     # Matches README, readme, ReadMe, etc.
contents ~= "(?i)todo"       # Matches TODO, todo, Todo, etc.
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
ext == py && !dirpath contains __pycache__ && contents ~= "class\s+\w+"

# JavaScript files importing React
ext in [js, jsx] && !dirpath contains node_modules && contents ~= "import.*React|from.*react"
```

### Project Maintenance
```
# Stale test files
filename contains test && modified < "-90.days"

# Config files that might need review
(filename ~= "config|settings" || ext in [yml, yaml, json]) && modified < "-180.days"

# Large generated files
size > 1000000 && (filename contains generated || dirpath contains "/dist/")
```

## Performance Optimization

### Query Order for Speed
Filters are evaluated left-to-right with short-circuiting:

```
# FAST: Path/name filters eliminate files immediately
ext == js && !dirpath contains node_modules && contents contains TODO

# SLOW: Searches all file contents before filtering
contents contains TODO && ext == js && !dirpath contains node_modules
```

### Three Tiers of Performance
```
# Tier 1 (instant): Path/name/extension checks
filename contains test && ext == py

# Tier 2 (fast): Metadata checks  
ext == log && size > 100000000 && modified < "-30.days"

# Tier 3 (slow): Content searches
ext == rs && size < 10000 && contents contains "fn main"
```

### Real-World Performance Patterns
```
# Skip build artifacts first
ext == py && !dirpath contains "build/" && !dirpath contains ".egg" && contents contains import

# Target specific files before content search
filename ~= "webpack\.config" && modified > "-7.days" && contents contains "devServer"

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