# Bug Report: Case Sensitivity Behavior Undocumented

## Summary
The case sensitivity behavior of string comparisons is not documented, leading to user confusion and unexpected results.

## Environment
- detect version: Latest
- Affects all string-based selectors and operators

## Description
Users need to know whether string comparisons are case-sensitive, but this information is missing from documentation. This affects:
- name/path selectors
- contains operator
- == operator for strings
- Regex patterns

## Current State
- No documentation mentions case sensitivity
- Users must experiment to discover behavior
- Behavior may vary by platform (filesystem-dependent)

## Examples of Ambiguity
```
name == "README.md" vs name == "readme.md"
name contains "Test" vs name contains "test"
ext == "MD" vs ext == "md"
```

## User Impact
- Failed queries when case doesn't match
- Inconsistent results across platforms
- Time wasted debugging case issues
- May write overly complex queries to handle both cases

## Suggested Documentation Additions

### In Basic Docs:
```
CASE SENSITIVITY:
• File/path names: Depends on your filesystem (case-insensitive on macOS/Windows, case-sensitive on Linux)
• Extension comparisons: Case-insensitive (ext == "md" matches .MD files)
• Content searches: Case-sensitive by default
```

### In Advanced Docs:
- Add section on platform differences
- Show how to handle case-insensitive searches
- Document regex flag options if available

## Priority
Medium - Causes user friction but has workarounds