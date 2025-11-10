# detect Operators Reference

All operators organized by selector type.

## String Operators

For: `name`, `ext`, `path`, `dir`, `content`

| Operator    | Description | Example |
|-------------|-------------|---------|
| `==`        | Exact match (case-sensitive) | `name == "README.md"` |
| `!=`        | Not equal | `ext != md` |
| `contains`  | Substring search (literal) | `content contains TODO` |
| `~=`        | Regex pattern matching | `name ~= "test.*\.rs$"` |
| `in [a,b,c]` | Match any item in set | `ext in [js,ts,jsx,tsx]` |

Regex uses Rust regex syntax. Set membership allows optional spaces: `ext in [rs, toml]`.

## Numeric Operators

For: `size`, `depth`

| Operator | Description | Example |
|----------|-------------|---------|
| `==`     | Exact value | `size == 1024` |
| `!=`     | Not equal | `depth != 0` |
| `>`      | Greater than | `size > 1mb` |
| `<`      | Less than | `depth < 5` |
| `>=`     | Greater or equal | `size >= 100kb` |
| `<=`     | Less or equal | `depth <= 2` |

Size units: `kb`, `mb`, `gb`, `tb` (lowercase only, e.g. `1kb`, `2.5mb`)

## Temporal Operators

For: `modified`, `created`, `accessed`

| Operator | Description | Example |
|----------|-------------|---------|
| `>`      | After (newer than) | `modified > -7d` |
| `<`      | Before (older than) | `created < 2024-01-01` |
| `>=`     | At or after | `modified >= -1w` |
| `<=`     | At or before | `accessed <= -1d` |
| `==`     | Exact time | `modified == 2024-01-15` |
| `!=`     | Not at time | `created != 2024-01-01` |

**Formats:** Relative `-7d`, `-2h`, `-30m`, `-1w` (units: `s`, `m`/`min`, `h`/`hr`, `d`/`day`, `w`/`week`, with plurals). Absolute `2024-01-15`, `2024-01-15T10:30:00`.

## Enum Operators

For: `type`

| Operator | Description | Example |
|----------|-------------|---------|
| `==`     | Exact match (validated at parse-time) | `type == file` |
| `!=`     | Not equal | `type != dir` |
| `in [a,b,c]` | Match any type in set | `type in [file,dir,symlink]` |

**Valid types (case-insensitive):** `file`, `dir`/`directory`, `symlink`/`link`, `socket`/`sock`, `fifo`/`pipe`, `block`/`blockdev`, `char`/`chardev`. Invalid values caught at parse-time with suggestions.

## Boolean Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `AND` / `&&` | Both conditions true | `ext == rs AND size > 1kb` |
| `OR` / `\|\|` | Either condition true | `file OR dir` |
| `NOT` / `!` | Negate condition | `NOT symlink` |
| `( )` | Group expressions | `(file OR dir) AND size > 1kb` |

**Precedence:** `NOT` > `AND` > `OR`. Use parentheses for clarity: `(a OR b) AND c`.

## Common Mistakes

**Units:** Lowercase only - `1mb` not `1MB`
**Regex quotes:** Quote patterns with spaces - `content ~= "class.*"` not `content ~= class.*`
**Wildcards:** Use `ext == rs` not `*.rs` (or `name ~= ".*\.rs$"` for regex)
