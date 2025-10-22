# detect Predicates Reference

All available selectors and their data types.

## File Identity Selectors

| Selector | Type   | Description | Example |
|----------|--------|-------------|---------|
| `name`   | String | Full filename with extension | `name == "README.md"` |
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
- Keywords: `now`, `today`, `yesterday`

## Content Selector

| Selector  | Type   | Description | Example |
|-----------|--------|-------------|---------|
| `content` | String | File text contents | `content contains TODO` |

## Selector Aliases

Some selectors have alternative names for convenience:

- `name` = `filename` (full filename with extension)
- `stem` = `basename` (filename without extension)
- `ext` = `extension` (file extension)
- `dir` = `parent` = `directory` (parent directory)
- `size` = `filesize` = `bytes` (file size)
- `type` = `filetype` (file type)
- `modified` = `mtime` (modification time)
- `created` = `ctime` (creation time)
- `accessed` = `atime` (access time)
- `content` = `text` = `contents` (file contents)

## Type Details

### String Selectors
Work with: `name`, `ext`, `path`, `dir`, `content`

Operators: `==`, `!=`, `contains`, `~=` (regex), `in [...]`

### Numeric Selectors
Work with: `size`, `depth`

Operators: `==`, `!=`, `>`, `<`, `>=`, `<=`

### Temporal Selectors
Work with: `modified`, `created`, `accessed`

Operators: `==`, `!=`, `>` (after), `<` (before)

### Enum Selectors
Work with: `type`

Operators: `==`, `!=`, `in [...]`

**Note**: Enum values are validated at parse-time. Invalid values produce errors listing all valid options.