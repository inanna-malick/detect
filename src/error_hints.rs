/// Provide helpful grammar guide when parsing fails
pub fn get_parse_error_hints() -> &'static str {
    "Quick grammar guide:
  • Selectors: name, type, size, contents, ext, path, modified
  • Operators: ==, !=, ~=, >, <, >=, <=, &&, ||, !
  • Set membership: ext in [js, ts, jsx]
  • Examples:
    - name ~= \".*\\.rs\"      # regex match
    - size > 1024            # size in bytes
    - type == \"file\"         # file or directory
    - contents ~= \"TODO\"     # search file contents
    - modified > \"-7.days\"   # modified in last week"
}