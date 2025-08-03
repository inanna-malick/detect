# detect

Find files by content AND metadata. Drop-in replacement for `find` that uses expressions instead of flags.

## Examples

- `contents ~= @(Injectable|Component|Directive)`
- `path.extension in [env, yml] && contents ~= (password|secret|api_key)`  
- `size > 10kb && contents ~= (async|await) && !contents contains TODO`
- `modified > -7.days && contents ~= (TODO|FIXME|HACK)`
- `path.name ~= \.service\.ts$ && !contents contains test`
- `!path.parent contains node_modules && contents contains console.log`
- `path.parent contains src && path.extension == ts && contents ~= class.*extends`
- `contents ~= import.*from\s+['"]react && size > 50kb`
- `path.extension == rs && contents ~= unsafe && !path.parent contains /target/`
- `path.name in [.env, .env.local] && contents ~= (AWS|STRIPE).*KEY`
- `(contents contains TODO || contents contains FIXME) && modified > -7.days`
- `(path.extension == js || path.extension == ts) && (size > 50kb || contents contains export)`
- `name == README AND size > 1kb OR NOT type == dir`

Remember: any String operator works with any String selector - mix freely!

## find → detect

- `find . -name "*.js" -size +1M` → `detect 'path.extension == js && size > 1mb'`
- `find . -name "*test*" -mtime -7` → `detect 'path.name contains test && modified > -7.days'`
- `find . -type f -exec grep -l TODO {} \;` → `detect 'type == file && contents contains TODO'`

## Reference

**Core Rule**: Any operator works with any selector of compatible type.

**Selectors**
- `String`: path.stem, path.name, path.parent, path.full, path.extension, contents
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
path.name in [Makefile, LICENSE] # set membership on names ✓
path.extension contains s                # substring on extension ✓

# Type mismatches won't work
size contains 100             # Number ✗ String operator
```

**Boolean**
- && || ! () : and/or/not/group (symbols)
- and/or/not : case-insensitive word forms (AND/OR/NOT also work)
- `(contents contains TODO || contents contains FIXME) && (size > 1000 || modified > -1.day)`
- `name == foo AND size > 100 OR NOT type == dir`

**Time formats**
- Relative: `-N.unit` (seconds/minutes/hours/days/weeks/months)
- Absolute: `YYYY-MM-DD` 

**Syntax rules**
- Quotes required: whitespace, special chars
- Regex: escape dots `\.`, anchors `^$` available
- Case-sensitive: all string comparisons
- Set items: comma-separated

**Performance**
- Queries run in 3 phases: name/metadata → then contents
- `path.extension == rs && contents contains unsafe` only scans contents of .rs files
- Boolean logic optimizes automatically regardless of order