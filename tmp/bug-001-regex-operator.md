# Bug Report: Regex Operator (~=) Not Matching Expected Patterns

## Summary
The regex operator `~=` appears to have matching issues where valid regex patterns return no results when they should match files.

## Environment
- detect version: Latest (with @ prefix removal)
- Tested via: MCP server interface

## Description
During beta testing, regex patterns that should match files are returning empty results. This affects the usability of advanced pattern matching.

## Steps to Reproduce
1. Create test files:
   ```
   test.rs
   test_foo.rs
   test_bar.rs
   ```

2. Run query: `name ~= "test.*\.rs$"`

3. Expected: Should match all three files
4. Actual: Returns no results

## Additional Test Cases That Failed
- `name ~= "^test_"` - Should match test_foo.rs and test_bar.rs
- `contents ~= "fn\s+main"` - Should match files with main functions

## Root Cause Analysis
Investigation revealed multiple issues:

1. **Unquoted patterns with special characters fail to parse**: Patterns like `name ~= test.*\.rs$` fail because the grammar's `bare_char` rule was missing backslash `\` and square brackets `[` `]`.

2. **Error messages still reference old @ syntax**: When patterns fail, the error help shows examples with `@name` instead of `name`.

3. **Name matching behavior**: The `NamePredicate` matches against both the full filename AND the stem (filename without extension), which can cause unexpected matches.

## Fix Applied
1. Updated `src/expr/expr.pest` to include `\`, `[`, and `]` in `bare_char`
2. Updated `src/error_hints.rs` to remove @ prefix from examples
3. Enabled previously ignored regex tests

## Current Status
- ✅ Quoted patterns work: `name ~= "test.*\.rs$"`
- ✅ Unquoted patterns now work: `name ~= test.*\.rs$`
- ✅ Error messages updated to show correct syntax
- ⚠️  Name matching against stem may cause unexpected behavior (separate issue)

## Priority
High - This is a documented feature that's broken