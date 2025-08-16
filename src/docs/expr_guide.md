SYNTAX: selector operator value

## Clean 12-Selector System

### File Identity (What is it?)
name - full filename with extension (e.g., "README.md")
basename - filename without extension (e.g., "README")
ext - file extension without dot (e.g., "md")
path - full absolute path
dir - parent directory path

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
String: == != ~= contains in
Numeric: == != > < >= <=
Temporal: == != > < (before/after)

## Examples

## Boolean Logic
Operators: AND OR NOT ()
Usage: combine and group expressions

## Examples:
ext == rs AND content ~= async           # Rust files with async
size > 1mb AND modified > -7d            # Large recent files  
content contains TODO AND NOT path ~= test # TODOs outside tests
type == file AND basename == Makefile    # Build files only
basename ~= "\.service$" AND ext == ts AND NOT content contains test
content ~= "@(Injectable|Component)" AND size > 10kb
(content contains TODO OR content contains FIXME) AND modified > -7d
basename == README AND size < 1kb        # Small README files

## Migration from find:
find . -name "*.js" -size +1M → detect 'ext == js AND size > 1mb'
find . -type f -exec grep -l TODO {} \; → detect 'content contains TODO'

## Syntax Notes:
- Quotes required for whitespace/special chars
- Regex: escape dots \., use anchors ^$
- Case-sensitive string comparisons
- Set items: comma-separated [a,b,c]
- Boolean precedence: NOT > AND > OR
- Performance: name/metadata filters before contents