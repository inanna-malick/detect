# Bug Report: Name Predicate Matches Both Full Filename and Stem

## Summary
The `name` selector matches against both the full filename AND the filename without extension (stem), which causes unexpected behavior with regex patterns.

## Environment
- detect version: Latest
- Affects: All name-based operations

## Description
When using the `name` selector, the predicate matches if either:
1. The full filename matches the pattern, OR
2. The filename without extension (stem) matches the pattern

This dual matching behavior causes issues with regex patterns that expect to match the complete filename.

## Example
```bash
# Given files: main.rs, main.rs.bak

# This regex should only match files ending in .rs
detect 'name ~= ".*\.rs$"'

# Expected: main.rs
# Actual: main.rs, main.rs.bak (because "main.rs" is the stem of "main.rs.bak")
```

## Current Behavior Analysis

### Benefits of Current Behavior
- User convenience: `name in [index, main]` matches "index.js", "main.ts", etc.
- Allows searching by base name without worrying about extensions
- Integration tests rely on this behavior

### Problems with Current Behavior
- Regex patterns that anchor to end of filename ($) don't work as expected
- Can't distinguish between "file.rs" and "file.rs.bak"
- Violates principle of least surprise for regex users

## Potential Solutions

### Option 1: Keep Current Behavior, Document It
- Add clear documentation about stem matching
- Suggest workarounds for users who need exact matching

### Option 2: Only Apply Stem Matching to Non-Regex Operators
- Keep stem matching for: ==, !=, contains, in
- Use only full filename for: ~= (regex)
- **Breaking Change**: Some existing queries would change behavior

### Option 3: Add New Selector
- Keep `name` as-is for backward compatibility
- Add `filename` selector that only matches full filename
- Add `stem` selector that only matches stem

## Recommendation
Option 2 would provide the most intuitive behavior, but it's a breaking change. Option 3 provides a migration path without breaking existing queries.

## Priority
Medium - This is a design decision that affects the API. Current behavior works but can be surprising.