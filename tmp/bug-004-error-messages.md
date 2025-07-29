# Bug Report: Error Messages Need Improvement

## Summary
Error messages from the detect MCP tool are not helpful enough for users to understand and fix their queries.

## Environment
- detect version: Latest
- Affects all error responses through MCP interface

## Description
Current error messages don't provide enough context or suggestions for users to correct their queries. This leads to frustration and repeated failed attempts.

## Partial Fix Applied
- ✅ Updated error hints to remove @ prefix from examples (fixed in commit 2098aa2)
- ✅ Added all selector aliases to error hints (filename, filepath, etc.)
- ✅ Added all operator aliases to error hints (=, ~, =~)
- ✅ Added case sensitivity note to error hints
- ❌ Still need more contextual error messages as suggested below

## Examples of Poor Error Messages

### Current:
```
"Parse error"
"Invalid expression"
"No results"
```

### Suggested Improvements:
```
"Parse error: Expected operator after 'name', got 'test.rs'. Did you mean 'name == test.rs'?"
"Invalid expression: Unknown selector 'filename'. Did you mean 'name'?"
"No results found. Try broadening your search or checking the path."
```

## Specific Improvements Needed

### 1. Parser Errors
- Show the position of the error
- Suggest valid operators for the context
- List valid selectors if unknown selector used

### 2. No Results
- Suggest removing filters to broaden search
- Mention if the directory is empty
- Suggest checking case sensitivity

### 3. Invalid Values
- Show valid format for temporal expressions
- Explain regex syntax errors
- Show valid values for enum-like fields (e.g., type)

### 4. Performance Warnings
- Warn when content search has no pre-filters
- Suggest adding path/extension filters
- Indicate when query might be slow

## Example Implementation

```rust
match parse_error {
    UnknownSelector(s) => {
        let suggestions = suggest_similar_selectors(s);
        format!("Unknown selector '{}'. Did you mean: {}?", s, suggestions.join(", "))
    }
    MissingOperator(selector) => {
        format!("Expected operator after '{}'. Valid operators: ==, contains, >, in [...]", selector)
    }
    // ... etc
}
```

## User Impact
- Reduced time to successful query
- Better learning experience
- Less frustration
- Fewer support requests

## Priority
Medium - Significantly improves user experience but tool is still functional