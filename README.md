## detect: a command line tool for finding filesystem entities using expressions


```shell
# Find files containing "map_frame" or files with name matching "detect" or .rs extension
➜  detect 'map_frame'
./src/expr/frame.rs

# Find files with name matching pattern "detect"
➜  detect '*detect*'
./target/release/detect
./target/release/deps/detect-6395eb2c29a3ed5e
./target/debug/detect
./target/debug/deps/detect-34cec1d5ea27ff11
./target/debug/deps/detect-e91a01500af9a97b
./target/debug/deps/detect-0b57d7084445c8b2
./target/debug/deps/detect-32c3beb592fdbbe3

# Find Rust files containing "map_frame"
➜  detect '*.rs && map_frame'
./src/expr/frame.rs
```

## Usage

### Simple Patterns (Level 1)
Most common searches are simple:
- Bare words search file contents: `detect TODO`
- Glob patterns search filenames: `detect '*.rs'`
- Regex patterns for advanced matching: `detect '/TODO.*urgent/i'`
- Quoted strings for exact matches: `detect "hello world.txt"`

### Filtered Searches (Level 2)
Add filters for more control:
- File types: `detect rust` or `detect python TODO`
- Size filters: `detect '*.log >1MB'`
- Time filters: `detect 'modified:today'`
- Path filters: `detect 'in:src *.py'`
- Properties: `detect executable` or `detect hidden`

### Full Expressions (Level 3)
For complex queries, use boolean logic and predicates:
- Boolean operators: `&&` (AND), `||` (OR), `!` (NOT)
- Predicates: `name == "test.rs"`, `size > 1000`, `contains(/unsafe/)` (prefer simpler syntax when possible: `"test.rs"`, `>1000`, `unsafe`)
- Combine everything: `detect '(*.rs || *.go) && size > 1MB && !hidden'`

## Examples

```shell
# Find TODO comments
detect TODO

# Find all Rust files
detect '*.rs'

# Find Python files containing TODO
detect 'python TODO'

# Find large images
detect 'image >10MB'

# Find recently modified logs
detect '*.log modified:7d'

# Complex query: large Rust files with unsafe code
detect '*.rs >10KB unsafe'
```
