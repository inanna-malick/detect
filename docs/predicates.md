# detect Predicates Reference

All selectors and their types. Aliases shown as `primary` / `alias`.

## File Identity

| Selector | Type   | Description | Example |
|----------|--------|-------------|---------|
| `name` / `filename` | String | Full filename with extension | `name == "README.md"` |
| `basename` / `stem` | String | Filename without extension | `basename == README` |
| `ext` / `extension` | String | File extension without dot | `ext == rs` |
| `path` | String | Full absolute path | `path ~= "/src/"` |
| `dir` / `parent` / `directory` | String | Parent directory path | `dir == "/usr/bin"` |

## File Properties

| Selector | Type    | Description | Example |
|----------|---------|-------------|---------|
| `size` / `filesize` / `bytes` | Numeric | File size in bytes | `size > 1mb` |
| `type` / `filetype` | Enum    | File type (validated at parse-time) | `type == file` |
| `depth` | Numeric | Directory depth from root | `depth <= 3` |

**Size units:** `kb`, `mb`, `gb`, `tb` (e.g. `45kb`, `1.5mb`)

**Valid types (case-insensitive):** `file`, `dir`/`directory`, `symlink`/`link`, `socket`/`sock`, `fifo`/`pipe`, `block`/`blockdev`, `char`/`chardev`

## Timestamps

| Selector | Type | Description | Example |
|----------|------|-------------|---------|
| `modified` / `mtime` | Temporal | Last modification time | `modified > -7d` |
| `created` / `ctime` | Temporal | File creation time | `created > 2024-01-01` |
| `accessed` / `atime` | Temporal | Last access time | `accessed < -1h` |

**Formats:** Relative `-7d`/`-7days`, `-2h`/`-2hours` (units: `s`, `m`/`min`, `h`/`hr`, `d`/`day`, `w`/`week` + plurals). Absolute `2024-01-15`, `2024-01-15T10:30:00`.

## Content

| Selector | Type | Description | Example |
|----------|------|-------------|---------|
| `content` / `text` / `contents` | String | File text contents | `content contains TODO` |

## Structured Data

Query YAML, JSON, TOML by navigating structure:

| Selector | Description | Example |
|----------|-------------|---------|
| `yaml:.path` | YAML navigation | `yaml:.server.port == 8080` |
| `json:.path` | JSON navigation | `json:.items[0].name == "test"` |
| `toml:.path` | TOML navigation | `toml:.package.edition == "2021"` |

**Navigation syntax:**

| Pattern | Meaning | Example |
|---------|---------|---------|
| `.field` | Object field access | `yaml:.server` |
| `.nested.field` | Nested fields | `json:.meta.author` |
| `[0]` | Array index | `yaml:.items[0]` |
| `[*]` | Wildcard - any element | `yaml:.features[*].enabled` |
| `..field` | Recursive - any depth | `toml:..password` |

**Operators:** `==`, `!=`, `>`, `<`, `>=`, `<=`, `contains`, `~=` (same as other selectors)

**Type coercion:** Numbers/booleans convert to strings - `yaml:.port == 8080` matches both `8080` and `"8080"`

**Existence check:** Use selector alone without operator - `yaml:.server` checks if field exists

**Limitations:**
- Files > 10MB skipped (configurable: `--max-structured-size`)
- Non-UTF8 files skip structured evaluation
- Invalid YAML/JSON/TOML returns false (no error)
- Multi-document YAML: matches if ANY document matches
