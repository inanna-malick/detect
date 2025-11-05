# detect Predicates Reference

All available selectors and their data types.

## File Identity Selectors

| Selector | Type   | Description | Example |
|----------|--------|-------------|---------|
| `name`   | String | Full filename with extension | `name == "README.md"` |
| `basename` | String | Filename without extension | `basename == README` |
| `ext`    | String | File extension without dot | `ext == rs` |
| `path`   | String | Full absolute path | `path ~= "/src/"` |
| `dir`    | String | Parent directory path | `dir == "/usr/bin"` |

## File Property Selectors

| Selector | Type    | Description | Example |
|----------|---------|-------------|---------|
| `size`   | Numeric | File size in bytes | `size > 1mb` |
| `type`   | Enum    | File type (validated at parse-time) | `type == file` |
| `depth`  | Numeric | Directory depth from root | `depth <= 3` |

**Size Units**: `kb`, `mb`, `gb`, `tb` (e.g., `45kb`, `1.5mb`)

**Valid File Types** (case-insensitive):
- `file` - Regular file
- `dir` / `directory` - Directory
- `symlink` / `link` - Symbolic link
- `socket` / `sock` - Unix socket
- `fifo` / `pipe` - Named pipe (FIFO)
- `block` / `blockdev` - Block device
- `char` / `chardev` - Character device

## Time Selectors

| Selector    | Type     | Description | Example |
|-------------|----------|-------------|---------|
| `modified`  | Temporal | Last modification time | `modified > -7d` |
| `created`   | Temporal | File creation time | `created > 2024-01-01` |
| `accessed`  | Temporal | Last access time | `accessed < -1h` |

**Time Formats**:
- Relative: `-7d` / `-7days`, `-2h` / `-2hours`, `-30m` / `-30minutes`, `-1w` / `-1week`
  - Units: `s`/`sec`/`second`, `m`/`min`/`minute`, `h`/`hr`/`hour`, `d`/`day`, `w`/`week` (+ plurals)
- Absolute: `2024-01-15`, `2024-01-15T10:30:00`

## Content Selector

| Selector  | Type   | Description | Example |
|-----------|--------|-------------|---------|
| `content` | String | File text contents | `content contains TODO` |

## Structured Data Selectors

Query YAML, JSON, and TOML file contents by navigating their structure:

| Selector | Type | Description | Example |
|----------|------|-------------|---------|
| `yaml:.path` | Structured | YAML navigation | `yaml:.server.port == 8080` |
| `json:.path` | Structured | JSON navigation | `json:.name == "test"` |
| `toml:.path` | Structured | TOML navigation | `toml:.dependencies.serde` |

**Navigation Syntax:**
- `.field` - Access object field
- `.nested.field` - Access nested fields
- `[0]` - Access array element by index
- `[*]` - Wildcard - all array elements
- `..field` - Recursive descent - all fields at any depth

**Operators:** `==`, `!=`, `>`, `<`, `>=`, `<=`, `contains`, `~=`

**Type Coercion:** Falls back to strings on type mismatch
- `yaml:.port == 8080` matches both integer 8080 and string "8080"
- `json:.version == "1.0"` matches both string "1.0" and number 1.0
- Applies to all comparison and string matching operators

**Examples:**
```bash
# Simple field access
yaml:.server.port == 8080

# Nested fields
json:.dependencies.react == "18.0.0"

# Array indexing
yaml:.features[0].name == "auth"

# Wildcards - all array elements
yaml:.features[*].enabled == true

# Recursive descent
yaml:..password contains "prod"

# Numeric comparisons
json:.spec.replicas > 3

# Type coercion
yaml:.port == "8080"  # matches port: 8080 or port: "8080"
```

**Limitations:**
- Files > 10MB skipped (configurable: `--max-structured-size`)
- Non-UTF8 files skip structured evaluation
- Invalid YAML/JSON/TOML returns false (no error)
- Multi-document YAML: matches if ANY document matches

## Selector Aliases

Alternative names for convenience:

- `name` = `filename`
- `stem` = `basename`
- `ext` = `extension`
- `dir` = `parent` = `directory`
- `size` = `filesize` = `bytes`
- `type` = `filetype`
- `modified` = `mtime`
- `created` = `ctime`
- `accessed` = `atime`
- `content` = `text` = `contents`