# detect Operators Reference

All available operators organized by data type.

## String Operators

For selectors: `name`, `ext`, `path`, `dir`, `content`

| Operator    | Description | Example |
|-------------|-------------|---------|
| `==`        | Exact match | `name == "README.md"` |
| `!=`        | Not equal | `ext != md` |
| `contains`  | Substring search (literal) | `content contains TODO` |
| `~=`        | Regex pattern matching | `name ~= "test.*\.rs$"` |
| `in [a,b,c]` | Match any item in set | `ext in [js,ts,jsx,tsx]` |

### String Operator Details

**Exact Matching (`==`, `!=`)**:
- Case-sensitive comparison
- Must match entire value
- Example: `name == "test.rs"` matches exactly "test.rs"

**Substring Search (`contains`)**:
- Literal text search (no regex)
- Case-sensitive
- Example: `content contains "class Foo"` finds exact text

**Regex Matching (`~=`)**:
- Full regex support
- Case-sensitive by default
- Example: `content ~= "class\\s+\\w+"` matches class definitions
- Escape special chars: `\.`, `\[`, `\(`

**Set Membership (`in`)**:
- Match any value in comma-separated list
- Example: `ext in [rs,toml,md]` or `ext in [rs, toml, md]` (spaces optional)
- Case-sensitive matching

## Numeric Operators

For selectors: `size`, `depth`

| Operator | Description | Example |
|----------|-------------|---------|
| `==`     | Exact value | `size == 1024` |
| `!=`     | Not equal | `depth != 0` |
| `>`      | Greater than | `size > 1mb` |
| `<`      | Less than | `depth < 5` |
| `>=`     | Greater or equal | `size >= 100kb` |
| `<=`     | Less or equal | `depth <= 2` |

### Size Values
- Raw bytes: `1024`, `2048`
- With units: `1kb`, `2.5mb`, `1gb`
- Units: `kb`, `mb`, `gb`, `tb`

## Temporal Operators

For selectors: `modified`, `created`, `accessed`

| Operator | Description | Example |
|----------|-------------|---------|
| `>`      | After (newer than) | `modified > -7d` |
| `<`      | Before (older than) | `created < 2024-01-01` |
| `>=`     | At or after | `modified >= -1w` |
| `<=`     | At or before | `accessed <= yesterday` |
| `==`     | Exact time | `modified == today` |
| `!=`     | Not at time | `created != 2024-01-01` |

### Time Value Formats

**Relative Times** (from now):
- `-7d` = 7 days ago (also: `-7days`, `-7day`)
- `-2h` = 2 hours ago (also: `-2hours`, `-2hrs`, `-2hr`)
- `-30m` = 30 minutes ago (also: `-30minutes`, `-30mins`, `-30min`)
- `-1w` = 1 week ago (also: `-1weeks`, `-1week`)
- Supported units: `s`/`sec`/`second`, `m`/`min`/`minute`, `h`/`hr`/`hour`, `d`/`day`, `w`/`week` (+ plurals)

**Absolute Dates**:
- `2024-01-15` = specific date
- `2024-01-15T10:30:00` = with time

**Keywords**:
- `now` = current time
- `today` = start of today
- `yesterday` = start of yesterday

## Enum Operators

For selectors: `type`

| Operator | Description | Example |
|----------|-------------|---------|
| `==`     | Exact match (validated at parse-time) | `type == file` |
| `!=`     | Not equal | `type != dir` |
| `in [a,b,c]` | Match any type in set | `type in [file,dir,symlink]` |

**Valid Values** (case-insensitive):
- `file` - Regular file
- `dir` / `directory` - Directory
- `symlink` / `link` - Symbolic link
- `socket` / `sock` - Unix socket
- `fifo` / `pipe` - Named pipe
- `block` / `blockdev` - Block device
- `char` / `chardev` - Character device

**Parse-time Validation**: Invalid enum values produce immediate errors with suggestions.

Example error for `type == dirq`:
```
× Expected one of: file, dir, directory, symlink, link, socket, sock, fifo,
│ pipe, block, blockdev, char, chardev value, found: dirq
```

## Boolean Operators

Combine expressions with logical operators:

| Operator | Description | Example |
|----------|-------------|---------|
| `AND` / `&&` | Both conditions true | `ext == rs AND size > 1kb` |
| `OR` / `\|\|` | Either condition true | `*.md OR content contains TODO` |
| `NOT` / `!` | Negate condition | `NOT *.test.*` |
| `( )` | Group expressions | `(*.rs OR *.toml) AND size > 1kb` |

### Precedence (highest to lowest):
1. `NOT` / `!`
2. `AND` / `&&` 
3. `OR` / `||`

Use parentheses for clarity: `(a OR b) AND c`

## Common Mistakes

❌ **Wrong**: `content ~= class.*` (unquoted regex with spaces)
✅ **Right**: `content ~= "class.*"` or `content ~= class.*` (no spaces)

❌ **Wrong**: `size > 1MB` (wrong unit case)
✅ **Right**: `size > 1mb` (units are lowercase)

❌ **Wrong**: `name contains *.rs` (mixing glob with predicate)
✅ **Right**: `*.rs` OR `name contains .rs`

**Note**: Spaces in sets are fine - both `ext in [js,ts]` and `ext in [js, ts]` work identically.