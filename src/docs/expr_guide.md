SYNTAX: selector operator value

## Path Predicates
Selectors: path.{name,stem,ext,parent,full}
Operators: == != ~= contains in
Examples: "file.rs" [rs,js] *.txt "/src/lib.rs" src/

## Content Predicates  
Selectors: contents
Operators: == != ~= contains
Examples: "TODO" async.*await @(Injectable|Component)

## Size Predicates
Selectors: size
Operators: == != > < >= <= in
Examples: 123 45kb 1mb 2gb 1mb-5gb
Formats: b kb mb gb tb

## Time Predicates
Selectors: modified, created, accessed
Operators: == != > < >= <=
Examples: -7d -1h 2023-01-01 yesterday today now

## Type Predicates
Selectors: type
Operators: == != in
Examples: file dir symlink socket fifo block char

## Boolean Logic
Operators: && || ! ()
Usage: combine and group expressions

## Examples:
path.ext == rs && contents ~= async     # Rust files with async
size > 1mb && modified > -7d            # Large recent files  
contents contains TODO && !path ~= test # TODOs outside tests
type == file && path in [Makefile,*.mk] # Build files only
path.name ~= \.service\.ts$ && !contents contains test
contents ~= @(Injectable|Component) && size > 10kb
(contents contains TODO || contents contains FIXME) && modified > -7d

## Migration from find:
find . -name "*.js" -size +1M → detect 'path.ext == js && size > 1mb'
find . -type f -exec grep -l TODO {} \; → detect 'contents contains TODO'

## Syntax Notes:
- Quotes required for whitespace/special chars
- Regex: escape dots \., use anchors ^$
- Case-sensitive string comparisons
- Set items: comma-separated [a,b,c]
- Boolean precedence: ! > && > ||
- Performance: name/metadata filters before contents