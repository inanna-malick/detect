# detect Operators Reference

All available operators organized by data type.

## String Operators

For selectors: `name`, `ext`, `path`, `dir`, `type`, `content`

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
- Example: `ext in [rs,toml,md]`
- No spaces around commas: `[a,b,c]` not `[a, b, c]`

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
- `-7d` = 7 days ago
- `-2h` = 2 hours ago  
- `-30m` = 30 minutes ago
- `-1w` = 1 week ago

**Absolute Dates**:
- `2024-01-15` = specific date
- `2024-01-15T10:30:00` = with time

**Keywords**:
- `now` = current time
- `today` = start of today
- `yesterday` = start of yesterday

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

❌ **Wrong**: `ext in [js, ts]` (spaces in set)  
✅ **Right**: `ext in [js,ts]`

❌ **Wrong**: `content ~= class.*` (unquoted regex)  
✅ **Right**: `content ~= "class.*"`

❌ **Wrong**: `size > 1MB` (wrong unit)  
✅ **Right**: `size > 1mb`

❌ **Wrong**: `name contains *.rs` (mixing operators)  
✅ **Right**: `*.rs` OR `name contains .rs`