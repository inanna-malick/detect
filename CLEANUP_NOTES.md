# Cleanup Notes - Deprecated Syntax

## Summary
Most of the codebase has been updated to use the new syntax. The documentation, tests, and examples are all using the new LLM-friendly syntax without `@` prefixes.

## Found Issues

### 1. Dead code in predicate.rs
- `RawPredicate` struct (lines 14-18) - appears to be from old parser, only used within predicate.rs
- Old `Selector` enum (lines ~50-57) - superseded by new Selector in query.rs
- `parse` method on RawPredicate - converts old format to new

### 2. Potentially unused parser functions
- Various parse functions in predicate.rs that work with the old RawPredicate

## Already Clean

### Documentation
- ✅ README.md - uses new syntax in all examples
- ✅ CLAUDE.md - updated with new syntax examples
- ✅ NEW_SYNTAX.md - documents the new syntax (mentions old syntax only for comparison)

### Code
- ✅ main.rs - fully updated to new CLI interface
- ✅ parser.rs - parses new syntax
- ✅ query.rs - new AST types for new syntax
- ✅ lib.rs - works with parsed expressions

### Tests
- ✅ test_queries.sh - all examples use new syntax
- ✅ llm_syntax_tests.rs - tests for new syntax
- ✅ new_syntax.rs - tests for new parser
- ✅ parser_tests.rs - updated tests

## Recommendation
The main cleanup needed is removing the old RawPredicate code in predicate.rs since it appears to be dead code from the previous parser implementation.