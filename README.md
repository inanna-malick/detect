## detect: a command line tool for finding filesystem entities using expressions


```shell
➜  detect 'name ~= detect || ext ~= rs && contents ~= map_frame'
./target/release/detect
./target/release/deps/detect-6395eb2c29a3ed5e
./target/debug/detect
./target/debug/deps/detect-34cec1d5ea27ff11
./target/debug/deps/detect-e91a01500af9a97b
./target/debug/deps/detect-0b57d7084445c8b2
./target/debug/deps/detect-32c3beb592fdbbe3
./src/expr/frame.rs
```

## boolean operators
- `a && b`
- `a || b`
- `!a`
- `(a)`


## string operators
- `==` (or `=`) - exact match (case-sensitive)
- `!=` - not equal
- `contains` - substring search (case-sensitive)
- `~=` (or `~`, `=~`) - regex match
- `in [...]` - set membership

## numeric operators
- `>`, `>=`, `<`, `<=` - comparisons
- `==` (or `=`) - exact match

# Selectors

## file path selectors

- name (or filename) - matches filename only
- path (or filepath) - matches full file path  
- ext (or extension) - file extension without dot

## metadata selectors

- size (or filesize) - file size in bytes
- type (or filetype) - file, dir, or symlink

## file contents predicates

- contents (or file) - search file contents

## temporal selectors

- modified (or mtime) - modification time
- created (or ctime) - creation time
- accessed (atime) - access time

# Usage Examples

## Simple queries
```bash
# Find specific file
detect 'name == README.md'

# Find by extension
detect 'ext == rs'

# Find large files (size in bytes)
detect 'size > 1000000'

# Search file contents
detect 'contents contains TODO'
```

## Complex queries
```bash
# Combine conditions
detect 'ext == js && size > 1024'

# Use sets
detect 'ext in [js, ts, jsx]'

# Temporal queries (quotes required)
detect 'modified > "-7.days"'

# Exclude patterns
detect 'ext == rs && !path contains target'
```

## Important Notes

- **Case Sensitivity**: All string comparisons are case-sensitive
  - `name == "README.md"` ≠ `name == "readme.md"`
- **Quotes**: Required for:
  - Values with spaces: `name == "my file.txt"`
  - Temporal expressions: `modified > "-7.days"`
  - Regex patterns with spaces: `contents ~= "class \\w+"`
- **Regex**: Use `~=` for pattern matching:
  - `name ~= "test.*\\.rs$"` (files starting with "test" ending in ".rs")
- **File size**: Must be specified in bytes (no KB/MB units)
