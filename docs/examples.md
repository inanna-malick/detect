# detect Examples

**Quick tips:**
- Start with cheap filters (`ext`, `size`, `type`) before expensive ones (`content`, structured)
- Quote expressions with spaces/special chars: `'ext == rs AND content ~= "async "'`
- Use `-i` to include gitignored files

## Progressive Examples

Each line adds complexity - shows how to combine features:

```bash
# Start simple
detect 'ext == rs'                                           # selector + operator

# Combine with AND
detect 'ext in [rs,toml] AND size > 1mb'                    # set membership, numeric

# Add temporal predicates
detect 'ext == rs AND size > 1mb AND modified > -7d'        # relative time

# Content matching with regex
detect 'ext == ts AND content ~= "class.*Service"'          # regex operator

# Boolean logic: grouping, NOT
detect '(ext == rs OR ext == toml) AND NOT path ~= test'    # precedence, path filter

# Structured data + file metadata
detect 'yaml:.server.port > 8000 AND size < 100kb'          # structured selector
```

## Structured Data Patterns

Navigate YAML/JSON/TOML with path syntax:

```bash
# Nested field access: .field.field
yaml:.server.port == 8080

# Array indexing + field access: [0].field
json:.items[0].name == "first"

# Wildcard - matches if ANY element matches: [*]
yaml:.features[*].enabled == true

# Recursive descent - finds field at any depth: ..field
toml:..database contains prod

# Combine with file predicates
yaml:.replicas > 3 AND size < 100kb AND NOT path ~= test

# Multi-format queries with OR
json:.version ~= "^1\\." OR toml:.package.version ~= "^1\\."
```

## Common Patterns

Real-world multi-feature queries:

```bash
# Large recent files with TODOs, excluding tests
detect 'size > 10kb AND modified > -7d AND content contains TODO AND NOT path ~= test'

# Security: env files with secrets outside node_modules
detect 'name ~= "^\.env" AND NOT path ~= node_modules AND content ~= "(password|secret|key)"'

# Recent config changes
detect 'ext in [json,yaml,toml] AND modified > -3d'

# Kubernetes manifests with high replicas
detect 'yaml:.kind == Deployment AND yaml:.spec.replicas > 3'

# Find TypeScript async functions in source directories
detect 'path ~= "^\./(src|lib)/" AND ext == ts AND content ~= "async\s+function"'
```

## Migration from find/grep

```bash
# find . -name "*.js" -size +1M
detect 'ext == js AND size > 1mb'

# find . -type f -exec grep -l "TODO" {} \;
detect 'type == file AND content contains TODO'

# grep -r "class.*Service" --include="*.ts" .
detect 'ext == ts AND content ~= "class.*Service"'
```
