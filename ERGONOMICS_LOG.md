# Detect Tool Ergonomics Log

This log tracks usability issues discovered while using the detect tool.

## Issues Found

### 1. Parentheses in patterns cause parsing errors
**Problem**: Trying to search for `unwrap()` or `.unwrap()` fails with parsing errors.
```bash
detect 'unwrap()'  # Error: expected EOI, filter, or_op, or and_op
```
**Workaround**: Search for `unwrap` without parentheses
**Fix needed**: Better handling of special characters in bare word patterns

### 2. Path confusion when piping to system detect vs local build
**Problem**: When using pipes, the system-installed `detect` may be used instead of `./target/debug/detect`
```bash
./target/debug/detect TODO | xargs detect  # Uses wrong version
```
**Workaround**: Use full path or update PATH
**Fix needed**: Maybe add a version flag to help debug?

### 3. Special characters in patterns need escaping or quoting
**Problem**: Patterns with dots, parentheses, etc. need careful handling
```bash
detect .unwrap()   # Fails
detect '\.unwrap'  # Works in regex mode?
```
**Fix needed**: Clearer documentation on when escaping is needed

### 4. Git range shows ALL files that match, not just added lines
**Problem**: `--git-range HEAD~10..HEAD TODO` shows files with TODO that changed, but doesn't indicate if the TODO was added or just the file was modified
**Enhancement**: Option to show only newly added matches, not all matches in changed files

### 5. No way to see matched lines, only filenames
**Problem**: Tool only outputs filenames, not the actual matching lines
**Enhancement**: Add `-n` flag to show line numbers and/or `-C` for context lines like grep

### 6. Expression mode still uses some non-intuitive syntax
**Problem**: In expression mode, need to know exact predicate names
```bash
detect -e 'name = "test.rs"'  # Works
detect -e 'filename = "test.rs"'  # Doesn't work - need to know it's "name"
```
**Enhancement**: Add aliases for common predicates

### 7. Regex patterns in simple mode aren't obvious
**Problem**: It's not clear when `/pattern/` syntax works vs needs expression mode
```bash
detect /TODO/      # Does this work in simple mode?
detect '/TODO/'    # Or does it need quotes?
```
**Enhancement**: Document regex handling better

### 8. Mixing simple mode flags with expression mode
**Problem**: When using `-e`, other flags like `--type` seem to be ignored
```bash
detect --type rust -e 'lines > 100'  # --type is ignored
```
**Workaround**: Include file type in expression: `-e 'ext = rs && lines > 100'`
**Fix needed**: Either make flags work with -e or give clear error

### 9. No line count predicate in simple mode
**Problem**: Can't easily search for files with many/few lines without expression mode
**Enhancement**: Add `--lines >100` or similar filter

## Positive Ergonomics

Things that work really well:
- Simple mode defaults are intuitive (bare words = content, globs = filenames)
- File type shortcuts are great
- Git integration is seamless
- Combining filters is natural (`detect --type rust TODO`)
- Git range feature is powerful for code review