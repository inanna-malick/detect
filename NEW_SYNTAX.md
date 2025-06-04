# New Progressive Syntax for Detect

The new syntax has three levels of complexity:

## Level 1: Simple Patterns (90% of use cases)

```bash
# Bare words search content
detect TODO
detect FIXME

# Glob patterns search filenames
detect '*.rs'
detect 'src/**/*.js'

# Quoted strings for exact matches
detect "hello world.txt"

# Regex patterns for advanced matching
detect '/TODO.*urgent/i'
```

## Level 2: Filtered Searches (9% of use cases)

```bash
# File type shortcuts
detect rust              # All Rust files
detect python TODO       # Python files containing TODO

# Size filters
detect '*.rs >1MB'       # Large Rust files
detect 'image <100KB'    # Small images

# Time filters
detect 'modified:today'  # Files modified today
detect '*.log m:1d'      # Logs from last day

# Path filters
detect 'in:src *.py'     # Python files in src/
detect 'dir:tests TODO'  # TODOs in tests/

# Property filters
detect executable        # Executable files
detect hidden           # Hidden files
detect empty            # Empty files
```

## Level 3: Full Expressions (1% of use cases)

```bash
# Boolean logic
detect '*.rs and TODO'
detect 'hidden or empty'
detect 'not binary'

# Complex queries
detect '(*.rs or *.go) and size > 1MB'
detect 'name == "test.rs" or contains(/test/)'

# Predicates
detect 'size > 1000 and lines < 100'
detect 'ext == rs and not contains(/unsafe/)'
```

## Key Improvements

1. **Progressive complexity** - Start simple, add power when needed
2. **Natural syntax** - `rust TODO` instead of `@type == rust && @contents ~= TODO`
3. **Smart defaults** - Bare words search content, patterns search names
4. **Type shortcuts** - `rust` instead of `@extension == rs`
5. **Human-friendly units** - `1MB`, `2d` instead of raw bytes/seconds

## Implementation Status

✅ Grammar definition
✅ Parser implementation  
✅ AST types
✅ Basic tests
🚧 Full integration with existing eval engine
🚧 Time filter implementation
🚧 Additional predicates (mime, binary detection, etc)