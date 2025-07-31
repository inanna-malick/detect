# detect

Find files by content AND metadata. Drop-in replacement for `find` that uses expressions instead of flags.

## Examples

- `contents ~= @(Injectable|Component|Directive)`
- `ext in [env, yml] && contents ~= (password|secret|api_key)`  
- `size > 10kb && contents ~= (async|await) && !contents contains TODO`
- `modified > "-7.days" && contents ~= (TODO|FIXME|HACK)`
- `name ~= \.service\.ts$ && !contents contains test`
- `!path contains node_modules && contents contains console.log`
- `path contains src && ext == ts && contents ~= class.*extends`
- `contents ~= import.*from\s+['"]react && size > 50kb`
- `ext == rs && contents ~= unsafe && !path contains /target/`
- `name in [.env, .env.local] && contents ~= (AWS|STRIPE).*KEY`

## find → detect

- `find . -name "*.js" -size +1M` → `detect 'ext == js && size > 1mb'`
- `find . -name "*test*" -mtime -7` → `detect 'name contains test && modified > "-7.days"'`
- `find . -type f -exec grep -l TODO {} \;` → `detect 'type == file && contents contains TODO'`

## Reference

**Selectors**
- name/path/ext: strings
- size: bytes/kb/mb/gb/tb  
- type: file/dir/symlink
- contents: file text
- modified/created/accessed: relative/absolute time

**Operators**
- ==/!= : exact
- </<=/>/>= : compare
- contains : substring
- ~= : regex
- in [...] : set membership

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