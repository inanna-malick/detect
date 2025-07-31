# detect

Find files by content AND metadata. Drop-in replacement for `find` that uses expressions instead of flags.

## Examples

- `contents ~= @(Injectable|Component|Directive)`
- `ext in [env, yml] && contents ~= (password|secret|api_key)`  
- `size > 10kb && contents ~= (async|await) && !contents contains TODO`
- `modified > "-7.days" && contents ~= (TODO|FIXME|HACK)`
- `filename ~= \.service\.ts$ && !contents contains test`
- `!dirpath contains node_modules && contents contains console.log`
- `dirpath contains src && ext == ts && contents ~= class.*extends`
- `contents ~= import.*from\s+['"]react && size > 50kb`
- `ext == rs && contents ~= unsafe && !dirpath contains /target/`
- `filename in [.env, .env.local] && contents ~= (AWS|STRIPE).*KEY`

Remember: any String operator works with any String selector - mix freely!

## find → detect

- `find . -name "*.js" -size +1M` → `detect 'ext == js && size > 1mb'`
- `find . -name "*test*" -mtime -7` → `detect 'filename contains test && modified > "-7.days"'`
- `find . -type f -exec grep -l TODO {} \;` → `detect 'type == file && contents contains TODO'`

## Reference

**Core Rule**: Any operator works with any selector of compatible type.

**Selectors**
- `String`: basename, filename, dirpath, fullpath, ext, contents
- `Number`: size (bytes/kb/mb/gb/tb)
- `Time`: modified, created, accessed  
- `Enum`: type (file/dir/symlink)

**Operators** 
- `String → Bool`: ==, !=, contains, ~=, in [...], >, <
- `Number → Bool`: ==, !=, >, <, >=, <=, in [...]
- `Time → Bool`: ==, !=, >, <, >=, <=, in [...]
- `Any → Bool`: in [...] works with all types

**Examples of orthogonality**
```bash
# ANY String selector with ANY String operator
contents ~= "TODO|FIXME"        # regex on contents ✓
filename in [Makefile, LICENSE] # set membership on names ✓
ext contains "s"                # substring on extension ✓

# Type mismatches won't work
size contains "100"             # Number ✗ String operator
```

**Boolean**
- && || ! () : and/or/not/group

**Time formats**
- Relative: `"-N.unit"` (seconds/minutes/hours/days/weeks/months)
- Absolute: `"YYYY-MM-DD"` 

**Syntax rules**
- Quotes required: whitespace, special chars, time expressions
- Regex: escape dots `\.`, anchors `^$` available
- Case-sensitive: all string comparisons
- Set items: comma-separated

**Performance**
- Queries run in 3 phases: name/metadata → then contents
- `ext == rs && contents contains unsafe` only scans contents of .rs files
- Boolean logic optimizes automatically regardless of order