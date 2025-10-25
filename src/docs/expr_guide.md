SYNTAX: selector operator value OR single-word-alias

## Single-Word Aliases

For common file type queries, detect supports single-word aliases as shorthand:

### File Type Aliases
file                                  # Regular files
dir, directory                        # Directories
symlink, link                         # Symbolic links
socket, sock                          # Unix sockets
fifo, pipe                            # Named pipes (FIFOs)
block, blockdev                       # Block devices
char, chardev                         # Character devices

### Aliases in Boolean Logic
dir && depth > 0                      # Directories below root level
file && size > 1mb                    # Large regular files
NOT symlink                           # Exclude symlinks
(file || dir) && modified > -7d       # Recent files or directories

### When to Use Aliases vs Predicates
- **Aliases**: Quick, concise for common patterns
- **Predicates**: Explicit, composable with all operators

### Equivalence
dir                  ===  type == dir
file && size > 1mb   ===  type == file AND size > 1mb

## Clean 12-Selector System

### File Identity (What is it?)
name - full filename with extension (e.g., "README.md")
ext - file extension without dot (e.g., "md")
path - full absolute path
dir - parent directory path

### File Properties (How big/what kind?)
size - file size in bytes (supports: 45kb 1mb 2gb)
type - file type (parse-time validated)
  Valid values: file, dir/directory, symlink/link, socket/sock, fifo/pipe, block/blockdev, char/chardev
depth - directory depth from search root

### Time (When did things happen?)
modified - last modification time
created - creation/birth time
accessed - last access time

### Content (What's inside?)
content - file text content

## Operators by Type

### String Operators (for name, content, path, type)
- `==` / `!=` - Exact match / not equal
- `contains` - Substring search (literal text)
- `~=` - Regex pattern matching
- `in [a,b,c]` - Match any item in set

### Numeric Operators (for size, depth)
- `==` / `!=` - Exact value / not equal
- `>` / `<` / `>=` / `<=` - Comparisons

### Temporal Operators (for modified, created, accessed)
- `>` / `<` - After / before (relative: `-7d`, absolute: `2024-01-15`)
- `==` / `!=` - Exact time match

### Key Operator Distinctions
- **Literal text**: Use `contains` → `content contains TODO`
- **Pattern matching**: Use `~=` → `name ~= "test.*\.rs$"`
- **Multiple options**: Use `in` → `ext in [js,ts,jsx]`

## Examples

## Boolean Logic
Operators: AND OR NOT ()
Usage: combine and group expressions

## Common Examples:

### Finding Files by Type
file                                     # Regular files only
dir && depth > 0                         # Subdirectories
symlink                                  # Symbolic links
ext == rs                                # Rust source files
ext in [js,ts,jsx,tsx]                   # JavaScript/TypeScript files

### Content Search
content contains TODO                     # Files with TODO comments (literal)
content ~= "class.*Service"              # Classes ending with Service (regex)
content ~= "@(Component|Injectable)"     # Angular decorators (regex)

### Size and Time Filters
size > 1mb                               # Large files
file && size > 100kb AND modified > -7d  # Large recent regular files
modified > -1h                           # Files changed in last hour

### Combining Aliases and Predicates
file && ext == rs && content ~= async    # Async code in Rust files
dir && name contains test                # Test directories
NOT (symlink || socket) && file          # Regular files only (exclude special)

### Complex Queries
ext == rs AND content ~= async           # Rust files with async
content contains TODO AND NOT path ~= test # TODOs outside tests
type == file AND name == Makefile        # Build files only
(content contains TODO OR content contains FIXME) AND modified > -7d
name == README AND size < 1kb            # Small README files

### Advanced Patterns
basename ~= "\.service$" AND ext == ts AND NOT content contains test
content ~= "@(Injectable|Component)" AND size > 10kb
path ~= "src/" AND ext == rs AND NOT content contains "test"

## Migration from find:
find . -name "*.js" -size +1M → detect 'ext == js AND size > 1mb'
find . -type f -exec grep -l TODO {} \; → detect 'file && content contains TODO'
find . -name "*test*" → detect 'name contains test'
find . -type d → detect 'dir'

## Syntax Notes:
- Quotes required for whitespace/special chars
- Regex: escape dots \., use anchors ^$
- Case-sensitive string comparisons
- Set items: comma-separated [a,b,c]
- Boolean precedence: NOT > AND > OR
- Performance: name/metadata filters before content
- Aliases are case-insensitive: `FILE` == `file` == `File`

## Troubleshooting:
- No results? Try broader criteria or check syntax
- Use -i flag to include gitignored files
- For large searches, start with name/size filters
- Test complex regex patterns separately first
- Unknown alias error? Use `type == value` instead
