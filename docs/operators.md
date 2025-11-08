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

All string matching is case-sensitive. Regex uses Rust regex syntax. Set membership allows optional spaces: `ext in [rs, toml, md]`.

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

Size values support units: `kb`, `mb`, `gb`, `tb` (e.g., `1kb`, `2.5mb`).

## Temporal Operators

For selectors: `modified`, `created`, `accessed`

| Operator | Description | Example |
|----------|-------------|---------|
| `>`      | After (newer than) | `modified > -7d` |
| `<`      | Before (older than) | `created < 2024-01-01` |
| `>=`     | At or after | `modified >= -1w` |
| `<=`     | At or before | `accessed <= -1d` |
| `==`     | Exact time | `modified == 2024-01-15` |
| `!=`     | Not at time | `created != 2024-01-01` |

Time formats: `-7d`, `-2h`, `-30m`, `-1w` (relative); `2024-01-15`, `2024-01-15T10:30:00` (absolute). Relative units support plurals: `-7days`, `-2hours`.

## Enum Operators

For selectors: `type`

| Operator | Description | Example |
|----------|-------------|---------|
| `==`     | Exact match (validated at parse-time) | `type == file` |
| `!=`     | Not equal | `type != dir` |
| `in [a,b,c]` | Match any type in set | `type in [file,dir,symlink]` |

Valid types (case-insensitive): `file`, `dir`/`directory`, `symlink`/`link`, `socket`/`sock`, `fifo`/`pipe`, `block`/`blockdev`, `char`/`chardev`. Invalid values are caught at parse-time.

## Boolean Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `AND` / `&&` | Both conditions true | `ext == rs AND size > 1kb` |
| `OR` / `\|\|` | Either condition true | `file OR dir` |
| `NOT` / `!` | Negate condition | `NOT symlink` |
| `( )` | Group expressions | `(file OR dir) AND size > 1kb` |

Precedence: `NOT` > `AND` > `OR`. Use parentheses when combining: `(a OR b) AND c`.

## Common Mistakes

❌ **Wrong**: `content ~= class.*` (unquoted regex with spaces)
✅ **Right**: `content ~= "class.*"` or `content ~= class.*` (no spaces)

❌ **Wrong**: `size > 1MB` (wrong unit case)
✅ **Right**: `size > 1mb` (units are lowercase)

❌ **Wrong**: `*.rs` (wildcards not supported)
✅ **Right**: `ext == rs` or use regex `name ~= ".*\.rs$"`

**Note**: Spaces in sets are fine - both `ext in [js,ts]` and `ext in [js, ts]` work identically.