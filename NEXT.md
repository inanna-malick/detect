# Next Features for detect

Based on extensive beta testing and user feedback, these enhancements would significantly improve detect's usability while maintaining its Unix philosophy.

## Phase 1: Core Usability Improvements

### 1. Output Format Control (--format flag)
**Make `relative` the default output format**
- Change default output to relative paths (more readable, less overwhelming)
- Add `--format` flag with options: `relative` (default), `full`, `name`, `jsonl`
- Use JSONL (newline-delimited JSON) for better streaming compatibility
- Format: `{"path":"relative/path","size":10240,"modified":"2024-01-01T10:00:00Z","type":"file"}`
- Benefits: Works better with `jq` and other streaming JSON tools

### 2. Limit Flag (--limit)
- Add `--limit N` to stop after N results
- Prevents need for `detect ... | head` workarounds
- Document clearly: Results may appear in different order between runs
- Use cases:
  - Sampling: `--limit 10` gives you "any 10 matches" (useful for testing)
  - With sort: `--sort size --limit 10` gives you "10 largest" (deterministic)
- Note: No `--skip` due to non-deterministic iteration order

### 3. Enhanced Error Messages
- Detect shell escaping issues (`\!`, `\&`, `\|` patterns)
- Show what user typed vs what parser received after shell processing
- Provide helpful hints for common mistakes
- Implementation approach:
  - Enhance `ParseError::hint()` method in `src/parse_error.rs`
  - Detect backslash patterns that indicate shell escaping
  - Show specific solutions for bash history expansion issues
- Example improved error:
  ```
  Error: Unexpected token '\!' at position 27
  Your command: detect 'path.name == foo && \!type == dir'
  Parser received: "path.name == foo && \\!type == dir"
                                        ^^
  Problem: Bash interpreted \! as escaped exclamation
  
  Solutions:
  1. Use word form: detect 'path.name == foo and not type == dir'
  2. Use double quotes: detect "path.name == foo && !type == dir"
  3. Disable history expansion: set +H
  ```
- Detect removed/renamed selectors:
  ```
  Error: Invalid selector 'suffix' at position 5
  Query: path.extension == txt
              ^^^^^^
  Did you mean: path.extension
  
  Note: 'suffix' was removed. Use 'extension' or 'ext' instead.
  ```

### 4. Exclude Patterns with Config Support
- Add `--exclude pattern1,pattern2` flag
- Support `.detectignore` file in current directory or home
- Default excludes: `node_modules`, `dist`, `target`, `.git`
- Config file format:
  ```toml
  # ~/.config/detect/config.toml or ./.detectignore
  exclude = ["node_modules", "dist", "*.generated.*", "target", "vendor"]
  ```
- Override with `--no-default-excludes` flag
- Benefits: Reduces query length by ~50% for common cases

### 5. Count Flag (--count)
- Add `--count` or `-c` flag to output only the count of matches
- Outputs just a number (nothing else) for easy scripting
- Eliminates need for `| wc -l` pipeline
- Implementation:
  - Add `count: bool` field to CLI args in `src/main.rs`
  - Modify `parse_and_run_fs` in `src/lib.rs` to track count
  - Return count from function
  - Print only count when flag is set
- Example usage:
  ```bash
  detect --count 'extension == rs'  # Output: 47
  detect -c 'size > 10mb'           # Output: 3
  ```
- Code changes:
  ```rust
  // In lib.rs - return count
  pub async fn parse_and_run_fs<F>(
      // ... params ...
  ) -> Result<usize, DetectError>
  
  // In main.rs - handle count flag
  if args.count {
      println!("{}", count);
  }
  ```
- No interaction issues with other flags (none implemented yet)

## Phase 2: Enhanced Functionality

### 6. Sort Operations (--sort flag)
- Add `--sort` with options: `path`, `name`, `size`, `modified`
- Add `--reverse` for descending order
- Enables deterministic output for reproducible results
- Makes `--limit` more useful (e.g., "10 largest files")
- Example: `detect --sort size --reverse --limit 10 'extension == ts'`

### 7. Content Preview (--show-matches flag)
- Display matching lines with context (like grep)
- Format: `path:line_num: matched_content`
- Limit matches per file to avoid spam
- Example output:
  ```
  src/service.ts:42:  // TODO: Add transaction support
  src/other.ts:10:    /* TODO: Implement caching */
  ```

### 8. Stats Flag (--stats)
- Aggregate statistics without needing specific order:
  ```
  Files: 1,973
  Total size: 45.2 MB
  Average size: 23.4 KB
  Size range: 125 B - 125 KB
  Modified range: 2024-01-01 to 2024-12-15
  ```

### 9. Query Explanation (--explain flag)
- Show query evaluation plan
- Help users write efficient queries
- Example:
  ```
  detect --explain 'extension == ts and size > 10kb and text contains TODO'
  Query plan:
  1. Filter by extension: ts (fast, ~2000 files)
  2. Filter by size > 10kb (fast, ~500 files)
  3. Search contents for: TODO (slower, searching 500 files)
  Estimated files to examine: ~2000
  Estimated files to search: ~500
  ```

## Design Principles

### Iteration Order Considerations
- filesystem iteration order is non-deterministic
- Results may appear in different order between runs
- Use `--sort` when deterministic order is needed
- Avoid features that assume stable ordering (like pagination with --skip)

### What We're NOT Adding
- **--skip**: Iteration order not stable between runs
- **Pagination**: Would require maintaining state or stable ordering
- **Group by operations**: Use `awk`/`sort`/`uniq` pipeline
- **Watch mode**: Use `entr` or `watchman` with detect
- **Interactive mode**: Against Unix philosophy
- **Query aliases**: Use shell functions or aliases

## Implementation Priority

**Must Have (Phase 1):**
1. Relative paths by default
2. --limit (with docs about non-deterministic order)
3. Better error messages (see separate implementation)
4. --exclude patterns
5. --count (see separate implementation)

**Should Have (Phase 2):**
6. --sort (enables deterministic workflows)
7. --show-matches
8. --stats

**Nice to Have:**
9. --explain
10. Config file support

## Success Metrics
- Common queries are 50% shorter (due to defaults and --exclude)
- Output is immediately useful without path manipulation  
- Users can get deterministic results when needed (via --sort)

