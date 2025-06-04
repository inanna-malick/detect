#!/bin/bash

# Test script for detect tool with new progressive syntax
# Each test has: description, command, expected behavior

echo "Testing detect tool with progressive syntax..."
echo "============================================"

run_test() {
    local desc="$1"
    local cmd="$2"
    local expected="$3"
    
    echo ""
    echo "TEST: $desc"
    echo "CMD:  $cmd"
    echo "EXPECT: $expected"
    echo "RESULT:"
    eval "$cmd" | head -5
    echo "---"
}

# Level 1: Simple pattern tests
run_test "Find TODO comments" \
    "detect TODO" \
    "Files containing the word TODO"

run_test "Find FIXME comments" \
    "detect FIXME" \
    "Files containing the word FIXME"

run_test "Find all Rust files" \
    "detect '*.rs'" \
    "All files with .rs extension"

run_test "Find all test files" \
    "detect '*test*'" \
    "Files with 'test' in the name"

run_test "Find integration tests" \
    "detect 'tests/*.rs'" \
    "Rust files in tests directory"

run_test "Find all markdown files" \
    "detect '*.md'" \
    "All markdown documentation"

run_test "Find pest grammar files" \
    "detect '*.pest'" \
    "Pest parser grammar files"

run_test "Find main entry point" \
    "detect main.rs" \
    "The main.rs file"

run_test "Find regex patterns" \
    "detect '/impl.*From/'" \
    "Files containing impl...From pattern"

run_test "Find parse functions" \
    "detect 'parse_'" \
    "Files containing parse_ prefix"

# Level 2: Filtered searches
run_test "Large Rust files" \
    "detect '*.rs >10KB'" \
    "Rust files larger than 10KB"

run_test "Small Rust files" \
    "detect '*.rs <1KB'" \
    "Rust files smaller than 1KB"

run_test "Rust files with TODO" \
    "detect 'rust TODO'" \
    "Rust files containing TODO"

run_test "Python files (should be none)" \
    "detect python" \
    "No Python files expected"

run_test "Executable files" \
    "detect executable" \
    "Files with execute permission"

run_test "Hidden files" \
    "detect hidden" \
    "Files starting with dot"

run_test "Empty files" \
    "detect empty" \
    "Files with zero size"

run_test "Files in src directory" \
    "detect 'in:src *.rs'" \
    "Rust files under src/"

run_test "Test files in tests dir" \
    "detect 'dir:tests test'" \
    "Files with 'test' in tests/"

run_test "Large files anywhere" \
    "detect '>100KB'" \
    "Any file larger than 100KB"

# Level 3: Expression tests
run_test "Rust OR Go files" \
    "detect '*.rs or *.go'" \
    "Files ending in .rs or .go"

run_test "Test files AND Rust" \
    "detect '*test* and *.rs'" \
    "Rust files with test in name"

run_test "NOT hidden files" \
    "detect 'not hidden'" \
    "Files that aren't hidden"

run_test "Large Rust with TODO" \
    "detect '*.rs and >5KB and TODO'" \
    "Large Rust files with TODO"

run_test "Contains unsafe code" \
    "detect 'contains(/unsafe/)'" \
    "Files with unsafe keyword"

run_test "Name equals predicate" \
    "detect 'name == parser.rs'" \
    "File named exactly parser.rs"

run_test "Size comparison" \
    "detect 'size > 1000'" \
    "Files larger than 1000 bytes"

run_test "Extension check" \
    "detect 'ext == rs'" \
    "Files with rs extension"

# Pattern variations
run_test "Glob with directory" \
    "detect 'src/**/*.rs'" \
    "All Rust files under src recursively"

run_test "Multiple extensions" \
    "detect '*.{rs,toml}'" \
    "Rust and TOML files"

run_test "Question mark glob" \
    "detect '?.rs'" \
    "Single character .rs files"

run_test "Character class glob" \
    "detect '[mt]*.rs'" \
    "Files starting with m or t"

# Content searches
run_test "Import statements" \
    "detect 'use std'" \
    "Files with std imports"

run_test "Function definitions" \
    "detect 'fn '" \
    "Files with function definitions"

run_test "Struct definitions" \
    "detect struct" \
    "Files defining structs"

run_test "Enum definitions" \
    "detect enum" \
    "Files defining enums"

run_test "Impl blocks" \
    "detect impl" \
    "Files with impl blocks"

run_test "Async functions" \
    "detect async" \
    "Files with async code"

run_test "Match expressions" \
    "detect match" \
    "Files with match expressions"

run_test "Error handling" \
    "detect 'Result<'" \
    "Files using Result type"

run_test "Option type" \
    "detect 'Option<'" \
    "Files using Option type"

run_test "Unwrap calls" \
    "detect unwrap" \
    "Files with unwrap calls"

# Combined filters
run_test "Test files over 5KB" \
    "detect '*test* >5KB'" \
    "Test files larger than 5KB"

run_test "Small markdown files" \
    "detect '*.md <10KB'" \
    "Small documentation files"

run_test "Rust in src directory" \
    "detect 'in:src rust'" \
    "Rust files in src folder"

run_test "TODO in test files" \
    "detect '*test* TODO'" \
    "Test files with TODOs"

# More complex expressions
run_test "Parse or Expr" \
    "detect 'parse or Expr'" \
    "Files with parse or Expr"

run_test "Not empty Rust files" \
    "detect '*.rs and not empty'" \
    "Non-empty Rust files"

run_test "Hidden or binary" \
    "detect 'hidden or binary'" \
    "Hidden or binary files"

# Case sensitivity tests
run_test "Case sensitive TODO" \
    "detect TODO" \
    "Files with uppercase TODO"

run_test "Lowercase todo" \
    "detect todo" \
    "Files with lowercase todo"

# Special characters
run_test "Question in content" \
    "detect '?'" \
    "Files containing question mark"

run_test "Exclamation in content" \
    "detect '!'" \
    "Files containing exclamation"

# Path-based searches
run_test "Files in current dir only" \
    "detect 'in:. *.rs'" \
    "Rust files in root directory"

run_test "Nested test search" \
    "detect 'tests/**/*test*.rs'" \
    "Test files in tests subdirs"

# Quoted string tests
run_test "Exact filename match" \
    "detect '\"main.rs\"'" \
    "Exact main.rs match"

run_test "Space in search" \
    "detect '\"use std\"'" \
    "Exact 'use std' match"

# Property tests
run_test "Binary files" \
    "detect binary" \
    "Binary files"

run_test "Text files" \
    "detect text" \
    "Text files"

# More content patterns
run_test "Panic calls" \
    "detect 'panic!'" \
    "Files with panic! macro"

run_test "Tests modules" \
    "detect '#[test]'" \
    "Files with test attributes"

run_test "Derive macros" \
    "detect '#[derive'" \
    "Files using derive"

run_test "Comments with slashes" \
    "detect '//'" \
    "Files with // comments"

# Numeric comparisons
run_test "Exactly 0 bytes" \
    "detect 'size == 0'" \
    "Empty files"

run_test "Under 100 bytes" \
    "detect 'size < 100'" \
    "Very small files"

run_test "KB size files" \
    "detect '>1KB <10KB'" \
    "Files between 1-10KB"

# Complex boolean logic
run_test "Rust but not test" \
    "detect '*.rs and not *test*'" \
    "Non-test Rust files"

run_test "TODO or FIXME" \
    "detect 'TODO or FIXME'" \
    "Files with either marker"

run_test "Multiple conditions" \
    "detect '(*.rs or *.toml) and >1KB'" \
    "Large Rust or TOML files"

# Regex tests
run_test "Regex with flags" \
    "detect '/todo/i'" \
    "Case-insensitive todo search"

run_test "Complex regex" \
    "detect '/fn\s+\w+/'" \
    "Function definitions pattern"

# Edge cases
run_test "Very large files" \
    "detect '>1MB'" \
    "Files over 1 megabyte"

run_test "Symlinks" \
    "detect symlink" \
    "Symbolic links"

# Final summary test
run_test "Count all files" \
    "detect '*' | wc -l" \
    "Total file count"

echo ""
echo "Test suite completed!"