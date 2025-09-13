SYNTAX: selector operator value OR glob pattern

## Glob Patterns (Shell-Style)

For users familiar with shell patterns, detect supports glob syntax as an alternative to predicates:

### Basic Glob Examples
*.rs                                  # All Rust files
*.{js,ts,jsx,tsx}                     # JavaScript/TypeScript files  
*test*                                # Files with "test" in name
src/*.rs                              # Rust files in src directory
**/*.md                               # Markdown files recursively
test_*.txt                            # Files starting with "test_"
[ab]*.rs                              # Files starting with 'a' or 'b'

### Glob + Boolean Logic
*.rs && size > 1kb                    # Large Rust files
*test* || content contains TODO       # Test files or files with TODOs
NOT *.md                              # Non-markdown files
(*.rs || *.toml) && modified > -7d    # Recent Rust or config files

### When to Use Globs vs Predicates
- **Globs**: Quick shell-style patterns, familiar syntax
- **Predicates**: More powerful (content search, size, dates), composable

## Clean 12-Selector System

### File Identity (What is it?)
name - full filename with extension (e.g., "README.md")  
ext - file extension without dot (e.g., "md")
path - full absolute path
dir - parent directory path

**Note**: `basename` selector may not be available in current parser

### File Properties (How big/what kind?)
size - file size in bytes (supports: 45kb 1mb 2gb)
type - file/directory/symlink/socket/fifo
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

### Finding Files by Extension (Two Ways)
ext == rs                                 # Rust source files (predicate)
*.rs                                      # Rust source files (glob)
ext in [js,ts,jsx,tsx]                   # JavaScript/TypeScript files (predicate)
*.{js,ts,jsx,tsx}                        # JavaScript/TypeScript files (glob)

### Content Search (Predicates Only)
content contains TODO                     # Files with TODO comments (literal)
content ~= "class.*Service"              # Classes ending with Service (regex)
content ~= "@(Component|Injectable)"     # Angular decorators (regex)

### Size and Time Filters  
size > 1mb                               # Large files
size > 100kb AND modified > -7d          # Large recent files
modified > -1h                           # Files changed in last hour

### Combining Approaches
*.rs && content ~= async                 # Rust files with async (glob + predicate)
*test* || content contains TODO          # Test files OR TODO files
NOT *.md && size > 1kb                   # Large non-markdown files

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
find . -name "*.js" -size +1M → detect '*.js && size > 1mb' OR detect 'ext == js AND size > 1mb'
find . -type f -exec grep -l TODO {} \; → detect 'content contains TODO'
find . -name "*test*" → detect '*test*' OR detect 'name contains test'

## Syntax Notes:
- Quotes required for whitespace/special chars
- Regex: escape dots \., use anchors ^$
- Case-sensitive string comparisons
- Set items: comma-separated [a,b,c]
- Boolean precedence: NOT > AND > OR
- Performance: name/metadata filters before content

## Troubleshooting:
- No results? Try broader criteria or check syntax
- Use -i flag to include gitignored files
- For large searches, start with name/size filters
- Test complex regex patterns separately first