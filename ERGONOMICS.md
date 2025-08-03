# Ergonomic Improvements in detect

This document describes the ergonomic features that make `detect` pleasant to use for both humans and LLMs.

## 1. Case-Insensitive Size Units ✅

All size units work regardless of case:
- `size > 1kb` = `size > 1KB` = `size > 1Kb` = `size > 1kB`
- `size > 1mb` = `size > 1MB` = `size > 1Mb` = `size > 1mB`
- `size > 1gb` = `size > 1GB` = `size > 1Gb` = `size > 1gB`

Short forms also work case-insensitively:
- `size > 1k` = `size > 1K`
- `size > 1m` = `size > 1M`
- `size > 1g` = `size > 1G`

## 2. Relaxed Temporal Syntax ✅

Multiple ways to express time:

**Modern syntax (no quotes or periods needed):**
- `modified > 7days`
- `created < 30minutes` 
- `accessed > -1hour`
- `modified > -2weeks`

**Short forms:**
- `modified > 7d`
- `created < 30m`
- `accessed > 1h`
- `modified > 2w`

**Legacy syntax (still supported):**
- `modified > -7.days`
- `created < -30.minutes`

**Keywords:**
- `modified > yesterday`
- `created == today`
- `accessed < now`

## 3. Intuitive Aliases ✅

### `filename` alias for `path.name`
These are equivalent:
- `filename == "test.rs"`
- `path.name == "test.rs"`

Use whichever feels more natural. `filename` is especially intuitive for new users.

### Extension shortcuts
- `extension` or `ext` for `path.extension`
- `name` for `path.name`
- `parent` for `path.parent`
- `stem` for `path.stem`

## 4. Flexible Set Syntax ✅

Sets are extremely flexible with spacing and quotes:

**All these are equivalent:**
```bash
extension in [js,ts,jsx]           # No spaces
extension in [js, ts, jsx]         # With spaces
extension in [ js , ts , jsx ]     # Extra spaces
extension in ["js", "ts", "jsx"]   # With quotes
extension in ['js', 'ts', 'jsx']   # Single quotes
extension in [js, "ts", 'jsx']     # Mixed (why not?)
```

**Multi-line sets for readability:**
```bash
extension in [
    js,
    ts,
    jsx,
    tsx
]
```

**Special characters work fine:**
```bash
name in [.gitignore, .eslintrc, .prettierrc]
name in [file-1.txt, file_2.txt, file#3.txt]
```

## 5. Smart Regex Warnings ✅

Helpful warnings for common regex mistakes:

### Empty regex warning
```bash
contents ~= ""
# Warning: Empty regex pattern will match every line in every file
```

### Unescaped dot warning
```bash
name ~= .ts
# Warning: Pattern '.ts' has an unescaped dot which matches any character.
# For file extensions, consider using 'path.extension == ts' or escape the dot: '\.ts'
```

### File extension suggestion
```bash
name ~= js
# Warning: Pattern 'js' might not match as expected.
# For file extensions, use 'path.extension == js' or a proper regex like '\.(js)'
```

### Glob pattern correction
The `*` wildcard is automatically converted to `.*` for convenience.

## 6. Case-Insensitive Word-Form Boolean Operators ✅

Choose your style - symbols or words, any case:

**Equivalent expressions:**
- `name == foo && size > 100`
- `name == foo and size > 100`
- `name == foo AND size > 100`
- `name == foo And size > 100`

- `name == foo || name == bar`
- `name == foo or name == bar`
- `name == foo OR name == bar`

- `!type == dir`
- `not type == dir`
- `NOT type == dir`
- `Not type == dir`

**All case variants work:**
```bash
# These are all equivalent:
name == README and not type == dir && size > 1kb
name == README AND NOT type == dir && size > 1kb
name == README And Not type == dir && size > 1kb
name == README aNd nOt type == dir && size > 1kb
```

**SQL familiarity:**
Users coming from SQL can use uppercase `AND`, `OR`, `NOT` as expected.

## 7. Natural Error Messages

Clear, actionable error messages with context:

```bash
detect 'name ~= [unclosed'
# Error: Invalid regex at position 8
# name ~= [unclosed
#         ^^^^^^^^^
# regex parse error: unclosed character class
# Tip: Check your regex at regex101.com
```

## Design Philosophy

These improvements follow the principle that **good ergonomics means meeting users where they are**, not forcing them to adapt to the tool. Whether you:
- Type `MB` out of habit from other tools
- Forget quotes around temporal values
- Use `filename` because it's more intuitive than `path.name`
- Mix quote styles in sets
- Write `and` instead of `&&`

...detect will understand what you mean and do the right thing.

## VIBES Assessment

Using the VIBES framework for LLM ergonomics:
- **Expressive Power**: 🔬 (Crystalline) - Multiple valid ways to express the same query
- **Context Flow**: 🪢 (Pipeline) - Clear, linear query structure
- **Error Surface**: 💧 (Liquid) - Errors handled gracefully with helpful guidance

The combination of semantic aliases, flexible syntax, and smart warnings makes detect highly ergonomic for both human and LLM users.