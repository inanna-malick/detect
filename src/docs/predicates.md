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
| `type`   | String  | file/directory/symlink | `type == file` |
| `depth`  | Numeric | Directory depth from root | `depth <= 3` |

**Size Units**: `kb`, `mb`, `gb`, `tb` (e.g., `45kb`, `1.5mb`)

## Time Selectors

| Selector    | Type     | Description | Example |
|-------------|----------|-------------|---------|
| `modified`  | Temporal | Last modification time | `modified > -7d` |
| `created`   | Temporal | File creation time | `created > 2024-01-01` |
| `accessed`  | Temporal | Last access time | `accessed < -1h` |

**Time Formats**:
- Relative: `-7d`, `-2h`, `-30m`, `-1w`
- Absolute: `2024-01-15`, `2024-01-15T10:30:00`
- Keywords: `now`, `today`, `yesterday`

## Content Selector

| Selector  | Type   | Description | Example |
|-----------|--------|-------------|---------|
| `content` | String | File text contents | `content contains TODO` |

## Selector Aliases

Some selectors have alternative names for convenience:

- `name` = `stem` = `basename` (filename without extension)
- `ext` = `extension` (file extension)  
- `content` = `text` = `contents` (file contents)

**Note**: Alias availability may vary by parser version.

## Type Details

### String Selectors
Work with: `name`, `ext`, `path`, `dir`, `type`, `content`

### Numeric Selectors  
Work with: `size`, `depth`

### Temporal Selectors
Work with: `modified`, `created`, `accessed`